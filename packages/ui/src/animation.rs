use std::{cell::Cell, rc::Rc};

use futures_signals::signal::{Mutable, SignalExt};
use silkenweb::{
    clone,
    elements::html::{div, DivBuilder},
    node::element::{Element, ElementBuilder},
    prelude::{HtmlElement, HtmlElementEvents, ParentBuilder},
    task::on_animation_frame,
    value::Sig,
};
use web_sys::DomRect;

use crate::css;

fn style_size(limit: &str, bounds: &DomRect) -> String {
    let width = bounds.width();
    let height = bounds.height();
    format!("overflow: hidden; {limit}-width: {width}px; {limit}-height: {height}px",)
}

pub trait AnimatedExpand {
    fn animated_expand<Elem>(
        self,
        child: impl FnMut() -> Elem + 'static,
        expanded: Mutable<bool>,
    ) -> Self
    where
        Elem: Into<Element>;
}

impl AnimatedExpand for DivBuilder {
    fn animated_expand<Elem>(
        self,
        mut child: impl FnMut() -> Elem + 'static,
        expanded: Mutable<bool>,
    ) -> Self
    where
        Elem: Into<Element>,
    {
        let style = Mutable::new("".to_owned());
        let initial_bounds: Rc<Cell<Option<DomRect>>> = Rc::new(Cell::new(None));

        let expanding_elem = div()
            .class(css::TRANSITION_ALL)
            .effect_signal(expanded.signal(), {
                clone!(style);
                move |elem, expanded| {
                    let final_bounds = elem.get_bounding_client_rect();

                    if let Some(initial_bounds) = initial_bounds.replace(Some(final_bounds.clone()))
                    {
                        let limit = if expanded { "max" } else { "min" };

                        style.set(style_size(limit, &initial_bounds));

                        on_animation_frame({
                            clone!(style);
                            move || {
                                style.set(style_size(limit, &final_bounds));
                            }
                        })
                    }
                }
            })
            .on_transitionend({
                clone!(style);
                move |_, _| style.set("".to_owned())
            })
            .style(Sig(style.signal_cloned()))
            .optional_child(Sig(expanded
                .signal()
                .map(move |expanded| expanded.then(|| child().into()))));

        self.child(expanding_elem)
    }
}
