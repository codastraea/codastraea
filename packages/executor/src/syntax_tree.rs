use std::{collections::HashMap, sync::Arc, thread::sleep, time::Duration};

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{alpha1, alphanumeric1, line_ending, multispace0, space0, space1},
    combinator::{all_consuming, eof, map, opt, recognize},
    error::{context, ErrorKind},
    multi::{many0, many_till, separated_list0, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    Finish, IResult, Parser as _,
};
use nom_greedyerror::{convert_error, GreedyError};
use nom_locate::LocatedSpan;
use scopeguard::defer;
use thiserror::Error;
use tokio::sync::watch;

use crate::{
    library::{FunctionId, Library},
    run::{NestedBlock, StackFrame, ThreadRunState},
};

pub fn parse(input: &str) -> Result<Module, ParseError> {
    match all_consuming(Module::parse())
        .parse(Span::new(input))
        .finish()
    {
        Ok((_, module)) => Ok(module),
        Err(e) => Err(ParseError(convert_error(input, e))),
    }
}

type Span<'a> = LocatedSpan<&'a str>;

type ParseResult<'a, T> = IResult<Span<'a>, T, GreedyError<Span<'a>, ErrorKind>>;

trait Parser<'a, O>: nom::Parser<Span<'a>, O, GreedyError<Span<'a>, ErrorKind>> {}

impl<'a, O, P: nom::Parser<Span<'a>, O, GreedyError<Span<'a>, ErrorKind>>> Parser<'a, O> for P {}

#[derive(PartialEq, Eq, Debug)]
pub struct Module {
    functions: Vec<Function>,
}

impl Module {
    fn parse<'a>() -> impl Parser<'a, Self> {
        context(
            "module",
            many_till(multiline_ws(Function::parse(None)), eof),
        )
        .map(|(functions, _)| Module { functions })
    }

    pub fn functions(&self) -> &[Function] {
        &self.functions
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Function {
    name: String,
    span: SrcSpan,
    body: Body<String>,
}

impl Function {
    pub fn name(&self) -> &str {
        &self.name
    }

    fn parse(current_indent: Option<&str>) -> impl Parser<Self> {
        context(
            "function",
            tuple((
                def,
                space1,
                identifier(),
                ws(tag("()")),
                colon,
                Body::parse(current_indent),
            )),
        )
        .map(|(_def, _, name, _params, _colon, body)| Function {
            name: name.fragment().to_string(),
            span: SrcSpan::from_span(&name),
            body,
        })
    }

    pub fn translate_ids(&self, id_map: &IdMap) -> LinkedFunction {
        LinkedFunction::local(
            &self.name,
            self.span,
            self.body
                .iter()
                .map(|statement| statement.translate_ids(id_map)),
        )
    }

    pub fn unresolved_symbols<'a>(
        &'a self,
        id_map: &'a HashMap<String, FunctionId>,
    ) -> impl Iterator<Item = String> + 'a {
        self.body
            .iter()
            .flat_map(|stmt| stmt.unresolved_symbols(id_map))
    }
}

// TODO: Move this
#[derive(Debug)]
pub struct LinkedFunction {
    name: String,
    span: Option<SrcSpan>,
    body: LinkedBody,
}

