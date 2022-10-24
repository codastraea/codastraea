use std::{collections::HashMap, sync::Arc, thread::sleep, time::Duration};

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{alpha1, alphanumeric1, line_ending, multispace0, space0, space1},
    combinator::{all_consuming, eof, map, opt, recognize},
    error::{context, ErrorKind},
    multi::{many0, many_till, separated_list0, separated_list1},
    sequence::{delimited, pair, separated_pair, tuple},
    Finish, IResult,
};
use nom_greedyerror::{convert_error, GreedyError};
use nom_locate::LocatedSpan;
use scopeguard::defer;
use thiserror::Error;
use tokio::sync::watch;

use crate::{
    library::{FunctionId, Library},
    run::{StackFrame, ThreadCallStates},
};

pub fn parse(input: &str) -> Result<Module, ParseError> {
    match all_consuming(Module::parse)(Span::new(input)).finish() {
        Ok((_, module)) => Ok(module),
        Err(e) => Err(ParseError(convert_error(input, e))),
    }
}

type Span<'a> = LocatedSpan<&'a str>;

type ParseResult<'a, T> = IResult<Span<'a>, T, GreedyError<Span<'a>, ErrorKind>>;

#[derive(PartialEq, Eq, Debug)]
pub struct Module {
    functions: Vec<Function>,
}

impl Module {
    fn parse(input: Span) -> ParseResult<Self> {
        let (input, (functions, _)) =
            context("module", many_till(multiline_ws(Function::parse), eof))(input)?;

        Ok((input, Module { functions }))
    }

