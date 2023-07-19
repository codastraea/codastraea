use futures_signals::signal::SignalExt;
use serpent_automation_executor::syntax_tree::SrcSpan;
use serpent_automation_frontend::{
    call_tree::{Body, Call, If},
    tree::{Expandable, Vertex},
};
use silkenweb::{clone, node::element::GenericElement, prelude::ParentElement};
use silkenweb_bootstrap::{
    button::{button, ButtonStyle},
    column,
    dropdown::{dropdown, dropdown_menu},
    utility::{Align, Axis, Colour, SetAlign, SetDisplay, SetSpacing, Size},
};

use super::{body_statements, dropdown_item, indented_block, item, leaf, node, CallTreeActions};
use crate::call_tree_view::call_view;

pub fn if_node(if_stmt: &If, actions: &impl CallTreeActions) -> GenericElement {
    column()
        .align_items(Align::Start)
        .child(branch_body(
            "if",
            if_stmt.span(),
            if_stmt.condition(),
            if_stmt.then_block(),
            actions,
        ))
        .optional_child(if_stmt.else_block().as_ref().map(|else_block| {
            branch_body(
                "else",
                else_block.span(),
                &Vertex::Leaf,
                else_block.body(),
                actions,
            )
        }))
        .into()
}

fn branch_body(
    name: &str,
    span: SrcSpan,
    condition: &Vertex<Expandable<Vec<Call>>>,
    body: &Body,
    actions: &impl CallTreeActions,
) -> GenericElement {
    let condition = condition_vertex(name, condition, span, actions);

    let body_elem = if !body.is_empty() {
        indented_block().children(body_statements(body.iter(), actions))
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

fn condition_vertex(
    name: &str,
    condition: &Vertex<Expandable<Vec<Call>>>,
    span: SrcSpan,
    actions: &impl CallTreeActions,
) -> GenericElement {
    if let Vertex::Node(condition) = condition {
        // TODO: Condition text (maybe truncated), with tooltip (how does that work on
        // touch)
        node(
            name,
            condition.is_expanded(),
            CONDITION_COLOUR,
            span,
            actions,
            condition.signal().map({
                clone!(actions);
                move |expandable_condition| {
                    expandable_condition.map({
                        clone!(actions);
                        move |condition| {
                            indented_block().children(condition.iter().map(|call| {
                                call_view(call.span(), call.name(), call.body(), &actions)
                            }))
                        }
                    })
                }
            }),
        )
    } else {
        leaf(name, CONDITION_COLOUR, span, actions)
    }
}

const CONDITION_COLOUR: Colour = Colour::Info;
