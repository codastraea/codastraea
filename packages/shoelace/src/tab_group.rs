use silkenweb::attribute::{AsAttribute, Attribute};
use strum::{Display, IntoStaticStr};

mod elements {
    use silkenweb::{
        custom_html_element,
        dom::{DefaultDom, Dom},
        node::element::ElementHandle,
        parent_element,
        prelude::{Element, HtmlElement, ParentElement},
    };
    use wasm_bindgen::{prelude::wasm_bindgen, JsCast};

    use super::Activation;
    use crate::Edge;

    custom_html_element!(
        sl_tab_group = {
            dom_type: web_sys::HtmlElement;
            attributes {
                placement: Edge,
                activation: Activation,
                no_scroll_controls: bool,
                fixed_scroll_controls: bool,
            };

            events {
                sl_tab_show: web_sys::CustomEvent,
                sl_tab_hide: web_sys::CustomEvent,
            };
        }
    );

    pub struct Control<D: Dom = DefaultDom>(ElementHandle<D, web_sys::HtmlElement>);

    impl<D: Dom> Clone for Control<D> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    #[wasm_bindgen]
    extern "C" {
        type TabGroup;

        #[wasm_bindgen(method)]
        fn show(this: &TabGroup, name: &str);
    }

    impl<D: Dom> Control<D> {
        pub fn show(&self, name: impl AsRef<str>) {
            self.0
                .dom_element()
                .unchecked_ref::<TabGroup>()
                .show(name.as_ref())
        }
    }

    impl<D: Dom> SlTabGroup<D> {
        pub fn child(self, name: impl AsRef<str>, tab: SlTab<D>, panel: SlTabPanel<D>) -> Self {
            let name = name.as_ref();
            Self(
                self.0
                    .child(tab.attribute("panel", name).slot("nav"))
                    .child(panel.attribute("name", name).slot(None as Option<String>)),
            )
        }

        pub fn control(&self) -> Control<D> {
            Control(self.handle())
        }
    }

    custom_html_element!(
        sl_tab = {
            dom_type: web_sys::HtmlElement;
            attributes {
                closable: bool,
                disabled: bool,
            };

            events {
                sl_close: web_sys::CustomEvent,
            };
        }
    );

    parent_element!(sl_tab);

    custom_html_element!(
        sl_tab_panel = {
            dom_type: web_sys::HtmlElement;
        }
    );

    parent_element!(sl_tab_panel);
}

pub use elements::{
    sl_tab as nav, sl_tab_group as container, sl_tab_panel as panel, Control, SlTab as Nav,
    SlTabGroup as Container, SlTabPanel as Panel,
};

#[derive(Display, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum Activation {
    Auto,
    Manual,
}

impl Attribute for Activation {
    type Text<'a> = &'a str;

    fn text(&self) -> Option<Self::Text<'_>> {
        Some(self.into())
    }
}

impl AsAttribute<Activation> for Activation {}
