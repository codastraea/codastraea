use derive_more::Into;
use futures_signals::signal::{Mutable, ReadOnlyMutable, Signal, SignalExt};
use serpent_automation_executor::{run::RunState, syntax_tree::SrcSpan};
use serpent_automation_frontend::{
    call_tree::{Body, Call, CallTree, Statement},
    tree::{Expandable, TreeNode},
};
use silkenweb::{
    clone,
    node::Node,
    prelude::{html::div, ElementEvents, ParentElement},
    value::Sig,
    Value,
};
use silkenweb_ui5::{
    button::{button, Design},
    icon, menu, tree,
};

use self::conditional::if_node;

mod conditional;

#[derive(Into, Value)]
pub struct CallTreeView(Node);

impl CallTreeView {
    pub fn new(call_tree: CallTree, actions: impl CallTreeActions) -> Self {
        // TODO: Handle uwnrap failure (python functions can't be run directly).
        let node_data = NodeData::new(
            call_tree.span().unwrap(),
            call_tree.name(),
            call_tree.run_state(),
        );

        Self(
            tree::container()
                .item_child(call_node(&node_data, call_tree.body(), &actions))
                .into(),
        )
    }
}

struct NodeData<'a> {
    span: SrcSpan,
    name: &'a str,
    run_state: ReadOnlyMutable<RunState>,
}

impl<'a> NodeData<'a> {
    fn new(span: SrcSpan, name: &'a str, run_state: ReadOnlyMutable<RunState>) -> Self {
        Self {
            span,
            name,
            run_state,
        }
    }

    fn from_call(call: &'a Call) -> Self {
        Self {
            span: call.span(),
            name: call.name(),
            run_state: call.run_state(),
        }
    }
}

fn call_node(
    node: &NodeData,
    body: &TreeNode<Expandable<Body>>,
    actions: &impl CallTreeActions,
) -> tree::CustomItem {
    if let TreeNode::Internal(body) = body {
        internal_node(
            node,
            body.is_expanded(),
            NodeType::Function,
            actions,
            body.signal().map({
                clone!(actions);
                move |expandable_body| {
                    expandable_body.map({
                        clone!(actions);
                        move |body| body_statements(body.iter(), &actions).collect()
                    })
                }
            }),
        )
    } else {
        leaf_node(node, NodeType::Function, actions)
    }
}

fn internal_node(
    node: &NodeData,
    is_expanded: &Mutable<bool>,
    node_type: NodeType,
    actions: &impl CallTreeActions,
    loadable_body: impl Signal<Item = Option<Vec<tree::CustomItem>>> + 'static,
) -> tree::CustomItem {
    let body = loadable_body
        .map(|body| {
            body.unwrap_or(vec![
                tree::custom_item().content_child(div().text("Loading..."))
            ])
        })
        .to_signal_vec();
    // TODO: Do this when a tree item is expanded. Need to watch the `expanded`
    // attribute.
    is_expanded.set(true);
    node_dropdown(node, node_type, actions).item_children_signal(body)
}

fn leaf_node(
    node: &NodeData,
    node_type: NodeType,
    actions: &impl CallTreeActions,
) -> tree::CustomItem {
    node_dropdown(node, node_type, actions)
}

fn node_dropdown(
    node: &NodeData,
    node_type: NodeType,
    actions: &impl CallTreeActions,
) -> tree::CustomItem {
    let design = match node_type {
        NodeType::Function => Design::Default,
        NodeType::Condition => Design::Emphasized,
    };
    let run_state = node.run_state.signal().map(|run_state| match run_state {
        RunState::NotRun => icon::base::circle_task(),
        RunState::Running => icon::base::busy(),
        RunState::Successful | RunState::PredicateSuccessful(true) => icon::base::sys_enter(),
        RunState::PredicateSuccessful(false) => icon::base::circle_task_2(),
        RunState::Failed => icon::base::error(),
    });

    let menu = menu::container().item_child(menu::item().text("View code").on_click({
        clone!(actions);
        let span = node.span;
        move |_, _| actions.view_code(span)
    }));
    let button = button()
        .design(design)
        .text(node.name)
        .end_icon(icon::base::slim_arrow_down())
        .menu_opener(&menu);
    tree::custom_item()
        .content_child(button)
        .content_child(icon::icon().name(Sig(run_state)))
        .content_child(menu)
}

fn body_statements<'a>(
    stmts: impl Iterator<Item = &'a Statement> + 'a,
    actions: &'a impl CallTreeActions,
) -> impl Iterator<Item = tree::CustomItem> + 'a {
    stmts.flat_map(|stmt| match stmt {
        Statement::Call(call) => vec![call_node(&NodeData::from_call(call), call.body(), actions)],
        Statement::If(if_stmt) => if_node(if_stmt, actions),
    })
}

pub trait CallTreeActions: Clone + 'static {
    fn view_code(&self, span: SrcSpan);
}

enum NodeType {
    Function,
    Condition,
}
