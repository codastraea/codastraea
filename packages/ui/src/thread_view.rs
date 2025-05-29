use derive_more::Into;
use futures_signals::signal::{Mutable, SignalExt};
use serpent_automation_frontend::ServerConnection;
use serpent_automation_server_api::SrcSpan;
use silkenweb::{
    node::{
        element::{Element, ParentElement},
        Node,
    },
    value::Sig,
    Value,
};
use silkenweb_ui5::tab;
use strum::AsRefStr;

use crate::{
    call_tree_view::{CallTreeActions, CallTreeView},
    css,
    source_view::{Editor, SourceView},
};

#[derive(Into, Value)]
pub struct ThreadView(Node);

impl ThreadView {
    pub fn new(server: ServerConnection) -> Self {
        let editor = Editor::new("Some code");
        let tab_group = tab::container().class(css::full_height());
        let selected_tab = Mutable::new(Tab::CallTree);
        let actions = Actions {
            selected_tab: selected_tab.clone(),
            editor: editor.clone(),
        };
        let call_tree_view = CallTreeView::new(server, actions);
        let tab = |tab: Tab| {
            tab::content().text(tab.as_ref()).selected(Sig(selected_tab
                .signal()
                .map(move |selected| selected == tab)))
        };

        Self(
            tab_group
                .content_children([
                    tab(Tab::CallTree).child(call_tree_view),
                    tab(Tab::SourceCode).child(SourceView::new(&editor)),
                ])
                .into(),
        )
    }
}

#[derive(Clone)]
struct Actions {
    selected_tab: Mutable<Tab>,
    editor: Editor,
}

impl CallTreeActions for Actions {
    fn view_code(&self, span: SrcSpan) {
        self.editor.set_selection(span);
        self.selected_tab.set(Tab::SourceCode);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, AsRefStr)]
enum Tab {
    CallTree,
    SourceCode,
}
