use silkenweb::{prelude::HtmlElement, value::RefSignalOrValue, StrAttribute, Value};
use strum::AsRefStr;
use wasm_bindgen::prelude::wasm_bindgen;

pub mod button;
pub mod icon;
pub mod link;
pub mod menu;
pub mod tab;
pub mod tree;

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute, Value)]
pub enum Highlight {
    None,
    Positive,
    Critical,
    Negative,
    Information,
}

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute, Value)]
pub enum ItemType {
    Inactive,
    Active,
    Detail,
    Navigation,
}

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute, Value)]
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

pub trait ComponentSize {
    fn compact_size<'a>(self, on: impl RefSignalOrValue<'a, Item = bool>) -> Self;
}

impl<T: HtmlElement> ComponentSize for T {
    fn compact_size<'a>(self, use_compact_size: impl RefSignalOrValue<'a, Item = bool>) -> Self {
        self.attribute("data-ui5-compact-size", use_compact_size)
    }
}
