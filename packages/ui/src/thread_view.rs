// TODO: Rename ThreadView to CallTreeView?
use std::{cell::RefCell, collections::HashMap, iter, rc::Rc};

use derive_more::Into;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use serpent_automation_executor::{
    library::{FunctionId, Library},
    run::{CallStack, RunState, StackFrame},
    syntax_tree::{Expression, LinkedBody, LinkedFunction, Statement},
};
use serpent_automation_frontend::{is_expandable, statement_is_expandable};
use silkenweb::{
    clone,
    elements::{
        html::{a, div, ABuilder},
        svg::{
            self,
            attributes::Presentation,
            path::{
                Data,
                Offset::{Abs, Rel},
            },
            svg,
        },
        ElementEvents,
    },
    node::{
        element::{Element, ElementBuilder},
        Node,
    },
    prelude::ParentBuilder,
    value::Sig,
    Value,
};
use silkenweb_bootstrap::{
    button::{icon_button, ButtonStyle},
    button_group::button_group,
    column,
    dropdown::{dropdown, dropdown_menu, DropdownBuilder},
    icon::{icon, Icon, IconType},
    utility::{
        Align, Colour, Position, SetAlign, SetBorder, SetFlex, SetGap, SetPosition, SetSpacing,
        Shadow, Side,
        Size::{Size1, Size2, Size3, Size5},
    },
};

use crate::{animation::AnimatedExpand, css, thread_view::conditional::if_node, ViewCallStates};

mod conditional;

#[derive(Into, Value)]
pub struct ThreadView(Node);

impl ThreadView {
    pub fn new(
        fn_id: FunctionId,
        library: &Rc<Library>,
        view_call_states: &ViewCallStates,
    ) -> Self {
        let view_state = ThreadViewState::new(view_call_states.clone(), library.clone());
        Self(
            div()
                .class(css::THREAD_VIEW)
                .child(function_node(fn_id, CallStack::new(), &view_state))
                .into(),
        )
    }
}

#[derive(Clone)]
struct ThreadViewState {
    expanded: Rc<RefCell<HashMap<CallStack, Mutable<bool>>>>,
    view_call_states: ViewCallStates,
    library: Rc<Library>,
}

impl ThreadViewState {
    fn new(view_call_states: ViewCallStates, library: Rc<Library>) -> Self {
        Self {
            expanded: Rc::new(RefCell::new(HashMap::new())),
            library,
            view_call_states,
        }
    }

    fn expanded(&self, call_stack: &CallStack) -> Mutable<bool> {
        self.expanded
            .borrow_mut()
            .entry(call_stack.clone())
            .or_insert_with(|| Mutable::new(false))
            .clone()
    }

    fn run_state(&self, call_stack: &CallStack) -> impl Signal<Item = RunState> {
        self.view_call_states.run_state(call_stack)
    }

    fn lookup_fn(&self, fn_id: FunctionId) -> &LinkedFunction {
        self.library.lookup(fn_id)
    }
}

fn function_node(
    fn_id: FunctionId,
    mut call_stack: CallStack,
    view_state: &ThreadViewState,
) -> Element {
    let f = view_state.lookup_fn(fn_id);
    call_stack.push(StackFrame::Function(fn_id));
    let name = f.name();
    let body = match f.body() {
        LinkedBody::Local(body) => is_expandable(body).then_some(body),
        LinkedBody::Python => None,
    };
    let run_state = view_state.run_state(&call_stack);

    if let Some(body) = body {
        let expanded = view_state.expanded(&call_stack);
        clone!(body, call_stack, view_state);
        let body = move || {
            column()
                .gap(Size1)
                .children(body_statements(body.iter(), &call_stack, &view_state))
        };

        expandable_node(name, FUNCTION_COLOUR, run_state, expanded, body)
    } else {
        leaf_node(name, FUNCTION_COLOUR, run_state)
    }
}

fn expandable_node<Elem>(
    type_name: &str,
    colour: Colour,
    run_state: impl Signal<Item = RunState> + 'static,
    is_expanded: Mutable<bool>,
    mut expanded: impl FnMut() -> Elem + 'static,
) -> Element
where
    Elem: Into<Element>,
{
    let style = ButtonStyle::Solid(colour);

    column()
        .align_self(Align::Stretch)
        .align_items(Align::Start)
        .child(connector(Connector::Socket, colour))
        .child(
            button_group(type_name)
                .dropdown(item_dropdown(type_name, style, run_state))
                .button(zoom_button(&is_expanded, style)),
        )
        .child(connector(Connector::Plug, colour))
        .child(column().align_items(Align::Start).animated_expand(
            move || {
                div()
                    .class(css::SPEECH_BUBBLE_BELOW)
                    .margin_on_side((Some(Size5), Side::Start))
                    .margin_on_side((Some(Size3), Side::Top))
                    .padding(Size3)
                    .border(true)
                    .border_colour(Colour::Secondary)
                    .rounded_border(true)
                    .shadow(Shadow::Medium)
                    .child(expanded().into())
            },
            is_expanded,
        ))
        .into()
}

