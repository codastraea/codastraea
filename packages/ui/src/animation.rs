use std::{cell::RefCell, rc::Rc};

use futures_signals::signal::{Mutable, Signal, SignalExt};
use silkenweb::{
    clone,
    elements::html::Div,
    node::element::Element,
    prelude::{ElementEvents, HtmlElement, Node, ParentElement},
    task::on_animation_frame,
    value::{Sig, Value},
};
use web_sys::DomRect;

use crate::css;

pub trait AnimatedExpand {
    fn animated_expand<Elem>(self, child: impl Signal<Item = Option<Elem>> + 'static) -> Self
    where
        Elem: Into<Node> + Value + 'static;
}

impl AnimatedExpand for Div {
    fn animated_expand<Elem>(self, child: impl Signal<Item = Option<Elem>> + 'static) -> Self
    where
        Elem: Into<Node> + Value + 'static,
    {
        let style = Mutable::new(None);
        let delayed_is_expanded = Mutable::<Option<bool>>::new(None);
        let initial_bounds: Rc<RefCell<Option<DomRect>>> = Rc::new(RefCell::new(None));
        let element = self.handle().dom_element();
        let delayed_is_expanded_signal = delayed_is_expanded.signal();

        self.class(css::TRANSITION)
            .style(Sig(style.signal_cloned()))
            .optional_child(Sig(child.map({
                clone!(initial_bounds);
                move |child| {
                    let existing_initial_bounds =
                        initial_bounds.replace(Some(element.get_bounding_client_rect()));

                    if existing_initial_bounds.is_some() {
                        delayed_is_expanded.set(Some(child.is_some()));
                    }

                    child
                }
            })))
            .on_transitionend({
                clone!(style);
                move |_, _| style.set_neq(None)
            })
            .on_transitioncancel({
                clone!(style);
                move |_, _| style.set_neq(None)
            })
            .effect_signal(delayed_is_expanded_signal, move |elem, expanded| {
                if let Some(expanded) = expanded {
                    let initial_bounds = initial_bounds.borrow().as_ref().unwrap().clone();
                    let final_bounds = elem.get_bounding_client_rect();

                    let limit = if expanded { "max" } else { "min" };
                    set_style_size(&style, limit, &initial_bounds);

                    on_animation_frame({
                        clone!(style);
                        move || set_style_size(&style, limit, &final_bounds)
                    })
                }
            })
    }
}

fn set_style_size(style: &Mutable<Option<String>>, limit: &str, bounds: &DomRect) {
    let width = bounds.width();
    let height = bounds.height();
    style.set(Some(format!(
        "overflow: hidden; {limit}-width: {width}px; {limit}-height: {height}px",
    )));
}
