use silkenweb::{custom_html_element, element_slot, prelude::html::Menu};

custom_html_element!(
    dropdown("sl-dropdown") = {
        dom_type: web_sys::HtmlElement;
    }
);

element_slot!(dropdown, menu, None::<String>, Menu);
