use futures_signals::signal::Mutable;
use silkenweb::{
    elements::html::{div, DivBuilder},
    prelude::{ElementEvents, HtmlElement, ParentBuilder},
    value::Sig,
};

pub trait Splitter {
    /// Adds `node` as a child, and a splitter bar that adjusts the size of
    /// `node`.
    fn horizontal_splitter_bar(self, elem: DivBuilder) -> Self;
}

impl Splitter for DivBuilder {
    fn horizontal_splitter_bar(self, elem: DivBuilder) -> Self {
        let style = Mutable::new("".to_string());
        let style_signal = Sig(style.signal_cloned());
        let splitter =
            div().on_mousemove(move |event, _| style.set(format!("width: {}px", event.client_x())));

        self.child(elem.style(style_signal)).child(splitter)
    }
}
