mod elements {
    use silkenweb::{custom_html_element, elements::CustomEvent, parent_element};

    use super::{BackgroundDesign, Design, Layout, OverflowMode, TabSelectEvent};
    use crate::icon;

    custom_html_element!(
        ui5_tabcontainer = {
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

    parent_element!(ui5_tabcontainer);

    custom_html_element!(
        ui5_tab = {
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

    parent_element!(ui5_tab);

    custom_html_element!(
        ui5_tab_separator = {
            dom_type: web_sys::HtmlElement;
        }
    );
}

pub use elements::{
    ui5_tab as tab, ui5_tab_separator as separator, ui5_tabcontainer as container, Ui5Tab as Tab,
    Ui5TabSeparator as Separator, Ui5Tabcontainer as Container,
};
use silkenweb::StrAttribute;
use strum::AsRefStr;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::HtmlElement;

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