    pub fn functions(&self) -> &[Function] {
        &self.functions
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Function {
    name: String,
    body: Body<String>,
}

impl Function {
    pub fn name(&self) -> &str {
        &self.name
    }

    fn parse(input: Span) -> ParseResult<Self> {
        let (input, (_def, _, name, _params, _colon, body)) = context(
            "function",
            tuple((def, space1, identifier, ws(tag("()")), colon, Body::parse)),
        )(input)?;

        Ok((
            input,
            Function {
                name: name.fragment().to_string(),
                body,
            },
        ))
    }

    pub fn translate_ids(&self, id_map: &IdMap) -> LinkedFunction {
        LinkedFunction::local(
            &self.name,
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
    body: LinkedBody,
}

impl LinkedFunction {
    pub fn local(name: &str, body: impl IntoIterator<Item = Statement<FunctionId>>) -> Self {
        Self {
            name: name.to_owned(),
            body: LinkedBody::Local(Arc::new(Body::new(body))),
        }
    }

    pub fn python(name: String) -> Self {
        Self {
            name,
            body: LinkedBody::Python,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn body(&self) -> &LinkedBody {
        &self.body
    }

    pub fn run(
        &self,
        args: &[Value],
        lib: &Library,
        call_states: &watch::Sender<ThreadCallStates>,
    ) {
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

// TODO: Rename (Body -> LinkedBody, LocalBody -> Body)
#[derive(Debug, Eq, PartialEq)]
pub struct Body<T>(Vec<Statement<T>>);

impl<T> Body<T> {
    pub fn new(stmts: impl IntoIterator<Item = Statement<T>>) -> Self {
        Self(stmts.into_iter().collect())
    }

    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Statement<T>> {
        self.0.iter()
    }
}

impl Body<String> {
    pub fn parse(input: Span) -> ParseResult<Self> {
        // TODO: Make sure body is more indented than parent
        map(alt((Self::parse_inline, Self::parse_block)), Self::new)(input)
    }

    fn parse_inline(input: Span) -> ParseResult<Vec<Statement<String>>> {
        let (input, statement) = context("inline body", Statement::parse(None))(input)?;

        Ok((input, vec![statement]))
    }

    fn parse_block(input: Span) -> ParseResult<Vec<Statement<String>>> {
        let (input, _) = discard(pair(eol, blank_lines))(input)?;
        let (input, prefix) = space0(input)?;

        // TODO: Is error reporting friendly enough?
        separated_list1(
            discard_indent(Some(prefix.fragment())),
            Statement::parse(Some(prefix.fragment())),
        )(input)
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
    pub fn run(&self, lib: &Library, call_states: &watch::Sender<ThreadCallStates>) {
        for (index, stmt) in self.iter().enumerate() {
            call_states.send_modify(|t| t.push(StackFrame::Statement(index)));
            defer! {call_states.send_modify(|t| t.pop());}
            stmt.run(lib, call_states);
        }
    }
}

pub type IdMap = HashMap<String, FunctionId>;

fn blank_lines(input: Span) -> ParseResult<()> {
    discard(many0(pair(space0, eol)))(input)
}

fn eol(input: Span) -> ParseResult<()> {
    discard(tuple((
        space0,
        opt(pair(tag("#"), is_not("\r\n"))),
        line_ending,
    )))(input)
}

#[derive(Eq, PartialEq, Debug)]
pub enum Statement<FnId> {
    Pass,
    Expression(Expression<FnId>),
    If {
        condition: Expression<FnId>,
        then_block: Arc<Body<FnId>>,
        else_block: Arc<Body<FnId>>,
    },
}

impl Statement<String> {
    fn parse<'a>(prefix: Option<&'a str>) -> impl FnMut(Span<'a>) -> ParseResult<'a, Self> {
        context(
            "statement",
            alt((
                map(pass, |_| Statement::Pass),
                move |input| Self::parse_if(prefix, input),
                map(Expression::parse, Statement::Expression),
            )),
        )
    }

    fn parse_if<'a>(prefix: Option<&'a str>, input: Span<'a>) -> ParseResult<'a, Self> {
        // TODO: elif
        let (input, (_if, condition, _colon, then_block, else_clause)) = context(
            "if",
            tuple((
                r#if,
                ws(Expression::parse),
                ws(colon),
                Body::parse,
                opt(tuple((
                    discard_indent(prefix),
                    r#else,
                    ws(colon),
                    Body::parse,
                ))),
            )),
        )(input)?;

        let statement = Self::If {
            condition,
            then_block: Arc::new(then_block),
            else_block: Arc::new(
                else_clause.map_or_else(Body::empty, |(_indent, _else, _colon, else_block)| {
                    else_block
                }),
            ),
        };

        Ok((input, statement))
    }

    fn translate_ids(&self, id_map: &IdMap) -> Statement<FunctionId> {
        match self {
            Self::Pass => Statement::Pass,
            Self::Expression(expression) => Statement::Expression(expression.translate_ids(id_map)),
            Self::If {
                condition,
                then_block,
                else_block,
            } => Statement::If {
                condition: condition.translate_ids(id_map),
                then_block: Arc::new(then_block.translate_ids(id_map)),
                else_block: Arc::new(else_block.translate_ids(id_map)),
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
            } => {
                let mut unresolved = condition.unresolved_symbols(id_map);

                unresolved.extend(then_block.unresolved_symbols(id_map));
                unresolved.extend(else_block.unresolved_symbols(id_map));

                unresolved
            }
        }
    }
}

impl Statement<FunctionId> {
    pub fn run(&self, lib: &Library, call_states: &watch::Sender<ThreadCallStates>) {
        match self {
            Self::Pass => (),
            Self::Expression(expr) => {
                expr.run(lib, call_states);
            }
            Self::If {
                condition,
                then_block,
                else_block,
            } => {
                if condition.run(lib, call_states).truthy() {
                    call_states.send_modify(|t| t.push(StackFrame::NestedBlock(0)));
                    defer! {call_states.send_modify(|t| t.pop());}
                    then_block.run(lib, call_states)
                } else {
                    // TODO: Don't forget to bump `index` up from 1 when adding `elif`
                    call_states.send_modify(|t| t.push(StackFrame::NestedBlock(1)));
                    defer! {call_states.send_modify(|t| t.pop());}
                    else_block.run(lib, call_states)
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Expression<FnId> {
    Literal(Literal),
    Variable {
        name: String,
    },
    Call {
        name: FnId,
        args: Vec<Expression<FnId>>,
    },
}

impl Expression<FunctionId> {
    pub fn run(&self, lib: &Library, call_states: &watch::Sender<ThreadCallStates>) -> Value {
        match self {
            Expression::Variable { name } => todo!("Variable {name}"),
            Expression::Call { name, args } => {
                let args: Vec<_> = args
                    .iter()
                    .enumerate()
                    .map(|(index, arg)| {
                        call_states.send_modify(|t| t.push(StackFrame::Argument(index)));
                        defer! {call_states.send_modify(|t| t.pop());}
                        arg.run(lib, call_states)
                    })
                    .collect();

                call_states.send_modify(|t| t.push(StackFrame::Function(*name)));
                defer! {call_states.send_modify(|t| t.pop());}
                lib.lookup(*name).run(&args, lib, call_states);

                Value::None
            }
            Expression::Literal(literal) => literal.run(),
        }
    }
}

impl Expression<String> {
    fn parse(input: Span) -> ParseResult<Self> {
        alt((
            Self::literal,
            Self::call,
            Self::variable,
            Self::parenthasized,
        ))(input)
    }

    fn literal(input: Span) -> ParseResult<Self> {
        map(Literal::parse, Self::Literal)(input)
    }

    fn variable(input: Span) -> ParseResult<Self> {
        map(identifier, |name| Self::Variable {
            name: name.fragment().to_string(),
        })(input)
    }

    fn call(input: Span) -> ParseResult<Self> {
        let (input, (name, args)) = context(
            "call",
            separated_pair(
                identifier,
                space0,
                delimited(
                    tag("("),
                    separated_list0(tag(","), multiline_ws(Self::parse)),
                    tag(")"),
                ),
            ),
        )(input)?;

        Ok((
            input,
            Self::Call {
                name: name.fragment().to_string(),
                args,
            },
        ))
    }

    fn parenthasized(input: Span) -> ParseResult<Self> {
        context(
            "parenthesized",
            delimited(tag("("), multiline_ws(Expression::parse), tag(")")),
        )(input)
    }

    fn translate_ids(&self, id_map: &IdMap) -> Expression<FunctionId> {
        match self {
            Self::Literal(literal) => Expression::Literal(literal.clone()),
            Self::Variable { name } => Expression::Variable { name: name.clone() },
            Self::Call { name, args } => Expression::Call {
                name: *id_map.get(name).unwrap(),
                args: args.iter().map(|arg| arg.translate_ids(id_map)).collect(),
            },
        }
    }

    fn unresolved_symbols(&self, id_map: &HashMap<String, FunctionId>) -> Vec<String> {
        match self {
            Expression::Literal(_) | Expression::Variable { .. } => vec![],
            Expression::Call { name, args } => {
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

    fn parse(input: Span) -> ParseResult<Self> {
        // TODO: Support other literal types + full python string literals
        context("literal", alt((Self::parse_string, Self::parse_bool)))(input)
    }

    fn parse_string(input: Span) -> ParseResult<Self> {
        let (input, contents) = delimited(tag("\""), is_not("\""), tag("\""))(input)?;

        Ok((input, Self::String(contents.fragment().to_string())))
    }

    fn parse_bool(input: Span) -> ParseResult<Self> {
        let (input, contents) = alt((tag("True"), tag("False")))(input)?;

        Ok((
            input,
            Self::Bool(match *(contents.fragment()) {
                "True" => true,
                "False" => false,
                _ => unreachable!("Unexpected bool literal value"),
            }),
        ))
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

fn identifier(input: Span) -> ParseResult<Span> {
    context(
        "identifier",
        recognize(pair(
            alt((alpha1, tag("_"))),
            many0(alt((alphanumeric1, tag("_")))),
        )),
    )(input)
}

fn discard_indent<'a>(prefix: Option<&'a str>) -> impl FnMut(Span<'a>) -> ParseResult<'a, ()> {
    move |input| {
        if let Some(prefix) = prefix {
            discard(tuple((eol, blank_lines, tag(prefix))))(input)
        } else {
            Ok((input, ()))
        }
    }
}

fn ws<'a, F, O>(inner: F) -> impl FnMut(Span<'a>) -> ParseResult<'a, O>
where
    F: 'a + FnMut(Span<'a>) -> ParseResult<'a, O>,
{
    delimited(space0, inner, space0)
}

fn multiline_ws<'a, F, O>(inner: F) -> impl FnMut(Span<'a>) -> ParseResult<'a, O>
where
    F: 'a + FnMut(Span<'a>) -> ParseResult<'a, O>,
{
    delimited(multispace0, inner, multispace0)
}

fn discard<'a, F, O>(inner: F) -> impl FnMut(Span<'a>) -> ParseResult<'a, ()>
where
    F: 'a + FnMut(Span<'a>) -> ParseResult<'a, O>,
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
        fn $kw(input: Span) -> ParseResult<()> {
            discard(tag(stringify!($kw)))(input)
        }
    };
    ($kw:ident($kw_text:literal)) => {
        fn $kw(input: Span) -> ParseResult<()> {
            discard(tag($kw_text))(input)
        }
    };
}

keywords!(def, pass, r#if("if"), r#else("else"));

macro_rules! operators {
    ($(($name:ident, $op:expr)),*) => {
        $(
            fn $name(input: Span) -> ParseResult<()> {
                ws(discard(tag($op)))(input)
            }
        )*
    }
}

operators!((colon, ":"));

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::{parse, Expression, Function, Literal, Module, Statement};
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
                    body: Body::new(body),
                }],
            }
        );
    }
}
