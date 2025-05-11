use silkenweb::{
    custom_html_element, element_slot, elements::CustomEvent, parent_element, StrAttribute,
};
use strum::AsRefStr;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::HtmlElement;

use crate::icon;

custom_html_element!(
    container("ui5-tabcontainer") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            collapsed: bool,
            tab_layout: Layout,
            overflow_mode: OverflowMode,
            header_background_design: BackgroundDesign,
            content_background_design: BackgroundDesign,
            no_auto_selection: bool,
        };

        events {
            tab_select: CustomEvent<TabSelectEvent>,
        };
    }
);

pub trait Child {}
impl Child for Content {}
impl Child for Separator {}

element_slot!(container, content, None::<String>, impl Child);

custom_html_element!(
    content("ui5-tab") = {
        dom_type: web_sys::HtmlElement;
        attributes {
            text: String,
            disabled: bool,
            additional_text: String,
            icon: icon::Name,
            design: Design,
            selected: bool,
        };
    }
);

parent_element!(content);
element_slot!(content, item, "items", impl Child);

custom_html_element!(
    separator("ui5-tab-separator") = {
        dom_type: web_sys::HtmlElement;
    }
);

#[wasm_bindgen]
extern "C" {
    pub type TabSelectEvent;

    #[wasm_bindgen(method, getter = tab, structural)]
    pub fn tab(this: &TabSelectEvent) -> HtmlElement;

    #[wasm_bindgen(method, getter = tabIndex, structural)]
    pub fn tab_index(this: &TabSelectEvent) -> usize;
}

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute)]
pub enum Layout {
    Inline,
    Standard,
}

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute)]
pub enum OverflowMode {
    End,
    StartAndEnd,
}

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute)]
pub enum BackgroundDesign {
    Solid,
    Transparent,
    Translucent,
}

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute)]
pub enum Design {
    Default,
    Positive,
    Negative,
    Critical,
    Neutral,
}
