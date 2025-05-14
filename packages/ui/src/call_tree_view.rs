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
    prelude::{Element, ParentElement},
    value::Sig,
    Value,
};
use silkenweb_shoelace::{
    button::{button, Variant},
    dropdown::dropdown,
    icon, menu, tree, Size,
};

use self::conditional::if_node;

mod conditional;

css_module!("call-tree");

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
                .class(class::call_tree())
                .child(call_node(&node_data, call_tree.body(), &actions))
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
) -> tree::Item {
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
    loadable_body: impl Signal<Item = Option<Vec<tree::Item>>> + 'static,
) -> tree::Item {
    let body = loadable_body
        .map(|body| body.unwrap_or(vec![tree::item().text("Loading...")]))
        .to_signal_vec();
    // TODO: Do this when a tree item is expanded. Need to watch the `expanded`
    // attribute.
    is_expanded.set(true);
    node_dropdown(node, node_type, actions).children_signal(body)
}

fn leaf_node(node: &NodeData, node_type: NodeType, actions: &impl CallTreeActions) -> tree::Item {
    node_dropdown(node, node_type, actions)
}

fn node_dropdown(
    node: &NodeData,
    node_type: NodeType,
    actions: &impl CallTreeActions,
) -> tree::Item {
    let variant = match node_type {
        NodeType::Function => Variant::Default,
        NodeType::Condition => Variant::Primary,
    };
    let run_state = &node.run_state;
    let icon = run_state.signal().map(|run_state| {
        match run_state {
            RunState::NotRun => icon::default::circle(),
            RunState::Running => icon::default::play_circle(),
            RunState::Successful | RunState::PredicateSuccessful(true) => {
                icon::default::check_circle()
            }
            RunState::PredicateSuccessful(false) => icon::default::dash_circle_dotted(),
            RunState::Failed => icon::default::exclamation_circle(),
        }
        .icon()
    });

    let menu = menu::container().item_child(menu::item().text("View code").on_select({
        clone!(actions);
        let span = node.span;
        move || actions.view_code(span)
    }));
    tree::item().child(
        dropdown()
            .trigger_child(
                button()
                    .variant(variant)
                    .pill(true)
                    .caret(true)
                    .text(node.name)
                    .prefix_child(Sig(icon))
                    .size(Size::Small),
            )
            .menu_child(menu),
    )
}

fn body_statements<'a>(
    stmts: impl Iterator<Item = &'a Statement> + 'a,
    actions: &'a impl CallTreeActions,
) -> impl Iterator<Item = tree::Item> + 'a {
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
