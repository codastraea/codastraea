use silkenweb::{custom_html_element, parent_element};

custom_html_element!(
    container("sl-tree") = {
        dom_type: web_sys::HtmlElement;
    }
);

parent_element!(container);

custom_html_element!(
    item("sl-tree-item") = {
        dom_type: web_sys::HtmlElement;
    }
);

parent_element!(item);
