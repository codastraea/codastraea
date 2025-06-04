use std::{cell::OnceCell, pin::pin, rc::Rc};

use codastraea_frontend::ServerConnection;
use codastraea_server_api::{
    CallTreeChildNodeId, NewNode, NodeStatus, NodeVecDiff, SrcSpan, WatchCallTree,
};
use derive_more::Into;
use futures::StreamExt;
use futures_signals::{
    signal::{Mutable, SignalExt},
    signal_vec::{MutableVec, SignalVecExt},
};
use silkenweb::{
    clone,
    node::{element::TextParentElement, Node},
    task::spawn_local,
    value::Sig,
    Value,
};
use silkenweb_signals_ext::SignalProduct;
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
    id: CallTreeChildNodeId,
    name: String,
    status: Mutable<NodeStatus>,
    has_children: Mutable<bool>,
}

impl NodeData {
    fn from_update(value: NewNode) -> Rc<Self> {
        Rc::new(Self {
            id: value.id,
            name: value.name,
            status: Mutable::new(value.status),
            has_children: Mutable::new(value.has_children),
        })
    }
}

impl CallTreeView {
    pub fn new(server: ServerConnection, actions: impl CallTreeActions) -> Self {
        let children = MutableVec::<Rc<NodeData>>::new();
        update_node_children(server.clone(), WatchCallTree::root(), children.clone());

        Self(
            tree::container()
                .compact_size(true)
                .item_children_signal(Self::node_children(server.clone(), actions, &children))
                .into(),
        )
    }

    fn node(
        server: &ServerConnection,
        data: &Rc<NodeData>,
        actions: impl CallTreeActions,
    ) -> tree::CustomItem {
        let node = node_contents(data, actions.clone());

        let once = OnceCell::new();
        let children = MutableVec::<Rc<NodeData>>::new();
        let children_empty = children.signal_vec_cloned().is_empty();
        let loading = (children_empty, data.has_children.signal())
            .signal_ref(|empty, has_children| *empty && *has_children);
        node.item_children_signal(Self::node_children(server.clone(), actions, &children))
            .item_optional_child(Sig(
                loading.map(move |loading| loading.then(|| tree::item().text("Loading...")))
            ))
            .on_toggle({
                let node_id = data.id;
                clone!(server);
                move |expanded| {
                    if expanded == Toggle::Expand {
                        once.get_or_init(|| {
                            update_node_children(
                                server.clone(),
                                WatchCallTree::node(node_id),
                                children.clone(),
                            );
                        });
                    }
                }
            })
    }

    fn node_children(
        server: ServerConnection,
        actions: impl CallTreeActions,
        children: &MutableVec<Rc<NodeData>>,
    ) -> futures_signals::signal_vec::Map<
        futures_signals::signal_vec::MutableSignalVec<Rc<NodeData>>,
        impl FnMut(Rc<NodeData>) -> tree::CustomItem,
    > {
        children
            .signal_vec_cloned()
            .map(move |c| Self::node(&server, &c, actions.clone()))
    }
}

fn update_node_children(
    server: ServerConnection,
    watch: WatchCallTree,
    children: MutableVec<Rc<NodeData>>,
) {
    // TODO: We need a way to cancel this. Put it in a Vec and cancel when we close
    // the call tree?
    spawn_local(async move {
        let mut updates = pin!(server.watch(watch).await);

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
                Diff::SetHasChildren { index } => children.lock_ref()[index].has_children.set(true),
            }
        }
    })
}

fn node_contents(node: &NodeData, actions: impl CallTreeActions) -> tree::CustomItem {
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
