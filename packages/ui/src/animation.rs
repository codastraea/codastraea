use futures_signals::signal::{Mutable, SignalExt};
use silkenweb::{
    clone,
    elements::html::{div, DivBuilder},
    node::element::{Element, ElementBuilder},
    prelude::{HtmlElement, HtmlElementEvents, ParentBuilder},
    task::on_animation_frame,
    value::Sig,
};

use crate::css;

fn style_size(bound: &str, width: f64, height: f64) -> String {
    format!("overflow: hidden; {bound}-width: {width}px; {bound}-height: {height}px",)
}

fn style_max_size(width: f64, height: f64) -> String {
    style_size("max", width, height)
}

fn style_min_size(width: f64, height: f64) -> String {
    style_size("min", width, height)
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
        let show_body = Mutable::new(false);
        let parent = self.handle().dom_element();

        let expanding_elem = div()
            .class(css::TRANSITION_ALL)
            .spawn_future(expanded.signal().for_each({
                clone!(show_body);
                move |expanded| {
                    if expanded {
                        show_body.set(true);
                    }
                    async {}
                }
            }))
            .effect_signal(expanded.signal(), {
                clone!(style, show_body);
                move |elem, expanded| {
                    let elem_bounds = elem.get_bounding_client_rect();

                    if expanded {
                        let initial_width = parent.get_bounding_client_rect().width();
                        let final_bounds = elem.get_bounding_client_rect();
                        style.set(style_max_size(initial_width, 0.0));

                        on_animation_frame({
                            clone!(style);
                            move || {
                                style.set(style_max_size(
                                    final_bounds.width(),
                                    final_bounds.height(),
                                ));
                            }
                        })
                    } else {
                        style.set(style_min_size(elem_bounds.width(), elem_bounds.height()));

                        on_animation_frame({
                            clone!(show_body);
                            move || show_body.set(false)
                        });
                    }
                }
            })
            .on_transitionend({
                clone!(style);
                move |_, _| style.set("".to_owned())
            })
            .style(Sig(style.signal_cloned()))
            .optional_child(Sig(show_body.signal().map({
                move |expanded| {
                    if expanded {
                        Some(child().into())
                    } else {
                        style.set(style_min_size(0.0, 0.0));
                        None
                    }
                }
            })));

        self.child(expanding_elem)
    }
}
