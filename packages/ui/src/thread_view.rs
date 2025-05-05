use derive_more::Into;
use futures_signals::signal::Mutable;
use serpent_automation_executor::{syntax_tree::SrcSpan, CODE};
use serpent_automation_frontend::call_tree::CallTree;
use serpent_automation_shoelace::tab_group;
use silkenweb::{elements::html::div, node::Node, prelude::ParentElement, Value};
use silkenweb_bootstrap::{
    column,
    utility::{Overflow, SetDisplay, SetGap, SetOverflow, SetSpacing, Size::Size3},
};

use crate::{
    call_tree_view::{CallTreeActions, CallTreeView},
    source_view::{Editor, SourceView},
};

#[derive(Into, Value)]
pub struct ThreadView(Node);

impl ThreadView {
    pub fn new(call_tree: CallTree) -> Self {
        let active = Mutable::new(Tab::CallTree);
        let editor = Editor::new(CODE);
        let call_tree_view = CallTreeView::new(
            call_tree,
            Actions {
                active: active.clone(),
                editor: editor.clone(),
            },
        );

        Self(
            column()
                .overflow(Overflow::Hidden)
                .padding(Size3)
                .gap(Size3)
                .child(
                    tab_group::container()
                        .child(
                            "CallTree",
                            tab_group::nav().text("Call Tree"),
                            tab_group::panel()
                                .child(div().child(call_tree_view).overflow(Overflow::Auto)),
                        )
                        .child(
                            "SourceCode",
                            tab_group::nav().text("Source Code"),
                            tab_group::panel().child(
                                div()
                                    .child(SourceView::new(&editor))
                                    .flex_column()
                                    .overflow(Overflow::Hidden),
                            ),
                        ),
                )
                .into(),
        )
    }
}

#[derive(Clone)]
struct Actions {
    active: Mutable<Tab>,
    editor: Editor,
}

impl CallTreeActions for Actions {
    fn view_code(&self, span: SrcSpan) {
        self.editor.set_selection(span);
        self.active.set_neq(Tab::SourceCode);
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Tab {
    CallTree,
    SourceCode,
}
