use derive_more::Into;
use serpent_automation_executor::{syntax_tree::SrcSpan, CODE};
use serpent_automation_frontend::call_tree::CallTree;
use serpent_automation_shoelace::tab_group;
use silkenweb::{
    node::{element::Element, Node},
    prelude::ParentElement,
    Value,
};
use strum::AsRefStr;

use crate::{
    call_tree_view::{CallTreeActions, CallTreeView},
    css,
    source_view::{Editor, SourceView},
};

#[derive(Into, Value)]
pub struct ThreadView(Node);

impl ThreadView {
    pub fn new(call_tree: CallTree) -> Self {
        let editor = Editor::new(CODE);
        let tab_group = tab_group::container().class(css::FULL_HEIGHT);
        let call_tree_view = CallTreeView::new(
            call_tree,
            Actions {
                tab_control: tab_group.control(),
                editor: editor.clone(),
            },
        );

        Self(
            tab_group
                .child(
                    Tab::CallTree,
                    tab_group::nav().text("Call Tree"),
                    tab_group::panel().child(call_tree_view),
                )
                .child(
                    Tab::SourceCode,
                    tab_group::nav().text("Source Code"),
                    tab_group::panel().child(SourceView::new(&editor)),
                )
                .into(),
        )
    }
}

#[derive(Clone)]
struct Actions {
    tab_control: tab_group::Control,
    editor: Editor,
}

impl CallTreeActions for Actions {
    fn view_code(&self, span: SrcSpan) {
        self.editor.set_selection(span);
        self.tab_control.show(Tab::SourceCode);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr)]
enum Tab {
    CallTree,
    SourceCode,
}