impl LinkedFunction {
    pub fn local(
        name: &str,
        span: SrcSpan,
        body: impl IntoIterator<Item = Statement<FunctionId>>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            span: Some(span),
            body: LinkedBody::Local(Arc::new(Body::new(body))),
        }
    }

    pub fn python(name: String) -> Self {
        Self {
            name,
            span: None,
            body: LinkedBody::Python,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn span(&self) -> Option<SrcSpan> {
        self.span
    }

    pub fn body(&self) -> &LinkedBody {
        &self.body
    }

    pub fn run(&self, args: &[Value], lib: &Library, call_states: &watch::Sender<ThreadRunState>) {
        println!("Running function '{}'", self.name());
        sleep(Duration::from_secs(1));

        match &self.body {
            LinkedBody::Local(local) => local.run(lib, call_states),
            LinkedBody::Python => {
                // TODO
                println!("{}({:?})", self.name(), args)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum LinkedBody {
    Local(Arc<Body<FunctionId>>),
    Python,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Body<T>(Vec<Statement<T>>);

impl<T> Body<T> {
    pub fn new(stmts: impl IntoIterator<Item = Statement<T>>) -> Self {
        Self(stmts.into_iter().collect())
    }

    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Statement<T>> {
        self.0.iter()
    }
}

impl Body<String> {
    fn parse(current_indent: Option<&str>) -> impl Parser<Self> {
        alt((Self::parse_inline(), Self::parse_block(current_indent))).map(Self::new)
    }

    fn parse_inline<'a>() -> impl Parser<'a, Vec<Statement<String>>> {
        context("inline body", Statement::parse(None)).map(|statement| vec![statement])
    }

    fn parse_block(current_indent: Option<&str>) -> impl Parser<Vec<Statement<String>>> {
        move |input| {
            let (input, prefix) = preceded(
                pair(eol(), blank_lines()),
                recognize(pair(discard_indent(current_indent), space1)),
            )
            .map(|prefix: Span| Some(*(prefix.fragment())))
            .parse(input)?;

            // TODO: Is error reporting friendly enough?
            separated_list1(discard_newline_indent(prefix), Statement::parse(prefix))(input)
        }
    }

    fn translate_ids(&self, id_map: &IdMap) -> Body<FunctionId> {
        Body(self.iter().map(|stmt| stmt.translate_ids(id_map)).collect())
    }

    fn unresolved_symbols(&self, id_map: &HashMap<String, FunctionId>) -> Vec<String> {
        self.iter()
            .flat_map(|stmt| stmt.unresolved_symbols(id_map))
            .collect()
    }
}

impl Body<FunctionId> {
    pub fn run(&self, lib: &Library, call_states: &watch::Sender<ThreadRunState>) {
        for (index, stmt) in self.iter().enumerate() {
            call_states.send_modify(|t| t.push(StackFrame::Statement(index)));
            defer! {call_states.send_modify(|t| t.pop_success());}
            stmt.run(lib, call_states);
        }
    }
}

pub type IdMap = HashMap<String, FunctionId>;

fn blank_lines<'a>() -> impl Parser<'a, ()> {
    discard(many0(pair(space0, eol())))
}

fn eol<'a>() -> impl Parser<'a, ()> {
    discard(tuple((
        space0,
        opt(pair(tag("#"), is_not("\r\n"))),
        line_ending,
    )))
}

#[derive(Eq, PartialEq, Debug)]
pub enum Statement<FnId> {
    Pass,
    Expression(Expression<FnId>),
    If {
        if_span: SrcSpan,
        condition: Arc<Expression<FnId>>,
        then_block: Arc<Body<FnId>>,
        else_block: Option<ElseClause<FnId>>,
    },
}

impl Statement<String> {
    fn parse(prefix: Option<&str>) -> impl Parser<Self> {
        context(
            "statement",
            alt((
                pass.map(|_| Statement::Pass),
                move |input| Self::parse_if(prefix, input),
                map(Expression::parse(), Statement::Expression),
            )),
        )
    }

    fn parse_if<'a>(current_indent: Option<&'a str>, input: Span<'a>) -> ParseResult<'a, Self> {
        // TODO: elif
        let (input, (if_keyword, condition, _colon, then_block, else_block)) = context(
            "if",
            tuple((
                r#if,
                ws(Expression::parse()),
                ws(colon),
                Body::parse(current_indent),
                opt(ElseClause::parse(current_indent)),
            )),
        )(input)?;

        let statement = Self::If {
            if_span: SrcSpan::from_span(&if_keyword),
            condition: Arc::new(condition),
            then_block: Arc::new(then_block),
            else_block,
        };

        Ok((input, statement))
    }

    fn translate_ids(&self, id_map: &IdMap) -> Statement<FunctionId> {
        match self {
            Self::Pass => Statement::Pass,
            Self::Expression(expression) => Statement::Expression(expression.translate_ids(id_map)),
            Self::If {
                if_span,
                condition,
                then_block,
                else_block,
            } => Statement::If {
                if_span: *if_span,
                condition: Arc::new(condition.translate_ids(id_map)),
                then_block: Arc::new(then_block.translate_ids(id_map)),
                else_block: else_block.as_ref().map(|e| e.translate_ids(id_map)),
            },
        }
    }

    fn unresolved_symbols(&self, id_map: &HashMap<String, FunctionId>) -> Vec<String> {
        match self {
            Self::Pass => vec![],
            Self::Expression(expr) => expr.unresolved_symbols(id_map),
            Self::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let mut unresolved = condition.unresolved_symbols(id_map);

                unresolved.extend(then_block.unresolved_symbols(id_map));

                if let Some(else_block) = else_block {
                    unresolved.extend(else_block.unresolved_symbols(id_map));
                }

                unresolved
            }
        }
    }
}

impl Statement<FunctionId> {
    pub fn run(&self, lib: &Library, call_states: &watch::Sender<ThreadRunState>) {
        match self {
            Self::Pass => (),
            Self::Expression(expr) => {
                expr.run(lib, call_states);
            }
            Self::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                let mut drop_through = true;

                // TODO: Tidy this
                call_states
                    .send_modify(|t| t.push(StackFrame::NestedBlock(0, NestedBlock::Predicate)));
                let truthy = condition.run(lib, call_states).truthy();
                call_states.send_modify(|t| t.pop_predicate_success(truthy));

                if truthy {
                    drop_through = false;
                    call_states
                        .send_modify(|t| t.push(StackFrame::NestedBlock(0, NestedBlock::Body)));
                    defer! {call_states.send_modify(|t| t.pop_success());}
                    then_block.run(lib, call_states);
                }

                if let Some(else_block) = else_block {
                    // TODO: Functions to `send_modify` `push` and `pop` stack
                    let block_index = 1;
                    call_states.send_modify(|t| {
                        t.push(StackFrame::NestedBlock(block_index, NestedBlock::Predicate))
                    });
                    call_states.send_modify(|t| t.pop_predicate_success(drop_through));

                    if drop_through {
                        call_states.send_modify(|t| {
                            t.push(StackFrame::NestedBlock(block_index, NestedBlock::Body))
                        });
                        defer! {call_states.send_modify(|t| t.pop_success());}

                        else_block.run(lib, call_states)
                    }
                }
            }
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct ElseClause<FnId> {
    else_span: SrcSpan,
    body: Arc<Body<FnId>>,
}

impl ElseClause<String> {
    fn parse(current_indent: Option<&str>) -> impl Parser<Self> {
        context(
            "else",
            tuple((
                discard_newline_indent(current_indent),
                r#else,
                ws(colon),
                Body::parse(current_indent),
            )),
        )
        .map(|(_indent, else_keyword, _colon, body)| Self {
            else_span: SrcSpan::from_span(&else_keyword),
            body: Arc::new(body),
        })
    }

    fn translate_ids(&self, id_map: &IdMap) -> ElseClause<FunctionId> {
        ElseClause {
            else_span: self.else_span,
            body: Arc::new(self.body.translate_ids(id_map)),
        }
    }

    fn unresolved_symbols(&self, id_map: &HashMap<String, FunctionId>) -> Vec<String> {
        self.body.unresolved_symbols(id_map)
    }
}

impl ElseClause<FunctionId> {
    pub fn run(&self, lib: &Library, call_states: &watch::Sender<ThreadRunState>) {
        self.body.run(lib, call_states)
    }

    pub fn span(&self) -> SrcSpan {
        self.else_span
    }

    pub fn body(&self) -> &Arc<Body<FunctionId>> {
        &self.body
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Expression<FnId> {
    Literal(Literal),
    Variable {
        name: String,
    },
    Call {
        span: SrcSpan,
        name: FnId,
        args: Vec<Expression<FnId>>,
    },
}

impl Expression<FunctionId> {
    pub fn run(&self, lib: &Library, call_states: &watch::Sender<ThreadRunState>) -> Value {
        match self {
            Expression::Variable { name } => todo!("Variable {name}"),
            Expression::Call { name, args, .. } => run_call(*name, args, lib, call_states),
            Expression::Literal(literal) => literal.run(),
        }
    }
}

pub(crate) fn run_call(
    name: FunctionId,
    args: &[Expression<FunctionId>],
    lib: &Library,
    call_states: &watch::Sender<ThreadRunState>,
) -> Value {
    let args: Vec<_> = args
        .iter()
        .enumerate()
        .map(|(index, arg)| {
            call_states.send_modify(|t| t.push(StackFrame::Argument(index)));
            defer! {call_states.send_modify(|t| t.pop_success());}
            arg.run(lib, call_states)
        })
        .collect();
    call_states.send_modify(|t| t.push(StackFrame::Call(name)));
    defer! {call_states.send_modify(|t| t.pop_success());}
    lib.lookup(name).run(&args, lib, call_states);
    Value::None
}

impl Expression<String> {
    fn parse<'a>() -> impl Parser<'a, Self> {
        alt((
            Self::literal(),
            Self::call(),
            Self::variable(),
            Self::parenthasized(),
        ))
    }

    fn literal<'a>() -> impl Parser<'a, Self> {
        Literal::parse().map(Self::Literal)
    }

    fn variable<'a>() -> impl Parser<'a, Self> {
        identifier().map(|name| Self::Variable {
            name: name.fragment().to_string(),
        })
    }

    fn call<'a>() -> impl Parser<'a, Self> {
        move |input| {
            context(
                "call",
                separated_pair(
                    identifier(),
                    space0,
                    delimited(
                        tag("("),
                        separated_list0(tag(","), multiline_ws(Self::parse())),
                        tag(")"),
                    ),
                ),
            )
            .map(|(name, args)| Self::Call {
                name: name.fragment().to_string(),
                args,
                span: SrcSpan::from_span(&name),
            })
            .parse(input)
        }
    }

    fn parenthasized<'a>() -> impl Parser<'a, Self> {
        move |input| {
            context(
                "parenthesized",
                delimited(tag("("), multiline_ws(Expression::parse()), tag(")")),
            )
            .parse(input)
        }
    }

    fn translate_ids(&self, id_map: &IdMap) -> Expression<FunctionId> {
        match self {
            Self::Literal(literal) => Expression::Literal(literal.clone()),
            Self::Variable { name } => Expression::Variable { name: name.clone() },
            Self::Call { name, args, span } => Expression::Call {
                name: *id_map.get(name).unwrap(),
                args: args.iter().map(|arg| arg.translate_ids(id_map)).collect(),
                span: *span,
            },
        }
    }

    fn unresolved_symbols(&self, id_map: &HashMap<String, FunctionId>) -> Vec<String> {
        match self {
            Expression::Literal(_) | Expression::Variable { .. } => vec![],
            Expression::Call { name, args, .. } => {
                let args_unresolved = args.iter().flat_map(|arg| arg.unresolved_symbols(id_map));

                if id_map.contains_key(name) {
                    None
                } else {
                    Some(name.to_owned())
                }
                .into_iter()
                .chain(args_unresolved)
                .collect()
            }
        }
    }
}

#[derive(Debug)]
pub enum Value {
    String(String),
    Bool(bool),
    None,
}
impl Value {
    fn truthy(&self) -> bool {
        match self {
            Value::String(s) => !s.is_empty(),
            Value::Bool(b) => *b,
            Value::None => false,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Literal {
    String(String),
    Bool(bool),
}

impl Literal {
    fn run(&self) -> Value {
        match self {
            Self::String(string) => Value::String(string.clone()),
            Self::Bool(b) => Value::Bool(*b),
        }
    }

    fn parse<'a>() -> impl Parser<'a, Self> {
        // TODO: Support other literal types + full python string literals
        context("literal", alt((Self::parse_string(), Self::parse_bool())))
    }

    fn parse_string<'a>() -> impl Parser<'a, Self> {
        delimited(tag("\""), is_not("\""), tag("\""))
            .map(|contents: Span| Self::String(contents.fragment().to_string()))
    }

    fn parse_bool<'a>() -> impl Parser<'a, Self> {
        alt((tag("True"), tag("False"))).map(|contents: Span| {
            Self::Bool(match *(contents.fragment()) {
                "True" => true,
                "False" => false,
                _ => unreachable!("Unexpected bool literal value"),
            })
        })
    }
}

#[derive(Error, Debug)]
#[error("Parse error:\n{0}")]
pub struct ParseError(String);

impl ParseError {
    pub fn text(&self) -> &str {
        &self.0
    }
}

fn identifier<'a>() -> impl Parser<'a, Span<'a>> {
    context(
        "identifier",
        recognize(pair(
            alt((alpha1, tag("_"))),
            many0(alt((alphanumeric1, tag("_")))),
        )),
    )
}

fn discard_newline_indent(prefix: Option<&str>) -> impl Parser<()> {
    move |input| {
        if let Some(prefix) = prefix {
            discard(tuple((eol(), blank_lines(), tag(prefix)))).parse(input)
        } else {
            Ok((input, ()))
        }
    }
}

fn discard_indent(prefix: Option<&str>) -> impl Parser<()> {
    move |input| {
        if let Some(prefix) = prefix {
            discard(tag(prefix)).parse(input)
        } else {
            Ok((input, ()))
        }
    }
}

fn ws<'a, F, O>(inner: F) -> impl Parser<'a, O>
where
    F: Parser<'a, O>,
{
    delimited(space0, inner, space0)
}

fn multiline_ws<'a, F, O>(inner: F) -> impl Parser<'a, O>
where
    F: Parser<'a, O>,
{
    delimited(multispace0, inner, multispace0)
}

fn discard<'a, F, O>(inner: F) -> impl Parser<'a, ()>
where
    F: Parser<'a, O>,
{
    map(inner, |_| ())
}

macro_rules! keywords {
    ($($kw:ident $(($kw_text:literal))?),*) => {
        $(
            keyword!($kw $( ($kw_text) )?);
        )*
    };
}

macro_rules! keyword {
    ($kw:ident) => {
        fn $kw(input: Span) -> ParseResult<Span> {
            tag(stringify!($kw)).parse(input)
        }
    };
    ($kw:ident($kw_text:literal)) => {
        fn $kw(input: Span) -> ParseResult<Span> {
            tag($kw_text).parse(input)
        }
    };
}

keywords!(def, pass, r#if("if"), r#else("else"));

macro_rules! operators {
    ($(($name:ident, $op:expr)),*) => {
        $(
            fn $name(input: Span) -> ParseResult<()> {
                ws(discard(tag($op))).parse(input)
            }
        )*
    }
}

operators!((colon, ":"));

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SrcSpan {
    line: usize,
    column: usize,
    len: usize,
}

impl SrcSpan {
    pub fn from_span(span: &Span) -> Self {
        Self {
            line: span.location_line() as usize,
            column: span.get_utf8_column(),
            len: span.fragment().len(),
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::{parse, Expression, Function, Literal, Module, SrcSpan, Statement};
    use crate::syntax_tree::Body;

    #[test]
    fn empty_fn() {
        parse_function_body(
            indoc! {"
                def test():
                    pass
            "},
            [Statement::Pass],
        );
    }

    #[test]
    fn multi_line() {
        parse_function_body(
            indoc! {"
                def test():
                    pass
                    pass
            "},
            [Statement::Pass, Statement::Pass],
        );
    }

    #[test]
    fn blank_line() {
        parse_function_body(
            indoc! {"
                def test():

                    pass
            "},
            [Statement::Pass],
        );
    }

    #[test]
    fn comment() {
        parse_function_body(
            indoc! {"
                def test():
                    # Comment
                    pass
            "},
            [Statement::Pass],
        );
    }

    #[test]
    fn variable_expression() {
        parse_expression(
            indoc! {"
                def test():
                    x
            "},
            Expression::Variable {
                name: "x".to_string(),
            },
        );
    }

    #[test]
    fn call0_expression() {
        parse_expression(
            indoc! {"
                def test():
                    x()
            "},
            Expression::Call {
                name: "x".to_string(),
                args: Vec::new(),
                span: src_span(2, 5, 1),
            },
        );
    }

    #[test]
    fn call1_expression() {
        parse_expression(
            indoc! {"
                def test():
                    x(y)
            "},
            Expression::Call {
                name: "x".to_string(),
                args: vec![Expression::Variable {
                    name: "y".to_string(),
                }],
                span: src_span(2, 5, 1),
            },
        );
    }

    #[test]
    fn call2_expression() {
        parse_expression(
            indoc! {"
                def test():
                    x(y, z)
            "},
            Expression::Call {
                name: "x".to_string(),
                args: vec![
                    Expression::Variable {
                        name: "y".to_string(),
                    },
                    Expression::Variable {
                        name: "z".to_string(),
                    },
                ],
                span: src_span(2, 5, 1),
            },
        );
    }

    #[test]
    fn call2_multiline_expression() {
        parse_expression(
            indoc! {"
                def test():
                    x(
                        y,
                        z
                    )
            "},
            Expression::Call {
                name: "x".to_string(),
                args: vec![
                    Expression::Variable {
                        name: "y".to_string(),
                    },
                    Expression::Variable {
                        name: "z".to_string(),
                    },
                ],
                span: src_span(2, 5, 1),
            },
        );
    }

    #[test]
    fn string_literal() {
        parse_expression(
            indoc! {"
                def test():
                    print(\"Hello, world!\")
            "},
            Expression::Call {
                name: "print".to_string(),
                args: vec![Expression::Literal(Literal::String(
                    "Hello, world!".to_string(),
                ))],
                span: src_span(2, 5, 5),
            },
        );
    }

    fn parse_expression(input: &str, expression: Expression<String>) {
        parse_function_body(input, [Statement::Expression(expression)])
    }

    fn parse_function_body<const COUNT: usize>(input: &str, body: [Statement<String>; COUNT]) {
        assert_eq!(
            parse(input).unwrap(),
            Module {
                functions: vec![Function {
                    name: "test".to_owned(),
                    span: src_span(1, 5, 4),
                    body: Body::new(body),
                }],
            }
        );
    }

    fn src_span(line: usize, column: usize, len: usize) -> SrcSpan {
        SrcSpan { line, column, len }
    }
}
