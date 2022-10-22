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
use thiserror::Error;
use tokio::sync::watch;

use crate::{
    library::{FunctionId, Library},
    run::ThreadCallStates,
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
    functions: Vec<Function<String>>,
}

impl Module {
    fn parse(input: Span) -> ParseResult<Self> {
        let (input, (functions, _)) =
            context("module", many_till(multiline_ws(Function::parse), eof))(input)?;

        Ok((input, Module { functions }))
    }

    pub fn functions(&self) -> &[Function<String>] {
        &self.functions
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Function<Id> {
    name: String,
    body: Arc<Vec<Statement<Id>>>,
}

impl Function<String> {
    fn parse(input: Span) -> ParseResult<Self> {
        let (input, (_def, _, name, _params, _colon, body)) = context(
            "function",
            tuple((
                def,
                space1,
                identifier,
                ws(tag("()")),
                colon,
                alt((Self::inline_body, Self::block_body)),
            )),
        )(input)?;

        Ok((
            input,
            Function {
                name: name.fragment().to_string(),
                body: Arc::new(body),
            },
        ))
    }

    fn inline_body(input: Span) -> ParseResult<Vec<Statement<String>>> {
        let (input, statement) = context("inline body", Statement::parse)(input)?;

        Ok((input, vec![statement]))
    }

    fn block_body(input: Span) -> ParseResult<Vec<Statement<String>>> {
        let (input, _) = discard(pair(eol, blank_lines))(input)?;
        let (input, prefix) = space0(input)?;

        // TODO: Is error reporting friendly enough?
        separated_list1(
            tuple((eol, blank_lines, tag(*prefix.fragment()))),
            Statement::parse,
        )(input)
    }

    pub fn translate_ids<Id: Clone>(&self, id_map: &IdMap<Id>) -> Function<Id> {
        Function {
            name: self.name.clone(),
            body: Arc::new(
                self.body
                    .iter()
                    .map(|statement| statement.translate_ids(id_map))
                    .collect(),
            ),
        }
    }
}

impl<Id> Function<Id> {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn body(&self) -> &Arc<Vec<Statement<Id>>> {
        &self.body
    }
}

impl Function<FunctionId> {
    pub fn run(
        &self,
        _args: impl Iterator<Item = Value>,
        lib: &Library,
        call_states: &watch::Sender<ThreadCallStates>,
    ) {
        println!("Running function '{}'", self.name());
        sleep(Duration::from_secs(2));

        for stmt in self.body().iter() {
            stmt.run(lib, call_states)
        }
    }
}

pub type IdMap<Id> = HashMap<String, Id>;

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
    // TODO: Loops
}

impl Statement<String> {
    fn parse(input: Span) -> ParseResult<Self> {
        let (input, stmt) = context(
            "statement",
            alt((
                map(pass, |_| Statement::Pass),
                map(Expression::parse, Statement::Expression),
            )),
        )(input)?;

        Ok((input, stmt))
    }

    fn translate_ids<Id: Clone>(&self, id_map: &IdMap<Id>) -> Statement<Id> {
        match self {
            Self::Pass => Statement::Pass,
            Self::Expression(expression) => Statement::Expression(expression.translate_ids(id_map)),
        }
    }
}

impl Statement<FunctionId> {
    pub fn run(&self, lib: &Library, call_states: &watch::Sender<ThreadCallStates>) {
        match self {
            Statement::Pass => (),
            Statement::Expression(expr) => {
                expr.run(lib, call_states);
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
                let args = args.iter().map(|arg| arg.run(lib, call_states));

                call_states.send_modify(|t| t.push(*name));
                lib.lookup(*name).run(args, lib, call_states);
                call_states.send_modify(|t| t.pop());

                Value::None
            }
            Expression::Literal(literal) => literal.run(),
        }
    }
}

impl Expression<String> {
    fn parse(input: Span) -> ParseResult<Self> {
        alt((Self::call, Self::variable, Self::parenthasized))(input)
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

    fn translate_ids<Id: Clone>(&self, id_map: &IdMap<Id>) -> Expression<Id> {
        match self {
            Self::Literal(literal) => Expression::Literal(literal.clone()),
            Self::Variable { name } => Expression::Variable { name: name.clone() },
            Self::Call { name, args } => Expression::Call {
                name: id_map.get(name).unwrap().clone(),
                args: args.iter().map(|arg| arg.translate_ids(id_map)).collect(),
            },
        }
    }
}

pub enum Value {
    String(String),
    None,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Literal {
    String(String),
}
impl Literal {
    fn run(&self) -> Value {
        match self {
            Literal::String(string) => Value::String(string.clone()),
        }
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
    ($($kw:ident),*) => {
        $(
            fn $kw(input: Span) -> ParseResult<()> {
                discard(tag(stringify!($kw)))(input)
            }
        )*
    }
}

keywords!(def, pass);

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
    use std::sync::Arc;

    use indoc::indoc;

    use super::{parse, Expression, Function, Module, Statement};

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

    fn parse_expression(input: &str, expression: Expression<String>) {
        parse_function_body(input, [Statement::Expression(expression)])
    }

    fn parse_function_body<const COUNT: usize>(input: &str, body: [Statement<String>; COUNT]) {
        assert_eq!(
            parse(input).unwrap(),
            Module {
                functions: vec![Function {
                    name: "test".to_owned(),
                    body: Arc::new(body.into()),
                }],
            }
        );
    }
}
