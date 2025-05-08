mod elements {
    use silkenweb::{custom_html_element, parent_element};

    custom_html_element!(
        ui5_tabcontainer = {
            dom_type: web_sys::HtmlElement;
        }
    );

    parent_element!(ui5_tabcontainer);

    custom_html_element!(
        ui5_tab = {
            dom_type: web_sys::HtmlElement;
            attributes {
                text: String,
                disabled: bool,
                additional_text: String,
                selected: bool,
            };
        }
    );

    parent_element!(ui5_tab);
}

pub use elements::{
    ui5_tab as tab, ui5_tabcontainer as container, Ui5Tab as Tab, Ui5Tabcontainer as Container,
};
