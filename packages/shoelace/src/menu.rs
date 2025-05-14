use silkenweb::{
    clone, custom_html_element,
    dom::Dom,
    element_slot,
    prelude::{ElementEvents, ParentElement},
    value::RefSignalOrValue,
};

custom_html_element!(
    container("sl-menu") = {
        dom_type: web_sys::HtmlElement;
    }
);

pub trait Child {}
impl<D: Dom> Child for Item<D> {}

element_slot!(container, item, None::<String>, impl Child);

custom_html_element!(
    item("sl-menu-item") = {
        dom_type: web_sys::HtmlElement;
    }
);

// TODO: element_text_slot! macro.
impl<D: Dom> Item<D> {
    pub fn text<'a, T>(self, child: impl RefSignalOrValue<'a, Item = T>) -> Self
    where
        T: 'a + AsRef<str> + Into<String>,
    {
        Self(self.0.text(child))
    }

    pub fn on_select(self, mut handler: impl FnMut() + Clone + 'static) -> Self {
        self.on_click({
            clone!(mut handler);
            move |_, _| handler()
        })
        .on_keydown(move |ev, _| {
            if ev.key() == "Enter" {
                handler()
            }
        })
    }
}

element_slot!(item, item, None::<String>, impl Child);
