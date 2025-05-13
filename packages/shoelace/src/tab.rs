use silkenweb::{
    custom_html_element,
    dom::{DefaultDom, Dom},
    node::element::ElementHandle,
    parent_element,
    prelude::{Element, HtmlElement, ParentElement},
    StrAttribute, Value,
};
use strum::{AsRefStr, Display};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast};

use crate::Edge;

custom_html_element!(
    group("sl-tab-group") = {
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

impl<D: Dom> Group<D> {
    pub fn child(self, name: impl AsRef<str>, tab: Header<D>, panel: Panel<D>) -> Self {
        let name = name.as_ref();
        Self(
            self.0
                .child(tab.attribute("panel", name).slot("nav"))
                .child(panel.attribute("name", name).slot(None::<String>)),
        )
    }

    pub fn control(&self) -> Control<D> {
        Control(self.handle())
    }
}

custom_html_element!(
    header("sl-tab") = {
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

parent_element!(header);

custom_html_element!(
    panel("sl-tab-panel") = {
        dom_type: web_sys::HtmlElement;
    }
);

parent_element!(panel);

#[derive(Display, AsRefStr, StrAttribute, Value)]
#[strum(serialize_all = "kebab-case")]
pub enum Activation {
    Auto,
    Manual,
}
