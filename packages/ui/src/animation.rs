use std::{cell::Cell, rc::Rc};

use futures_signals::signal::{Mutable, SignalExt};
use silkenweb::{
    clone,
    elements::html::DivBuilder,
    node::element::{Element, ElementBuilder},
    prelude::{HtmlElement, HtmlElementEvents, ParentBuilder},
    task::on_animation_frame,
    value::Sig,
};
use web_sys::DomRect;

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
        let style = Mutable::new(Some("".to_owned()));
        let initial_bounds: Rc<Cell<Option<DomRect>>> = Rc::new(Cell::new(None));

        self.style(Sig(style.signal_cloned()))
            .optional_child(Sig(expanded
                .signal()
                .map(move |expanded| expanded.then(|| child().into()))))
            .on_transitionend({
                clone!(style);
                move |_, _| style.set(None)
            })
            .effect_signal(expanded.signal(), move |elem, expanded| {
                let final_bounds = elem.get_bounding_client_rect();

                if let Some(initial_bounds) = initial_bounds.replace(Some(final_bounds.clone())) {
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
