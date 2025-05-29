use derive_more::Into;
use futures_signals::signal_vec::SignalVecExt;
use serpent_automation_frontend::ServerConnection;
use serpent_automation_server_api::{NodeUpdate, WatchCallTree};
use silkenweb::{node::Node, Value};
use silkenweb_ui5::{tree, ComponentSize};

#[derive(Into, Value)]
pub struct CallTreeView(Node);

impl CallTreeView {
    pub fn new(server: ServerConnection) -> Self {
        Self(
            tree::container()
                .compact_size(true)
                .item_children_signal(server.watch(WatchCallTree::root()).map(child))
                .into(),
        )
    }
}

fn child(node: NodeUpdate) -> tree::Item {
    tree::item()
        .text(node.name)
        .item_optional_child(node.has_children.then(|| tree::item().text("Loading")))
}
