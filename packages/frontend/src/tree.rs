use std::rc::Rc;

use futures_signals::signal::{Mutable, Signal};
use once_cell::unsync::Lazy;

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

    pub fn is_expanded(&self) -> &Mutable<bool> {
        &self.expanded
    }

    pub fn signal(&self) -> impl Signal<Item = Option<Item>> {
        let item = self.item.clone();

        self.expanded
            .signal_ref(move |expanded| expanded.then(|| (*item).clone()))
    }
}
