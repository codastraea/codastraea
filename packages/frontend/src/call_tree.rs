use std::rc::Rc;

use futures_signals::signal::{Mutable, Signal};
use once_cell::unsync::Lazy;
use serpent_automation_executor::{
    library::{FunctionId, Library},
    syntax_tree::{self, LinkedBody, SrcSpan},
};

use crate::is_expandable;

pub struct CallTree {
    name: String,
    body: Vertex<Expandable<Body>>,
}

impl CallTree {
    pub fn root(fn_id: FunctionId, library: &Rc<Library>) -> Self {
        let f = library.lookup(fn_id);

        Self {
            name: f.name().to_string(),
            body: Body::from_linked_body(library, f.body()),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn children(&self) -> Vertex<impl Signal<Item = Option<Body>>> {
        self.body.signal()
    }
}

#[derive(Clone)]
pub enum Vertex<Children> {
    Leaf,
    Node(Children),
}

impl<Children> Vertex<Children> {
    pub fn map<R>(&self, f: impl FnOnce(&Children) -> R) -> Vertex<R> {
        match self {
            Vertex::Leaf => Vertex::Leaf,
            Vertex::Node(children) => Vertex::Node(f(children)),
        }
    }
}

impl<Children> From<Option<Children>> for Vertex<Children> {
    fn from(value: Option<Children>) -> Self {
        value.map_or(Self::Leaf, |children| Vertex::Node(children))
    }
}

impl<Children: Clone> Vertex<Expandable<Children>> {
    pub fn signal(&self) -> Vertex<impl Signal<Item = Option<Children>>> {
        self.map(|body| body.signal())
    }
}

#[derive(Clone)]
pub struct Expandable<Item> {
    expanded: Mutable<bool>,
    item: Rc<DynLazy<Item>>,
}

type DynLazy<T> = Lazy<T, Box<dyn FnOnce() -> T>>;

impl<Item: Clone> Expandable<Item> {
    pub fn new(f: impl FnOnce() -> Item + 'static) -> Self {
        Self {
            expanded: Mutable::new(false),
            item: Rc::new(Lazy::new(Box::new(f))),
        }
    }

    pub fn collapse(&self) {
        self.expanded.set_neq(false)
    }

    pub fn expand(&self) {
        self.expanded.set_neq(true)
    }

    pub fn signal(&self) -> impl Signal<Item = Option<Item>> {
        let item = self.item.clone();

        self.expanded
            .signal_ref(move |expanded| expanded.then(|| (*item).clone()))
    }
}

#[derive(Clone)]
pub struct Body(Rc<Vec<Statement>>);

impl Body {
    fn from_linked_body(
        library: &Rc<Library>,
        body: &syntax_tree::LinkedBody,
    ) -> Vertex<Expandable<Self>> {
        match body {
            LinkedBody::Local(body) if is_expandable(body) => {
                let body = body.clone();
                Vertex::Node(Expandable::new({
                    let library = library.clone();
                    move || Self::from_body(&library, &body)
                }))
            }
            LinkedBody::Python | LinkedBody::Local(_) => Vertex::Leaf,
        }
    }

    fn from_body(library: &Rc<Library>, body: &syntax_tree::Body<FunctionId>) -> Self {
        let mut stmts = Vec::new();

        for stmt in body.iter() {
            match stmt {
                syntax_tree::Statement::Pass => (),
                syntax_tree::Statement::Expression(expr) => {
                    stmts.extend(Call::from_expression(library, expr).map(Statement::Call))
                }
                syntax_tree::Statement::If {
                    if_span,
                    condition,
                    then_block,
                    else_block,
                } => stmts.push(Statement::If(If::new(
                    library, *if_span, condition, then_block, else_block,
                ))),
            }
        }

        Self(Rc::new(stmts))
    }
}

pub enum Statement {
    Call(Call),
    If(If),
}

#[derive(Clone)]
pub struct Call {
    span: SrcSpan,
    args: Vec<Self>,
    name: String,
    body: Vertex<Expandable<Body>>,
}

impl Call {
    fn new(
        library: &Rc<Library>,
        span: SrcSpan,
        name: FunctionId,
        args: &[syntax_tree::Expression<FunctionId>],
    ) -> Self {
        let function = &library.lookup(name);
        let name = function.name().to_string();
        let args = args
            .iter()
            .filter_map(|arg| Self::from_expression(library, arg))
            .collect();
        let body = Body::from_linked_body(library, function.body());

        Self {
            span,
            name,
            args,
            body,
        }
    }

    fn from_expression(
        library: &Rc<Library>,
        expr: &syntax_tree::Expression<FunctionId>,
    ) -> Option<Call> {
        match expr {
            syntax_tree::Expression::Literal(_) | syntax_tree::Expression::Variable { .. } => None,
            syntax_tree::Expression::Call { span, name, args } => {
                Some(Self::new(library, *span, *name, args))
            }
        }
    }

    pub fn span(&self) -> SrcSpan {
        self.span
    }

    pub fn args(&self) -> &[Call] {
        &self.args
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn body(&self) -> Vertex<impl Signal<Item = Option<Body>>> {
        self.body.signal()
    }
}

pub struct If {
    span: SrcSpan,
    condition: Vertex<Expandable<Call>>,
    then_block: Body,
    else_block: Option<Body>,
}

impl If {
    fn new(
        library: &Rc<Library>,
        span: SrcSpan,
        condition: &syntax_tree::Expression<FunctionId>,
        then_block: &syntax_tree::Body<FunctionId>,
        else_block: &Option<syntax_tree::ElseClause<FunctionId>>,
    ) -> Self {
        Self {
            span,
            condition: Call::from_expression(library, condition)
                .map(|cond| Expandable::new(|| cond))
                .into(),
            then_block: Body::from_body(library, then_block),
            else_block: else_block
                .as_ref()
                .map(|el_blk| Body::from_body(library, el_blk.body())),
        }
    }

    pub fn span(&self) -> SrcSpan {
        self.span
    }

    pub fn condition(&self) -> Vertex<impl Signal<Item = Option<Call>>> {
        self.condition.signal()
    }

    pub fn then_block(&self) -> &Body {
        &self.then_block
    }

    pub fn else_block(&self) -> &Option<Body> {
        &self.else_block
    }
}
