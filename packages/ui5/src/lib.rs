use silkenweb::StrAttribute;
use strum::AsRefStr;
use wasm_bindgen::prelude::wasm_bindgen;

pub mod button;
pub mod icon;
pub mod link;
pub mod menu;
pub mod tab;
pub mod tree;

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute)]
pub enum Highlight {
    None,
    Positive,
    Critical,
    Negative,
    Information,
}

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute)]
pub enum ItemType {
    Inactive,
    Active,
    Detail,
    Navigation,
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
