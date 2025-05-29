use derive_more::Into;
use futures_signals::{
    signal::SignalExt,
    signal_vec::{MutableVec, MutableVecLockMut, SignalVecExt},
};
use gloo_console::info;
use serpent_automation_executor::syntax_tree::SrcSpan;
use serpent_automation_frontend::ServerConnection;
use serpent_automation_server_api::{NodeStatus, NodeUpdate, WatchCallTree};
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
        update: &NodeUpdate,
        actions: impl CallTreeActions,
    ) -> tree::CustomItem {
        let node = node_dropdown(update, actions.clone());
        // TODO: pass this around in `NodeUpdate`
        path.push(0);

        if update.has_children {
            let children = MutableVec::<NodeUpdate>::new();
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
                        spawn_local(server.watch(WatchCallTree::node(path.clone())).for_each({
                            clone!(children);
                            move |update| {
                                info!("Applying update");
                                MutableVecLockMut::apply_vec_diff(&mut children.lock_mut(), update);
                                async {}
                            }
                        }));
                    } else {
                        let mut children = children.lock_mut();
                        children.clear();
                    }
                }
            })
        } else {
            node
        }
    }
}

fn node_dropdown(node: &NodeUpdate, actions: impl CallTreeActions) -> tree::CustomItem {
    // TODO: `Design::Emphasized` for control flow nodes
    let design = Design::Default;
    let run_status = node.status;
    let icon = match run_status {
        NodeStatus::NotRun => icon::base::circle_task(),
        NodeStatus::Running => icon::base::busy(),
        NodeStatus::Complete => icon::base::sys_enter(),
        // TODO:
        // NodeStatus::PredicateSuccessful(false) => icon::base::circle_task_2(),
        // NodeStatus::Failed => icon::base::error(),
    };
    let badge = if run_status == NodeStatus::Running {
        Some(badge().design(BadgeDesign::AttentionDot))
    } else {
        None
    };

    let menu = menu::container().item_child(
        menu::item()
            .text("View code")
            .on_select(move || actions.view_code(SrcSpan::start())),
    );
    let button = button()
        .design(design)
        .text(&node.name)
        .icon(icon)
        .end_icon(icon::base::slim_arrow_down())
        .menu_opener(&menu)
        .badge_optional_child(badge);
    tree::custom_item()
        .content_child(button)
        .content_child(menu)
}
