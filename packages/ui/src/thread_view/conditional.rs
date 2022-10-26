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
    button::ButtonStyle,
    column, row,
    utility::{Align, Colour, SetFlex, SetGap, Size::Size3},
};

use super::{leaf_node, ThreadViewState};
use crate::thread_view::{body_statements, expandable_node, expression};

pub(super) fn if_node(
    condition: Arc<Expression<FunctionId>>,
    then_block: Arc<Body<FunctionId>>,
    else_block: Arc<Body<FunctionId>>,
    is_last: bool,
    call_stack: &CallStack,
    view_state: &ThreadViewState,
) -> Element {
    let expanded = view_state.expanded(call_stack);
    let run_state = view_state.run_state(call_stack);

    // TODO: Make call stack cheap to clone.
    clone!(call_stack, view_state);
    let has_else = !else_block.is_empty();

    expandable_node(
        "If",
        CONDITION_STYLE,
        run_state,
        is_last,
        expanded,
        move || {
            column()
                .align_items(Align::Start)
                .gap(Size3)
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
        },
    )
}

fn condition_node(
    condition: Option<&Arc<Expression<FunctionId>>>,
    block_index: usize,
    is_last: bool,
    call_stack: &CallStack,
    view_state: &ThreadViewState,
) -> Element {
    clone!(mut call_stack);
    call_stack.push(StackFrame::BlockPredicate(block_index));
    let run_state = view_state.run_state(&call_stack);

    if let Some(condition) = condition {
        if expression_is_expandable(condition) {
            let expanded = view_state.expanded(&call_stack);

            clone!(condition, call_stack, view_state);
            expandable_node(
                "condition",
                CONDITION_STYLE,
                run_state,
                is_last,
                expanded,
                move || {
                    row().align_items(Align::Stretch).children(expression(
                        &condition,
                        true,
                        &call_stack,
                        &view_state,
                    ))
                },
            )
        } else {
            // TODO: Condition text (maybe truncated), with tooltip (how does that work on
            // touch)
            condition_leaf_node("condition", is_last, run_state)
        }
    } else {
        condition_leaf_node("else", is_last, run_state)
    }
}

fn condition_leaf_node(
    name: &str,
    is_last: bool,
    run_state: impl Signal<Item = RunState> + 'static,
) -> Element {
    leaf_node(name, CONDITION_STYLE, run_state, is_last)
}

fn branch_body(
    condition: Option<&Arc<Expression<FunctionId>>>,
    body: &Arc<Body<FunctionId>>,
    nested_block_index: usize,
    call_stack: &CallStack,
    view_state: &ThreadViewState,
) -> Element {
    let is_expandable = is_expandable(body);
    let condition = condition_node(
        condition,
        nested_block_index,
        !is_expandable,
        call_stack,
        view_state,
    );

    clone!(mut call_stack);
    call_stack.push(StackFrame::NestedBlock(nested_block_index));

    let body_elem = row().align_items(Align::Stretch).child(condition);

    if is_expandable {
        body_elem.children(body_statements(body.iter(), &call_stack, view_state))
    } else {
        body_elem
    }
    .into()
}

const CONDITION_STYLE: ButtonStyle = ButtonStyle::Solid(Colour::Info);
