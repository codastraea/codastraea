use silkenweb::{custom_html_element, element_slot, element_slot_single, StrAttribute};
use strum::AsRefStr;

use crate::{button::Button, icon, ItemType, Highlight};

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

pub trait Child {}
impl Child for Item {}
impl Child for CustomItem {}

element_slot!(container, item, None::<String>, impl Child);
element_slot!(container, header, "header");

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
            additional_text_state: Highlight,
            accessible_name: String,
            r#type: ItemType,
            navigated: bool,
            tooltip: String,
            highlight: Highlight,
            selected: bool,
        };

        events {
            detail_click: web_sys::CustomEvent,
        };
    }
);

element_slot!(item, item, None::<String>, impl Child);
element_slot_single!(item, delete_button, "deleteButton", Button);

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
            additional_text_state: Highlight,
            accessible_name: String,
            r#type: ItemType,
            navigated: bool,
            tooltip: String,
            highlight: Highlight,
            selected: bool,
        };

        events {
            detail_click: web_sys::CustomEvent,
        };
    }
);

element_slot!(custom_item, item, None::<String>, impl Child);
element_slot!(custom_item, content, "content");
element_slot_single!(custom_item, delete_button, "deleteButton", Button);

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
