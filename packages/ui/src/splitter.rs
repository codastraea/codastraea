use std::{cell::Cell, rc::Rc};

use futures_signals::signal::Mutable;
use silkenweb::{
    clone,
    elements::html::DivBuilder,
    prelude::{ElementEvents, HtmlElement, ParentBuilder},
    value::Sig,
};
use web_sys::MouseEvent;

pub trait Splitter {
    /// Adds `node` as a child, and a splitter bar that adjusts the size of
    /// `node`.
    fn horizontal_splitter(self, elem: DivBuilder, splitter: DivBuilder) -> Self;
}

impl Splitter for DivBuilder {
    fn horizontal_splitter(self, elem: DivBuilder, splitter: DivBuilder) -> Self {
        let style = Mutable::new("".to_string());
        let style_signal = Sig(style.signal_cloned());
        let is_left_mouse_down = Rc::new(Cell::new(false));

        self.child(elem.style(style_signal)).child(
            splitter
                .on_mousedown({
                    clone!(is_left_mouse_down);
                    move |event, _| {
                        if is_left_button(&event) {
                            is_left_mouse_down.set(true);
                        }
                    }
                })
                // TODO: These 2 events need to be on the document, but removed when the split bar
                // is removed.
                .on_mouseup({
                    clone!(is_left_mouse_down);
                    move |event, _| {
                        if is_left_button(&event) {
                            is_left_mouse_down.set(false);
                        }
                    }
                })
                .on_mousemove(move |event, _| {
                    if is_left_mouse_down.get() {
                        style.set(format!("width: {}px", event.client_x()))
                    }
                }),
        )
    }
}

fn is_left_button(_event: &MouseEvent) -> bool {
    false
    // TODO: event.button() == 0
}
