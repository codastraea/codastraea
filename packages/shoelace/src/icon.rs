use silkenweb::{custom_html_element, StrAttribute, Value};
use strum::Display;

macro_rules! define_icons {
    ($library:expr => { $($name:ident = $str_name:literal),* $(,)?}) => {
        $(
            pub fn $name() -> Name {
                Name { library: $library, name: $str_name }
            }
        )*
    }
}

pub mod default;

custom_html_element!(
    icon("sl-icon") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            name: String,
            src: String,
            label: String,
            library: String,
        };

        events {
            sl_load : web_sys::CustomEvent,
            sl_error : web_sys::CustomEvent,
        };
    }
);

custom_html_element!(
    button("sl-icon-button") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            name: String,
            library: String,
            src: String,
            href: String,
            target: Target,
            download: String,
            label: String,
            disabled: bool,
        };

        events {
            sl_blur: web_sys::CustomEvent,
            sl_focus: web_sys::CustomEvent,
        };
    }
);

#[derive(Copy, Clone, Eq, PartialEq, Display, StrAttribute, Value)]
pub enum Target {
    Blank,
    Parent,
    Self_,
    Top,
}

impl AsRef<str> for Target {
    fn as_ref(&self) -> &str {
        match self {
            Target::Blank => "_blank",
            Target::Parent => "_parent",
            Target::Self_ => "_self",
            Target::Top => "_top",
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Name {
    name: &'static str,
    library: Option<&'static str>,
}

impl Name {
    pub fn icon(self) -> Icon {
        Icon::new().name(self.name).library(self.library)
    }

    pub fn button(self) -> Button {
        Button::new().name(self.name).library(self.library)
    }
}
