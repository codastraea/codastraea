use silkenweb::{
    custom_html_element, elements::CustomEvent, prelude::ParentElement, value::RefSignalOrValue,
    StrAttribute,
};
use strum::AsRefStr;

use crate::{icon, AccessibleRole, ClickEvent};

custom_html_element!(
    button("ui5-button") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            design: Design,
            disabled: bool,
            icon: icon::Name,
            end_icon: icon::Name,
            submits: bool,
            tooltip: String,
            accessible_name: String,
            accessible_name_ref: String,
            accessible_description: String,
            r#type: Type,
            accessible_role: AccessibleRole,
        };

        events {
            click: CustomEvent<ClickEvent>
        };
    }
);

impl Button {
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
    Positive,
    Negative,
    Transparent,
    Emphasized,
    Attention,
}

#[derive(Copy, Clone, Eq, PartialEq, AsRefStr, StrAttribute)]
pub enum Type {
    Button,
    Submit,
    Reset,
}
