use silkenweb::{
    custom_html_element, elements::CustomEvent, prelude::ParentElement, value::RefSignalOrValue,
    StrAttribute,
};
use strum::AsRefStr;

use crate::{icon, AccessibleRole, ClickEvent};

custom_html_element!(
    link("ui5-link") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            disabled: bool,
            tooltip: String,
            href: String,
            target: Target,
            design: Design,
            interactive_area_size: InteractiveAreaSize,
            wrapping_type: WrappingType,
            accessible_name: String,
            accessible_name_ref: String,
            accessible_role: AccessibleRole,
            accessible_description: String,
            icon: icon::Name,
            end_icon: icon::Name,
        };

        events {
            click: CustomEvent<ClickEvent>
        };
    }
);

impl Link {
    pub fn text<'a, T>(self, child: impl RefSignalOrValue<'a, Item = T>) -> Self
    where
        T: 'a + AsRef<str> + Into<String>,
    {
        Self(self.0.text(child))
    }
}

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute)]
pub enum Design {
    Default,
    Subtle,
    Emphasized,
}

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute)]
pub enum InteractiveAreaSize {
    Normal,
    Large,
}

#[derive(Copy, Clone, Eq, PartialEq, StrAttribute)]
pub enum Target {
    SelfTarget,
    Top,
    Blank,
    Parent,
    Search,
}

impl AsRef<str> for Target {
    fn as_ref(&self) -> &str {
        match self {
            Self::SelfTarget => "_self",
            Self::Top => "_top",
            Self::Blank => "_blank",
            Self::Parent => "_parent",
            Self::Search => "_search",
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute)]
pub enum WrappingType {
    None,
    Normal,
}
