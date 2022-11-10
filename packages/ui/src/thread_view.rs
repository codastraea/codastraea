use std::rc::Rc;

use derive_more::Into;
use futures_signals::signal::{Mutable, SignalExt};
use serpent_automation_executor::library::{FunctionId, Library};
use silkenweb::{
    clone,
    elements::html,
    node::Node,
    prelude::{ElementEvents, ParentBuilder},
    value::Sig,
    Value,
};
use silkenweb_bootstrap::{
    column,
    tab_bar::{tab_bar, Style},
    utility::Active,
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
                .child(
                    tab_bar().style(Style::Tabs).children([
                        html::button()
                            .text("Call Tree")
                            .active(Sig(active.signal().eq(Tab::CallTree)))
                            .on_click({
                                clone!(active);
                                move |_, _| active.set(Tab::CallTree)
                            }),
                        html::button()
                            .text("Source Code")
                            .active(Sig(active.signal().eq(Tab::SourceCode)))
                            .on_click({
                                clone!(active);
                                move |_, _| active.set(Tab::SourceCode)
                            }),
                    ]),
                )
                .child(CallTree::new(fn_id, library, view_call_states))
                .into(),
        )
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Tab {
    CallTree,
    SourceCode,
}
