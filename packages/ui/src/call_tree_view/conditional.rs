use std::sync::Arc;

use futures_signals::signal::Signal;
use serpent_automation_executor::{
    library::FunctionId,
    run::{CallStack, RunState, StackFrame},
    syntax_tree::{Body, Expression},
};
use serpent_automation_frontend::{expression_is_expandable, is_expandable};
use silkenweb::{clone, node::element::Element, prelude::ParentBuilder};
use silkenweb_bootstrap::{
    badge::badge,
    column,
    utility::{Align, Axis, Colour, SetAlign, SetBorder, SetDisplay, SetSpacing, Side, Size},
};

use super::{leaf_node, CallTreeState};
use crate::call_tree_view::{body_statements, expandable_node, expression, item};

pub(super) fn if_node(
    condition: Arc<Expression<FunctionId>>,
    then_block: Arc<Body<FunctionId>>,
    else_block: Arc<Body<FunctionId>>,
    call_stack: &CallStack,
    view_state: &CallTreeState,
) -> Element {
    // TODO: Make call stack cheap to clone.
    clone!(call_stack, view_state);
    let has_else = !else_block.is_empty();

    column()
        .align_items(Align::Start)
        .child(branch_body(
            Some(&condition),
            &then_block,
            0,
            &call_stack,
            &view_state,
        ))
        .optional_child(
            has_else.then(|| branch_body(None, &else_block, 1, &call_stack, &view_state)),
        )
        .into()
}

fn condition_node(
    condition: Option<&Arc<Expression<FunctionId>>>,
    block_index: usize,
    call_stack: &CallStack,
    view_state: &CallTreeState,
) -> Element {
    clone!(mut call_stack);
    call_stack.push(StackFrame::BlockPredicate(block_index));
    let run_state = view_state.run_state(&call_stack);

    if let Some(condition) = condition {
        // TODO: Condition text (maybe truncated), with tooltip (how does that work on
        // touch)
        if expression_is_expandable(condition) {
            let expanded = view_state.expanded(&call_stack);

            clone!(condition, call_stack, view_state);
            expandable_node(
                "if condition",
                CONDITION_COLOUR,
                run_state,
                expanded,
                move || {
                    column()
                        .border_on(Side::Start)
                        .border_colour(Colour::Secondary)
                        .align_items(Align::Start)
                        .padding_on_side((Size::Size3, Side::Start))
                        .children(expression(&condition, &call_stack, &view_state))
                },
            )
        } else {
            condition_leaf_node("if condition", run_state)
        }
    } else {
        condition_leaf_node("else", run_state)
    }
}

fn condition_leaf_node(name: &str, run_state: impl Signal<Item = RunState> + 'static) -> Element {
    leaf_node(name, CONDITION_COLOUR, run_state)
}

fn branch_body(
    condition: Option<&Arc<Expression<FunctionId>>>,
    body: &Arc<Body<FunctionId>>,
    nested_block_index: usize,
    call_stack: &CallStack,
    view_state: &CallTreeState,
) -> Element {
    let is_expandable = is_expandable(body);
    let condition = condition_node(condition, nested_block_index, call_stack, view_state);

    clone!(mut call_stack);
    call_stack.push(StackFrame::NestedBlock(nested_block_index));

    let mut body_elem = column()
        .border_on(Side::Start)
        .border_colour(Colour::Secondary)
        .align_items(Align::Start)
        .padding_on_side((Size::Size3, Side::Start));

    body_elem = if is_expandable {
        body_elem.children(body_statements(body.iter(), &call_stack, view_state))
    } else {
        body_elem.child(
            item(Colour::Secondary).child(
                badge("pass", Colour::Secondary)
                    .padding_on_axis((Size::Size5, Axis::X))
                    .padding_on_axis((Size::Size2, Axis::Y)),
            ),
        )
    };

    column()
        .align_self(Align::Stretch)
        .child(condition)
        .child(body_elem)
        .into()
}

const CONDITION_COLOUR: Colour = Colour::Info;
