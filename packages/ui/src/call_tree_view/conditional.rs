use futures_signals::signal::SignalExt;
use serpent_automation_frontend::{
    call_tree::{Body, Call, If},
    tree::{Expandable, TreeNode},
};
use silkenweb::{clone, node::element::GenericElement, prelude::ParentElement};
use silkenweb_bootstrap::{
    button::{button, ButtonStyle},
    column,
    dropdown::{dropdown, dropdown_menu},
    utility::{Align, Axis, Colour, SetAlign, SetDisplay, SetSpacing, Size},
};

use super::{
    body_statements, call_node, dropdown_item, indented_block, internal_node, leaf_node,
    node_container, CallTreeActions, NodeData,
};

pub fn if_node(if_stmt: &If, actions: &impl CallTreeActions) -> GenericElement {
    column()
        .align_items(Align::Start)
        .child(branch_body(
            &NodeData::new(if_stmt.span(), "if", if_stmt.run_state()),
            if_stmt.condition(),
            if_stmt.then_block(),
            actions,
        ))
        .optional_child(if_stmt.else_block().as_ref().map(|else_block| {
            branch_body(
                &NodeData::new(else_block.span(), "else", else_block.run_state()),
                &TreeNode::Leaf,
                else_block.body(),
                actions,
            )
        }))
        .into()
}

fn branch_body(
    node: &NodeData,
    condition: &TreeNode<Expandable<Vec<Call>>>,
    body: &Body,
    actions: &impl CallTreeActions,
) -> GenericElement {
    let condition = condition_node(node, condition, actions);

    let body_elem = if !body.is_empty() {
        indented_block().children(body_statements(body.iter(), actions))
    } else {
        indented_block().child(
            node_container(Colour::Secondary).child(dropdown(
                button("button", "pass", ButtonStyle::Solid(Colour::Secondary))
                    .padding_on_axis((Size::Size4, Axis::X)),
                dropdown_menu().child(dropdown_item("View code")),
            )),
        )
    };

    column()
        .align_self(Align::Stretch)
        .child(condition)
        .child(body_elem)
        .into()
}

fn condition_node(
    node: &NodeData,
    condition: &TreeNode<Expandable<Vec<Call>>>,
    actions: &impl CallTreeActions,
) -> GenericElement {
    if let TreeNode::Internal(condition) = condition {
        // TODO: Condition text (maybe truncated), with tooltip (how does that work on
        // touch)
        internal_node(
            node,
            condition.is_expanded(),
            CONDITION_COLOUR,
            actions,
            condition.signal().map({
                clone!(actions);
                move |expandable_condition| {
                    expandable_condition.map({
                        clone!(actions);
                        move |condition| {
                            indented_block().children(condition.iter().map(|call| {
                                let actions = &actions;
                                call_node(&NodeData::from_call(call), call.body(), actions)
                            }))
                        }
                    })
                }
            }),
        )
    } else {
        leaf_node(node, CONDITION_COLOUR, actions)
    }
}

const CONDITION_COLOUR: Colour = Colour::Info;
