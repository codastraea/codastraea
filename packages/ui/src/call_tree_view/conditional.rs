use futures_signals::signal::SignalExt;
use serpent_automation_executor::{run::RunState, syntax_tree::SrcSpan};
use serpent_automation_frontend::{
    call_tree::{Body, Call, If},
    tree::{Expandable, TreeNode},
};
use silkenweb::{
    clone,
    prelude::{html::span, Mutable, TextParentElement},
};
use silkenweb_ui5::tree;

use super::{
    body_statements, call_node, internal_node, leaf_node, node_dropdown, CallTreeActions, NodeData,
    NodeType,
};

pub fn if_node(if_stmt: &If, actions: &impl CallTreeActions) -> Vec<tree::CustomItem> {
    let mut items = vec![node_dropdown(
        &NodeData::new(if_stmt.span(), "if", if_stmt.run_state()),
        NodeType::Condition,
        actions,
    )
    .item_child(condition_node(if_stmt.condition(), if_stmt.span(), actions))
    .item_child(
        tree::custom_item()
            .content_child(span().text("then"))
            .item_children(body_statements(if_stmt.then_block().iter(), actions)),
    )];

    if let Some(else_block) = if_stmt.else_block() {
        items.push(else_body(
            &NodeData::new(else_block.span(), "else", else_block.run_state()),
            else_block.body(),
            actions,
        ))
    }

    items
}

fn else_body(node: &NodeData, body: &Body, actions: &impl CallTreeActions) -> tree::CustomItem {
    node_dropdown(node, NodeType::Condition, actions)
        .item_children(body_statements(body.iter(), actions))
}

fn condition_node(
    condition: &TreeNode<Expandable<Vec<Call>>>,
    span: SrcSpan,
    actions: &impl CallTreeActions,
) -> tree::CustomItem {
    // TODO: Run state for conditions
    let run_state = Mutable::new(RunState::NotRun).read_only();
    let node = &NodeData::new(span, "condition", run_state);

    if let TreeNode::Internal(condition) = condition {
        internal_node(
            node,
            condition.is_expanded(),
            NodeType::Function,
            actions,
            condition.signal().map({
                clone!(actions);
                move |loadable_condition| {
                    loadable_condition.map(|condition| {
                        condition
                            .iter()
                            .map(|call| {
                                call_node(&NodeData::from_call(call), call.body(), &actions)
                            })
                            .collect()
                    })
                }
            }),
        )
    } else {
        leaf_node(node, NodeType::Function, actions)
    }
}
