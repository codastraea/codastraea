use silkenweb::{custom_html_element, element_slot, parent_element, StrAttribute, Value};
use strum::{AsRefStr, Display};

use crate::Size;

custom_html_element!(
    button("sl-button") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            variant: Variant,
            size: Size,
            caret: bool,
            pill: bool,
        };
    }
);

// TODO: What's allowed in this slot?
element_slot!(button, prefix, "prefix");
parent_element!(button);

#[derive(Copy, Clone, Eq, PartialEq, Display, AsRefStr, StrAttribute, Value)]
#[strum(serialize_all = "kebab-case")]
pub enum Variant {
    Default,
    Primary,
    Success,
    Neutral,
    Warning,
    Danger,
    Text,
}
