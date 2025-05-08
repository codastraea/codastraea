use silkenweb::attribute::{AsAttribute, Attribute};
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

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr)]
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

impl Attribute for Design {
    type Text<'a> = &'a str;

    fn text(&self) -> Option<Self::Text<'_>> {
        Some(self.as_ref())
    }
}

impl AsAttribute<Design> for Design {}

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr)]
pub enum Mode {
    Image,
    Decorative,
    Interactive,
}

impl Attribute for Mode {
    type Text<'a> = &'a str;

    fn text(&self) -> Option<Self::Text<'_>> {
        Some(self.as_ref())
    }
}

impl AsAttribute<Mode> for Mode {}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Name(&'static str);

impl Attribute for Name {
    type Text<'a> = &'static str;

    fn text(&self) -> Option<Self::Text<'_>> {
        Some(self.0)
    }
}

impl AsAttribute<Name> for Name {}
