use silkenweb::{custom_html_element, element_slot, element_slot_single};

use crate::{button::Button, menu};

custom_html_element!(
    dropdown("sl-dropdown") = {
        dom_type: web_sys::HtmlElement;
    }
);

// TODO: Can anything else be used as a trigger?
element_slot!(dropdown, trigger, "trigger", Button);
element_slot_single!(dropdown, menu, None::<String>, menu::Container);
