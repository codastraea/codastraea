mod elements {
    use silkenweb::{custom_html_element, parent_element};

    use super::Design;

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
                design: Design,
                selected: bool,
            };
        }
    );

    parent_element!(ui5_tab);
}

pub use elements::{
    ui5_tab as tab, ui5_tabcontainer as container, Ui5Tab as Tab, Ui5Tabcontainer as Container,
};
use silkenweb::attribute::{AsAttribute, Attribute};
use strum::AsRefStr;

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr)]
pub enum Design {
    Default,
    Positive,
    Negative,
    Critical,
    Neutral,
}

impl Attribute for Design {
    type Text<'a> = &'a str;

    fn text(&self) -> Option<Self::Text<'_>> {
        Some(self.as_ref())
    }
}

// TODO: Can we have a blanket `AsAttribute` in Silkenweb?
impl AsAttribute<Design> for Design {}
