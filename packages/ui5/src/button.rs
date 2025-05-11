use silkenweb::{
    custom_html_element,
    dom::Dom,
    elements::CustomEvent,
    prelude::{Element, ParentElement},
    value::RefSignalOrValue,
    StrAttribute,
};
use strum::AsRefStr;
use wasm_bindgen::UnwrapThrowExt;

use crate::{icon, menu, AccessibleRole, ClickEvent};

custom_html_element!(
    button("ui5-button") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            design: Design,
            disabled: bool,
            icon: icon::Name,
            end_icon: icon::Name,
            submits: bool,
            tooltip: String,
            accessible_name: String,
            accessible_name_ref: String,
            accessible_description: String,
            r#type: Type,
            accessible_role: AccessibleRole,
        };

        events {
            click: CustomEvent<ClickEvent>
        };
    }
);

impl<D: Dom> Button<D> {
    pub fn text<'a, T>(self, child: impl RefSignalOrValue<'a, Item = T>) -> Self
    where
        T: 'a + AsRef<str> + Into<String>,
    {
        Self(self.0.text(child))
    }

    pub fn toggle_on_click(self, menu: &menu::Container<D>) -> Self {
        menu.set_opener(&self.handle().dom_element());
        let dom_menu = menu.handle().dom_element();
        let open_attr = "open";

        self.on_click(move |_, _| {
            if dom_menu.has_attribute(open_attr) {
                dom_menu.remove_attribute(open_attr).unwrap_throw();
            } else {
                dom_menu.set_attribute(open_attr, "").unwrap_throw();
            }
        })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute)]
pub enum Design {
    Default,
    Positive,
    Negative,
    Transparent,
    Emphasized,
    Attention,
}

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute)]
pub enum Type {
    Button,
    Submit,
    Reset,
}