#[derive(Copy, Clone)]
enum Connector {
    Plug,
    Socket,
}

fn connector(connector: Connector, colour: Colour) -> Element {
    // TODO: Shared SVG
    let width = 32.0;
    let height = 6.0;

    let connector_class = match connector {
        Connector::Plug => css::THREAD_VIEW__PLUG,
        Connector::Socket => css::THREAD_VIEW__SOCKET,
    };

    div()
        .class(colour.text())
        .position(Position::Relative)
        .child(
            svg()
                .classes([css::THREAD_VIEW__CONNECTOR, connector_class])
                .width(width)
                .height(height)
                .stroke_width(1.0)
                .child(svg::path().d(Data::new().move_to(Abs, 0.0, 0.0).lines_to(
                    Rel,
                    [
                        (height, height),
                        (width - height * 2.0, 0.0),
                        (height, -height),
                        (-width, 0.0),
                    ],
                ))),
        )
        .into()
}

fn leaf_node(
    name: &str,
    colour: Colour,
    run_state: impl Signal<Item = RunState> + 'static,
) -> Element {
    column()
        .align_items(Align::Start)
        .child(connector(Connector::Socket, colour))
        .child(item_dropdown(name, ButtonStyle::Solid(colour), run_state))
        .child(connector(Connector::Plug, colour))
        .into()
}

fn item_dropdown(
    name: &str,
    style: ButtonStyle,
    run_state: impl Signal<Item = RunState> + 'static,
) -> DropdownBuilder {
    let run_state = run_state.map(|run_state| {
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
    });

    dropdown(
        icon_button("button", Sig(run_state), style).text(name),
        dropdown_menu().children([dropdown_item("Run"), dropdown_item("Pause")]),
    )
}

fn dropdown_item(name: &str) -> ABuilder {
    a().href("#").text(name)
}

fn zoom_button(
    expanded: &Mutable<bool>,
    style: ButtonStyle,
) -> silkenweb_bootstrap::button::ButtonBuilder {
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

fn call<'a>(
    name: FunctionId,
    args: &'a [Expression<FunctionId>],
    call_stack: CallStack,
    view_state: &'a ThreadViewState,
) -> impl Iterator<Item = Element> + 'a {
    args.iter()
        .enumerate()
        .flat_map({
            clone!(mut call_stack);

            move |(arg_index, arg)| {
                // TODO: Push and pop call stack for efficiency
                call_stack.push(StackFrame::Argument(arg_index));
                expression(arg, &call_stack, view_state)
            }
        })
        .chain(iter::once(function_node(name, call_stack, view_state)))
}

fn expression(
    expr: &Expression<FunctionId>,
    call_stack: &CallStack,
    view_state: &ThreadViewState,
) -> Vec<Element> {
    match expr {
        Expression::Variable { .. } | Expression::Literal(_) => Vec::new(),
        Expression::Call { name, args, .. } => {
            call(*name, args, call_stack.clone(), view_state).collect()
        }
    }
}

fn body_statements<'a>(
    body: impl Iterator<Item = &'a Statement<FunctionId>> + 'a,
    call_stack: &'a CallStack,
    view_state: &'a ThreadViewState,
) -> impl Iterator<Item = Element> + 'a {
    body.filter(|stmt| statement_is_expandable(stmt))
        .enumerate()
        .flat_map(move |(stmt_index, statement)| {
            clone!(mut call_stack);
            call_stack.push(StackFrame::Statement(stmt_index));

            match statement {
                Statement::Pass => Vec::new(),
                Statement::Expression(expr) => expression(expr, &call_stack, view_state),
                Statement::If {
                    condition,
                    then_block,
                    else_block,
                } => vec![if_node(
                    condition.clone(),
                    then_block.clone(),
                    else_block.clone(),
                    &call_stack,
                    view_state,
                )],
            }
            .into_iter()
        })
}

const FUNCTION_COLOUR: Colour = Colour::Primary;
