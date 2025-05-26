use silkenweb::{
    clone, custom_html_element, dom::Dom, element_slot, elements::ElementEvents,
    text_parent_element,
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

text_parent_element!(item);

impl<D: Dom> Item<D> {
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
