use std::sync::Arc;

use serpent_automation_executor::{
    library::FunctionId,
    run::{CallStack, StackFrame},
    syntax_tree::{Body, ElseClause, Expression, SrcSpan},
};
use serpent_automation_frontend::{expression_is_expandable, is_expandable};
use silkenweb::{clone, node::element::GenericElement, prelude::ParentElement};
use silkenweb_bootstrap::{
    button::{button, ButtonStyle},
    column,
    dropdown::{dropdown, dropdown_menu},
    utility::{Align, Axis, Colour, SetAlign, SetDisplay, SetSpacing, Size},
};

use super::{leaf_node, CallTreeActions, CallTreeState};
use crate::call_tree_view::{
    body_statements, dropdown_item, expandable_node, expression, indented_block, item,
};

pub(super) fn if_node<Actions: CallTreeActions>(
    if_span: SrcSpan,
    condition: Arc<Expression<FunctionId>>,
    then_block: Arc<Body<FunctionId>>,
    else_block: &Option<ElseClause<FunctionId>>,
    call_stack: &CallStack,
    view_state: &CallTreeState<Actions>,
) -> GenericElement {
    // TODO: Make call stack cheap to clone.
    column()
        .align_items(Align::Start)
        .child(branch_body(
            if_span,
            Some(&condition),
            &then_block,
            0,
            call_stack,
            view_state,
        ))
        .optional_child(else_block.as_ref().map(|else_block| {
            branch_body(
                else_block.span(),
                None,
                else_block.body(),
                1,
                call_stack,
                view_state,
            )
        }))
        .into()
}

fn condition_node<Actions: CallTreeActions>(
    condition: Option<&Arc<Expression<FunctionId>>>,
    span: SrcSpan,
    block_index: usize,
    call_stack: &CallStack,
    view_state: &CallTreeState<Actions>,
) -> GenericElement {
    clone!(mut call_stack);
    call_stack.push(StackFrame::BlockPredicate(block_index));
    if let Some(condition) = condition {
        // TODO: Condition text (maybe truncated), with tooltip (how does that work on
        // touch)
        if expression_is_expandable(condition) {
            expandable_node(
                "if condition",
                CONDITION_COLOUR,
                span,
                &call_stack,
                view_state,
                {
                    clone!(condition, call_stack, view_state);
                    move || {
                        indented_block().children(expression(&condition, &call_stack, &view_state))
                    }
                },
            )
        } else {
            condition_leaf_node("if condition", span, &call_stack, view_state)
        }
    } else {
        condition_leaf_node("else", span, &call_stack, view_state)
    }
}

fn condition_leaf_node<Actions: CallTreeActions>(
    name: &str,
    span: SrcSpan,
    call_stack: &CallStack,
    view_state: &CallTreeState<Actions>,
) -> GenericElement {
    leaf_node(name, CONDITION_COLOUR, span, call_stack, view_state)
}

fn branch_body<Actions: CallTreeActions>(
    span: SrcSpan,
    condition: Option<&Arc<Expression<FunctionId>>>,
    body: &Arc<Body<FunctionId>>,
    nested_block_index: usize,
    call_stack: &CallStack,
    view_state: &CallTreeState<Actions>,
) -> GenericElement {
    let is_expandable = is_expandable(body);
    let condition = condition_node(condition, span, nested_block_index, call_stack, view_state);

    clone!(mut call_stack);
    call_stack.push(StackFrame::NestedBlock(nested_block_index));

    let body_elem = if is_expandable {
        indented_block().children(body_statements(body.iter(), &call_stack, view_state))
    } else {
        indented_block().child(
            item(Colour::Secondary).child(dropdown(
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

const CONDITION_COLOUR: Colour = Colour::Info;
