use silkenweb::custom_html_element;

custom_html_element!(
    sl_tab("sl-tab") = {
        dom_type: web_sys::HtmlElement;
        attributes {
            panel: String,
            active: bool,
            closable: bool,
            disabled: bool,
        };

        custom_events {
            // TODO: `custom-html-element` needs to support a text name in brackets
            sl_close: web_sys::CustomEvent,
        };
    }
);
