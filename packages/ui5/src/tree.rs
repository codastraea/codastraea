use silkenweb::{custom_html_element, parent_element, StrAttribute};
use strum::AsRefStr;

use crate::{icon, ItemType, TextState};

custom_html_element!(
    container("ui5-tree") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            selection_mode: SelectionMode,
            no_data_text: String,
            header_text: String,
            footer_text: String,
            accessible_name: String,
            accessible_name_ref: String,
            accessible_description: String,
            accessible_description_ref: String,
        };
    }
);

parent_element!(container);

custom_html_element!(
    item("ui5-tree-item") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            text: String,
            additional_text: String,
            icon: icon::Name,
            expanded: bool,
            movable: bool,
            indeterminate: bool,
            has_children: bool,
            additional_text_state: TextState,
            accessible_name: String,
            r#type: ItemType,
            navigated: bool,
            tooltip: String,
            highlight: TextState,
            selected: bool,
        };

        events {
            detail_click: web_sys::CustomEvent,
        };
    }
);

parent_element!(item);

custom_html_element!(
    custom_item("ui5-tree-item-custom") = {
        dom_type: web_sys::HtmlElement;

        attributes {
            hide_selection_element: bool,
            icon: icon::Name,
            expanded: bool,
            movable: bool,
            indeterminate: bool,
            has_children: bool,
            additional_text_state: TextState,
            accessible_name: String,
            r#type: ItemType,
            navigated: bool,
            tooltip: String,
            highlight: TextState,
            selected: bool,
        };

        events {
            detail_click: web_sys::CustomEvent,
        };
    }
);

parent_element!(custom_item);

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr, StrAttribute)]
pub enum SelectionMode {
    None,
    Single,
    SingleStart,
    SingleEnd,
    SingleAuto,
    Multiple,
    Delete,
}
