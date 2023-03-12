use std::rc::Rc;

use futures_signals::signal::{Mutable, Signal};
use once_cell::unsync::Lazy;
use serpent_automation_executor::{
    library::{FunctionId, Library},
    syntax_tree::LinkedBody,
};

use crate::is_expandable;

pub struct CallTree {
    name: String,
    body: Vertex<Accordion<Body>>,
}

impl CallTree {
    pub fn root(fn_id: FunctionId, library: &Rc<Library>) -> Self {
        let f = library.lookup(fn_id);
        let body = match f.body() {
            LinkedBody::Local(body) if is_expandable(body) => {
                Vertex::Node(Accordion::new(Lazy::new(|| Body {})))
            }
            LinkedBody::Python | LinkedBody::Local(_) => Vertex::Leaf,
        };

        Self {
            name: f.name().to_string(),
            body,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn children(&self) -> Vertex<impl Signal<Item = Option<Body>>> {
        self.body.map(|body| body.item())
    }
}

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

pub struct Accordion<Item> {
    expanded: Mutable<bool>,
    item: Rc<Lazy<Item>>,
}

impl<Item: Clone> Accordion<Item> {
    pub fn new(item: Lazy<Item>) -> Self {
        Self {
            expanded: Mutable::new(false),
            item: Rc::new(item),
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

// TODO: Implement
#[derive(Clone)]
pub struct Body {}
