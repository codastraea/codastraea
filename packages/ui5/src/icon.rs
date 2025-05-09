use silkenweb::{
    attribute::{AsAttribute, Attribute},
    StrAttribute,
};
use strum::AsRefStr;

pub mod base;
pub mod business_suite;
pub mod tnt;

mod elements {
    use silkenweb::custom_html_element;

    use super::{Design, Mode, Name};

    custom_html_element!(
        ui5_icon = {
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
}

pub use elements::{ui5_icon as element, Ui5Icon as Element};

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
