use std::{cell::Cell, rc::Rc};

use futures_signals::signal::Mutable;
use silkenweb::{
    clone, document,
    elements::html::DivBuilder,
    node::element::ElementBuilder,
    prelude::{ElementEvents, HtmlElement, ParentBuilder},
    value::Sig,
};
use wasm_bindgen::JsCast;
use web_sys::{Event, MouseEvent};

pub trait Splitter {
    /// Adds `node` as a child, and a splitter bar that adjusts the size of
    /// `node`.
    fn horizontal_splitter(self, elem: DivBuilder, splitter: DivBuilder) -> Self;
}

impl Splitter for DivBuilder {
    fn horizontal_splitter(self, elem: DivBuilder, splitter: DivBuilder) -> Self {
        let style = Mutable::new("".to_string());
        let style_signal = Sig(style.signal_cloned());
        let events: Rc<Cell<Option<[document::EventCallback; 2]>>> = Rc::new(Cell::new(None));
        self.child(elem.style(style_signal))
            .child(splitter.on_mousedown({
                move |event, _| {
                    // Otherwise we'll "select" things in the panes.
                    // TODO: Grok these
                    event.prevent_default();
                    event.stop_propagation();

                    if is_left_button(&event) {
                        clone!(style);
                        let move_handler = move |event: MouseEvent| {
                            // TODO: Account for where we're clicking in the bar.
                            style.set(format!("width: {}px", event.client_x()))
                        };

                        // TODO: MDN docs doesn't have `mousemove`, `mouseup` events listed.
                        // TODO: Should these go on window, not document? Behaviour is slightly
                        // weird when you move the mouse outside the browser window while dragging.
                        // TODO: Also "touchmove"
                        let move_event = document::on_mousemove(move_handler);

                        let events_weak = Rc::downgrade(&events);
                        let stop_handler = move |event| {
                            if is_left_button(&event) {
                                if let Some(events) = events_weak.upgrade() {
                                    events.set(None);
                                }
                            }
                        };

                        let stop_event = document::on_mouseup(stop_handler);
                        events.set(Some([move_event, stop_event]));
                    }
                }
            }))
    }
}

fn is_left_button(event: &MouseEvent) -> bool {
    event.button() == 0
}
