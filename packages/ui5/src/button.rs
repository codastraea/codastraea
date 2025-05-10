use silkenweb::{custom_html_element, elements::CustomEvent, parent_element, StrAttribute};
use strum::AsRefStr;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::icon;

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

parent_element!(button);

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

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute)]
pub enum AccessibleRole {
    Button,
    Link,
}

#[wasm_bindgen]
extern "C" {
    pub type ClickEvent;

    #[wasm_bindgen(method, getter = originalEvent, structural)]
    pub fn original(this: &ClickEvent) -> web_sys::Event;

    #[wasm_bindgen(method, getter = altKey, structural)]
    pub fn alt_key(this: &ClickEvent) -> bool;

    #[wasm_bindgen(method, getter = ctrlKey, structural)]
    pub fn ctrl_key(this: &ClickEvent) -> bool;

    #[wasm_bindgen(method, getter = metaKey, structural)]
    pub fn meta_key(this: &ClickEvent) -> bool;

    #[wasm_bindgen(method, getter = shiftKey, structural)]
    pub fn shift_key(this: &ClickEvent) -> bool;
}
