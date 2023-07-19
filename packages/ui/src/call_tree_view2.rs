use std::rc::Rc;

use derive_more::Into;
use futures_signals::signal::{always, Mutable, Signal, SignalExt};
use serpent_automation_executor::{
    library::{FunctionId, Library},
    run::RunState,
    syntax_tree::SrcSpan,
};
use serpent_automation_frontend::call_tree::{Body, CallTree, Expandable, Statement, Vertex};
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

use crate::{animation::AnimatedExpand, component, ViewCallStates};

component!("call-tree");

#[derive(Into, Value)]
pub struct CallTreeView(Node);

impl CallTreeView {
    pub fn new(
        fn_id: FunctionId,
        actions: impl CallTreeActions,
        library: &Rc<Library>,
        _view_call_states: &ViewCallStates,
    ) -> Self {
        let call_tree = CallTree::root(fn_id, library);
        // TODO: Handle uwnrap failure (python functions can't be run directly).
        let span = call_tree.span().unwrap();
        let name = call_tree.name();

        Self(
            div()
                .class(class::container())
                .child(call_view(span, name, call_tree.body(), &actions))
                .into(),
        )
    }
}

fn call_view(
    span: SrcSpan,
    name: &str,
    body: &Vertex<Expandable<Body>>,
    actions: &impl CallTreeActions,
) -> GenericElement {
    if let Vertex::Node(body) = body {
        vertex(
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
        leaf(name, FUNCTION_COLOUR, span, actions)
    }
}

fn vertex<Elem>(
    type_name: &str,
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
            item(colour)
                .align_self(Align::Start)
                .border_colour(border_colour(colour))
                .child(
                    button_group(type_name)
                        .dropdown(item_dropdown(type_name, style, span, actions))
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

fn leaf(
    name: &str,
    colour: Colour,
    span: SrcSpan,
    actions: &impl CallTreeActions,
) -> GenericElement {
    column()
        .align_items(Align::Start)
        .child(item(colour).child(item_dropdown(
            name,
            ButtonStyle::Solid(colour),
            span,
            actions,
        )))
        .into()
}

fn item(colour: Colour) -> Div {
    div()
        .position(Position::Relative)
        .class(class::item())
        .border_colour(border_colour(colour))
        .border_on(Side::Bottom)
        .background_colour(colour)
        .rounded_border(true)
}

fn item_dropdown(
    name: &str,
    style: ButtonStyle,
    span: SrcSpan,
    actions: &impl CallTreeActions,
) -> Dropdown {
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
    stmts.filter_map(|stmt| match stmt {
        Statement::Call(call) => Some(call_view(call.span(), call.name(), call.body(), actions)),
        Statement::If(_) => None,
    })
}

pub trait CallTreeActions: Clone + 'static {
    fn view_code(&self, span: SrcSpan);
}

const FUNCTION_COLOUR: Colour = Colour::Primary;
