use silkenweb::{custom_html_element, StrAttribute, Value};
use strum::AsRefStr;

macro_rules! define_icons {
    ($collection:literal { $($name:ident = $str_name:literal),* $(,)? }) => {
        $(
            pub fn $name() -> $crate::icon::Name {
                $crate::icon::Name(concat!($collection, "/", $str_name))
            }
        )*
    }
}

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

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute, Value)]
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

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute, Value)]
pub enum Mode {
    Image,
    Decorative,
    Interactive,
}

#[derive(Copy, Clone, Eq, PartialEq, StrAttribute, Value)]
pub struct Name(&'static str);

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        self.0
    }
}
