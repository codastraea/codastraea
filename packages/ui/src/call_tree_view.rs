use std::{pin::pin, rc::Rc};

use derive_more::Into;
use futures::StreamExt;
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::{MutableVec, MutableVecLockMut, SignalVecExt},
};
use gloo_console::info;
use serpent_automation_frontend::ServerConnection;
use serpent_automation_server_api::{NodeStatus, NodeUpdate, NodeVecDiff, SrcSpan, WatchCallTree};
use silkenweb::{
    clone,
    node::{element::TextParentElement, Node},
    task::spawn_local,
    value::Sig,
    Value,
};
use silkenweb_ui5::{
    button::{badge, button, BadgeDesign, Design},
    icon, menu,
    tree::{self, Toggle},
    ComponentSize,
};

#[derive(Into, Value)]
pub struct CallTreeView(Node);

pub trait CallTreeActions: Clone + 'static {
    fn view_code(&self, span: SrcSpan);
}

struct NodeData {
    name: String,
    status: Mutable<NodeStatus>,
    has_children: bool,
}

impl NodeData {
    fn from_update(value: NodeUpdate) -> Rc<Self> {
        Rc::new(Self {
            name: value.name,
            status: Mutable::new(value.status),
            has_children: value.has_children,
        })
    }
}

impl CallTreeView {
    pub fn new(server: ServerConnection, actions: impl CallTreeActions) -> Self {
        Self(
            tree::container()
                .compact_size(true)
                .item_children_signal(
                    server
                        .watch(WatchCallTree::root())
                        .map(move |c| Self::node(&server, Vec::new(), &c, actions.clone())),
                )
                .into(),
        )
    }

    fn node(
        server: &ServerConnection,
        mut path: Vec<usize>,
        data: &Rc<NodeData>,
        actions: impl CallTreeActions,
    ) -> tree::CustomItem {
        let node = node_dropdown(data, actions.clone());
        // TODO: pass this around in `NodeData`, or use a node_id (slot map id) rather
        // than path?
        path.push(0);

        if data.has_children {
            let children = MutableVec::<Rc<NodeData>>::new();
            node.item_children_signal(children.signal_vec_cloned().map({
                clone!(server, path);
                move |c| Self::node(&server, path.clone(), &c, actions.clone())
            }))
            .item_optional_child(Sig(children
                .signal_vec_cloned()
                .is_empty()
                .map(|loading| loading.then(|| tree::item().text("Loading...")))))
            .on_toggle({
                clone!(server);
                move |expanded| {
                    if expanded == Toggle::Expand {
                        children.lock_mut().clear();

                        // TODO: We need a way to cancel this before we apply the next one.
                        clone!(server, path, children);
                        spawn_local(async move {
                            let mut updates =
                                pin!(server.watch(WatchCallTree::node(path.clone())).await);

                            while let Some(update) = updates.next().await {
                                use NodeVecDiff as Diff;
                                match update {
                                    Diff::Replace(updates) => children.lock_mut().replace_cloned(
                                        updates.into_iter().map(NodeData::from_update).collect(),
                                    ),
                                    Diff::Push(update) => children
                                        .lock_mut()
                                        .push_cloned(NodeData::from_update(update)),
                                    Diff::SetStatus { index, status } => {
                                        children.lock_ref()[index].status.set(status)
                                    }
                                }
                            }
                        })
                    } else {
                        children.lock_mut().clear();
                    }
                }
            })
        } else {
            node
        }
    }
}

fn node_dropdown(node: &NodeData, actions: impl CallTreeActions) -> tree::CustomItem {
    // TODO: `Design::Emphasized` for control flow nodes
    let design = Design::Default;
    let run_status = &node.status;
    let icon = run_status.signal().map(|run_status| match run_status {
        NodeStatus::NotRun => icon::base::circle_task(),
        NodeStatus::Running => icon::base::busy(),
        NodeStatus::Complete => icon::base::sys_enter(),
        // TODO:
        // NodeStatus::PredicateSuccessful(false) => icon::base::circle_task_2(),
        // NodeStatus::Failed => icon::base::error(),
    });
    let badge = run_status.signal().map(|run_status| {
        if run_status == NodeStatus::Running {
            Some(badge().design(BadgeDesign::AttentionDot))
        } else {
            None
        }
    });

    let menu = menu::container().item_child(
        menu::item()
            .text("View code")
            .on_select(move || actions.view_code(SrcSpan::start())),
    );
    let button = button()
        .design(design)
        .text(&node.name)
        .icon(Sig(icon))
        .end_icon(icon::base::slim_arrow_down())
        .menu_opener(&menu)
        .badge_optional_child(Sig(badge));
    tree::custom_item()
        .content_child(button)
        .content_child(menu)
}
