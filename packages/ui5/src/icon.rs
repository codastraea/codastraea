use silkenweb::{
    attribute::{AsAttribute, Attribute},
    custom_html_element, StrAttribute,
};
use strum::AsRefStr;

pub mod base;
pub mod business_suite;
pub mod tnt;

custom_html_element!(
    icon("ui5-icon") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            design: Design,
            name: Name,
            accessible_name: String,
            show_tooltip: String,
            mode: Mode,
        };
    }
);

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute)]
pub enum Design {
    Contrast,
    Critical,
    Default,
    Information,
    Negative,
    Neutral,
    NonInteractive,
    Positive,
}

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute)]
pub enum Mode {
    Image,
    Decorative,
    Interactive,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Name(&'static str);

impl Attribute for Name {
    type Text<'a> = &'static str;

    fn text(&self) -> Option<Self::Text<'_>> {
        Some(self.0)
    }
}

impl AsAttribute<Name> for Name {}
