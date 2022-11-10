use std::rc::Rc;

use derive_more::Into;
use futures_signals::signal::{Mutable, SignalExt};
use serpent_automation_executor::library::{FunctionId, Library};
use silkenweb::{
    clone,
    elements::html::{self, div},
    node::Node,
    prelude::{ElementEvents, ParentBuilder},
    value::Sig,
    Value,
};
use silkenweb_bootstrap::{
    column,
    tab_bar::{tab_bar, Style},
    utility::{Active, Display, SetDisplay},
};

use crate::{call_tree_view::CallTree, ViewCallStates};

#[derive(Into, Value)]
pub struct ThreadView(Node);

impl ThreadView {
    pub fn new(
        fn_id: FunctionId,
        library: &Rc<Library>,
        view_call_states: &ViewCallStates,
    ) -> Self {
        let active = Mutable::new(Tab::CallTree);

        Self(
            column()
                .child(tab_bar().style(Style::Tabs).children([
                    tab(Tab::CallTree, "Call Tree", &active),
                    tab(Tab::SourceCode, "Source Code", &active),
                ]))
                .child(content(
                    Tab::CallTree,
                    &active,
                    CallTree::new(fn_id, library, view_call_states),
                ))
                .into(),
        )
    }
}

fn tab(tab: Tab, name: &str, active: &Mutable<Tab>) -> html::ButtonBuilder {
    html::button()
        .text(name)
        .active(Sig(active.signal().eq(tab)))
        .on_click({
            clone!(active);
            move |_, _| active.set(tab)
        })
}

fn content(tab: Tab, active: &Mutable<Tab>, content: impl Into<Node>) -> Node {
    div()
        .display(Sig(active.signal().map(move |active| {
            if active == tab {
                Display::Block
            } else {
                Display::None
            }
        })))
        .child(content.into())
        .into()
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Tab {
    CallTree,
    SourceCode,
}
