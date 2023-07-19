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
        self.body.map(|body| body.item())
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

    pub fn item(&self) -> impl Signal<Item = Option<Item>> {
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
                syntax_tree::Statement::Expression(expr) => match expr {
                    syntax_tree::Expression::Literal(_) => (),
                    syntax_tree::Expression::Variable { .. } => (),
                    syntax_tree::Expression::Call { span, name, args } => {
                        stmts.push(Statement::Call(Call::new(library, *span, *name, args)))
                    }
                },
                // TODO: Implement
                syntax_tree::Statement::If { .. } => stmts.push(Statement::If),
            }
        }

        Self(Rc::new(stmts))
    }
}

pub enum Statement {
    Call(Call),
    If,
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
        let args =
            args.iter()
                .filter_map(|arg| match arg {
                    syntax_tree::Expression::Literal(_)
                    | syntax_tree::Expression::Variable { .. } => None,
                    syntax_tree::Expression::Call { span, name, args } => {
                        Some(Self::new(library, *span, *name, args))
                    }
                })
                .collect();
        let body = Body::from_linked_body(library, function.body());

        Self {
            span,
            name,
            args,
            body,
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
        self.body.map(|body| body.item())
    }
}
