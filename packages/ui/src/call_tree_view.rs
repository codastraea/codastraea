use std::rc::Rc;

use derive_more::Into;
use futures_signals::signal::{always, Mutable, Signal, SignalExt};
use serpent_automation_executor::{
    library::{FunctionId, Library},
    run::RunState,
    syntax_tree::SrcSpan,
};
use serpent_automation_frontend::{
    call_tree::{Body, Call, CallTree, Statement},
    tree::{Expandable, TreeNode},
};
use silkenweb::{
    clone,
    node::{element::GenericElement, Node},
    prelude::{
        html::{self, div, Div},
        Element, ElementEvents, ParentElement,
    },
    value::{Sig, Value},
    Value,
};
use silkenweb_bootstrap::{
    button::{icon_button, ButtonStyle},
    button_group::button_group,
    column,
    dropdown::{dropdown, dropdown_menu, Dropdown},
    icon::{icon, Icon, IconType},
    utility::{
        Align, Colour, Position, SetAlign, SetBorder, SetColour, SetDisplay, SetPosition,
        SetSpacing, Side,
        Size::{Size2, Size3},
    },
};

use self::conditional::if_node;
use crate::{animation::AnimatedExpand, component};

mod conditional;

component!("call-tree");

#[derive(Into, Value)]
pub struct CallTreeView(Node);

impl CallTreeView {
    pub fn new(fn_id: FunctionId, actions: impl CallTreeActions, library: &Rc<Library>) -> Self {
        let call_tree = CallTree::root(fn_id, library);
        // TODO: Handle uwnrap failure (python functions can't be run directly).
        let span = call_tree.span().unwrap();
        let name = call_tree.name();

        Self(
            div()
                .class(class::container())
                .child(node(span, name, call_tree.body(), &actions))
                .into(),
        )
    }
}

fn call_view(call: &Call, actions: &impl CallTreeActions) -> GenericElement {
    node(call.span(), call.name(), call.body(), actions)
}

fn node(
    span: SrcSpan,
    name: &str,
    body: &TreeNode<Expandable<Body>>,
    actions: &impl CallTreeActions,
) -> GenericElement {
    if let TreeNode::Internal(body) = body {
        internal_node(
            name,
            body.is_expanded(),
            FUNCTION_COLOUR,
            span,
            actions,
            body.signal().map({
                clone!(actions);
                move |expandable_body| {
                    expandable_body.map({
                        clone!(actions);
                        move |body| column().children(body_statements(body.iter(), &actions))
                    })
                }
            }),
        )
    } else {
        leaf_node(name, FUNCTION_COLOUR, span, actions)
    }
}

fn internal_node<Elem>(
    name: &str,
    is_expanded: &Mutable<bool>,
    colour: Colour,
    span: SrcSpan,
    actions: &impl CallTreeActions,
    expandable_child_signal: impl Signal<Item = Option<Elem>> + 'static,
) -> GenericElement
where
    Elem: Into<Node> + Value + 'static,
{
    let style = ButtonStyle::Solid(colour);

    column()
        .align_self(Align::Stretch)
        .child(
            node_container(colour)
                .align_self(Align::Start)
                .border_colour(border_colour(colour))
                .child(
                    button_group(name)
                        .dropdown(node_dropdown(name, style, span, actions))
                        .button(zoom_button(is_expanded, style)),
                ),
        )
        .child(div().align_self(Align::Stretch).animated_expand(
            expandable_child_signal.map(|expandable_child| {
                expandable_child.map(|child| indented_block().child(child))
            }),
        ))
        .into()
}

fn zoom_button(
    expanded: &Mutable<bool>,
    style: ButtonStyle,
) -> silkenweb_bootstrap::button::Button {
    icon_button(
        "button",
        icon(Sig(expanded.signal().map(|expanded| {
            if expanded {
                IconType::ZoomOut
            } else {
                IconType::ZoomIn
            }
        }))),
        style,
    )
    .on_click({
        clone!(expanded);
        move |_, _| {
            expanded.replace_with(|e| !*e);
        }
    })
}

fn indented_block() -> Div {
    column()
        .border_on(Side::Start)
        .border_colour(Colour::Secondary)
        .align_items(Align::Start)
        .padding_on_side((Size3, Side::Start))
}

fn leaf_node(
    name: &str,
    colour: Colour,
    span: SrcSpan,
    actions: &impl CallTreeActions,
) -> GenericElement {
    column()
        .align_items(Align::Start)
        .child(node_container(colour).child(node_dropdown(
            name,
            ButtonStyle::Solid(colour),
            span,
            actions,
        )))
        .into()
}

fn node_container(colour: Colour) -> Div {
    div()
        .position(Position::Relative)
        .class(class::item())
        .border_colour(border_colour(colour))
        .border_on(Side::Bottom)
        .background_colour(colour)
        .rounded_border(true)
}

fn node_dropdown(
    name: &str,
    style: ButtonStyle,
    span: SrcSpan,
    actions: &impl CallTreeActions,
) -> Dropdown {
    // TODO: Get run state from call_tree
    let run_state = always(RunState::NotRun).map(|run_state| {
        match run_state {
            RunState::NotRun => Icon::circle().colour(Colour::Secondary),
            RunState::Running => Icon::play_circle_fill().colour(Colour::Primary),
            RunState::Successful | RunState::PredicateSuccessful(true) => {
                Icon::check_circle_fill().colour(Colour::Success)
            }
            RunState::PredicateSuccessful(false) => Icon::circle_fill().colour(Colour::Success),
            RunState::Failed => Icon::exclamation_triangle_fill().colour(Colour::Danger),
        }
        .margin_on_side((Some(Size2), Side::End))
        .class(class::node_status_icon())
    });

    dropdown(
        icon_button("button", Sig(run_state), style).text(name),
        dropdown_menu().children([
            dropdown_item("View code").on_click({
                clone!(actions);
                move |_, _| actions.view_code(span)
            }),
            dropdown_item("Run"),
            dropdown_item("Pause"),
        ]),
    )
}

fn dropdown_item(name: &str) -> html::Button {
    html::button().text(name)
}

fn border_colour(colour: Colour) -> Colour {
    match colour {
        Colour::Primary => Colour::Dark,
        Colour::Secondary => Colour::Dark,
        Colour::Success => Colour::Dark,
        Colour::Danger => Colour::Dark,
        Colour::Warning => Colour::Dark,
        Colour::Info => Colour::Secondary,
        Colour::Light => Colour::Secondary,
        Colour::Dark => Colour::Secondary,
    }
}

fn body_statements<'a>(
    stmts: impl Iterator<Item = &'a Statement> + 'a,
    actions: &'a impl CallTreeActions,
) -> impl Iterator<Item = GenericElement> + 'a {
    stmts.map(|stmt| match stmt {
        Statement::Call(call) => call_view(call, actions),
        Statement::If(if_stmt) => if_node(if_stmt, actions),
    })
}

pub trait CallTreeActions: Clone + 'static {
    fn view_code(&self, span: SrcSpan);
}

const FUNCTION_COLOUR: Colour = Colour::Primary;
