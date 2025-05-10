use silkenweb::{
    custom_html_element,
    dom::Dom,
    elements::HtmlElement,
    node::{ChildNode, Node},
    parent_element,
    prelude::{ParentElement, SignalVec, SignalVecExt},
    value::SignalOrValue,
    StrAttribute,
};
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

// TODO: Divide this into silkenweb macros: `element_slot` and
// `element_multi_slot`. Add an optional trait for child elements.
impl<D: Dom> CustomItem<D> {
    pub fn content_child(
        self,
        child: impl SignalOrValue<Item = impl HtmlElement + ChildNode<D>>,
    ) -> Self {
        Self(self.0.child(child.map(|child| child.slot("content"))))
    }

    pub fn content_optional_child(
        self,
        child: impl SignalOrValue<Item = Option<impl HtmlElement + ChildNode<D>>>,
    ) -> Self {
        Self(
            self.0
                .optional_child(child.map(|child| child.map(|child| child.slot("content")))),
        )
    }

    pub fn content_children<N>(self, children: impl IntoIterator<Item = N>) -> Self
    where
        N: HtmlElement + Into<Node<D>>,
    {
        Self(
            self.0
                .children(children.into_iter().map(|child| child.slot("content"))),
        )
    }

    pub fn content_children_signal<N>(self, children: impl SignalVec<Item = N> + 'static) -> Self
    where
        N: HtmlElement + Into<Node<D>>,
    {
        Self(
            self.0
                .children_signal(children.map(|child| child.slot("content"))),
        )
    }
}

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
