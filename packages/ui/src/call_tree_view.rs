use std::{cell::OnceCell, pin::pin, rc::Rc};

use derive_more::Into;
use futures::StreamExt;
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::{MutableVec, SignalVecExt},
};
use serpent_automation_frontend::ServerConnection;
use serpent_automation_server_api::{NewNode, NodeStatus, NodeVecDiff, SrcSpan, WatchCallTree};
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
    fn from_update(value: NewNode) -> Rc<Self> {
        Rc::new(Self {
            name: value.name,
            status: Mutable::new(value.status),
            has_children: value.has_children,
        })
    }
}

impl CallTreeView {
    pub fn new(server: ServerConnection, actions: impl CallTreeActions) -> Self {
        let children = MutableVec::<Rc<NodeData>>::new();
        let path = Vec::new();
        update_node_children(server.clone(), path.clone(), children.clone());

        Self(
            tree::container()
                .compact_size(true)
                .item_children_signal(Self::node_children(
                    server.clone(),
                    path,
                    actions,
                    &children,
                ))
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

        // TODO: `has_children` should be a signal
        let has_children = data.has_children;

        // This could be optimized by sending a flag to say if the node can ever have
        // children
        let once = OnceCell::new();
        let children = MutableVec::<Rc<NodeData>>::new();
        node.item_children_signal(Self::node_children(
            server.clone(),
            path.clone(),
            actions,
            &children,
        ))
        .item_optional_child(Sig(children.signal_vec_cloned().is_empty().map(
            move |loading| (has_children && loading).then(|| tree::item().text("Loading...")),
        )))
        .on_toggle({
            clone!(server);
            move |expanded| {
                if expanded == Toggle::Expand {
                    once.get_or_init(|| {
                        update_node_children(server.clone(), path.clone(), children.clone());
                    });
                }
            }
        })
    }

    fn node_children(
        server: ServerConnection,
        path: Vec<usize>,
        actions: impl CallTreeActions,
        children: &MutableVec<Rc<NodeData>>,
    ) -> futures_signals::signal_vec::Map<
        futures_signals::signal_vec::MutableSignalVec<Rc<NodeData>>,
        impl FnMut(Rc<NodeData>) -> tree::CustomItem,
    > {
        children
            .signal_vec_cloned()
            .map(move |c| Self::node(&server, path.clone(), &c, actions.clone()))
    }
}

fn update_node_children(
    server: ServerConnection,
    path: Vec<usize>,
    children: MutableVec<Rc<NodeData>>,
) {
    // TODO: We need a way to cancel this. Put it in a Vec and cancel when we close
    // the call tree?
    spawn_local(async move {
        let mut updates = pin!(server.watch(WatchCallTree::node(path.clone())).await);

        while let Some(update) = updates.next().await {
            use NodeVecDiff as Diff;
            match update {
                Diff::Replace(updates) => children
                    .lock_mut()
                    .replace_cloned(updates.into_iter().map(NodeData::from_update).collect()),
                Diff::Push(update) => children
                    .lock_mut()
                    .push_cloned(NodeData::from_update(update)),
                Diff::SetStatus { index, status } => children.lock_ref()[index].status.set(status),
            }
        }
    })
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
