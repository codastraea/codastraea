use std::{cell::RefCell, collections::HashMap, iter, rc::Rc};

use derive_more::Into;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use serpent_automation_executor::{
    library::{FunctionId, Library},
    run::{CallStack, RunState, StackFrame},
    syntax_tree::{Expression, LinkedBody, LinkedFunction, SrcSpan, Statement},
};
use serpent_automation_frontend::{is_expandable, statement_is_expandable};
use silkenweb::{
    clone,
    elements::{
        html::{self, div, Div},
        ElementEvents,
    },
    node::{
        element::{Element, GenericElement},
        Node,
    },
    prelude::ParentElement,
    value::Sig,
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

use crate::{
    animation::AnimatedExpand, call_tree_view::conditional::if_node, component, ViewCallStates,
};

mod conditional;

// TODO: Accept `path = concat!("css/", "call-tree", ".css")`
component!("css/call-tree.css");

#[derive(Into, Value)]
pub struct CallTree(Node);

impl CallTree {
    pub fn new(
        fn_id: FunctionId,
        actions: impl CallTreeActions,
        library: &Rc<Library>,
        view_call_states: &ViewCallStates,
    ) -> Self {
        let view_state = CallTreeState::new(actions, library.clone(), view_call_states.clone());
        // TODO: Handle error (python functions can't be run directly).
        let span = library.lookup(fn_id).span().unwrap();
        Self(
            div()
                .class(class::container())
                .child(function_node(fn_id, span, CallStack::new(), &view_state))
                .into(),
        )
    }
}

pub trait CallTreeActions: Clone + 'static {
    fn view_code(&self, span: SrcSpan);
}

#[derive(Clone)]
struct CallTreeState<Actions: CallTreeActions> {
    expanded: Rc<RefCell<HashMap<CallStack, Mutable<bool>>>>,
    view_call_states: ViewCallStates,
    library: Rc<Library>,
    actions: Actions,
}

impl<Actions: CallTreeActions> CallTreeState<Actions> {
    fn new(actions: Actions, library: Rc<Library>, view_call_states: ViewCallStates) -> Self {
        Self {
            expanded: Rc::new(RefCell::new(HashMap::new())),
            library,
            view_call_states,
            actions,
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

    fn actions(&self) -> &Actions {
        &self.actions
    }
}

fn function_node<Actions: CallTreeActions>(
    fn_id: FunctionId,
    span: SrcSpan,
    mut call_stack: CallStack,
    view_state: &CallTreeState<Actions>,
) -> GenericElement {
    let f = view_state.lookup_fn(fn_id);
    call_stack.push(StackFrame::Function(fn_id));
    let name = f.name();
    let body = match f.body() {
        LinkedBody::Local(body) => is_expandable(body).then_some(body),
        LinkedBody::Python => None,
    };

    if let Some(body) = body {
        expandable_node(name, FUNCTION_COLOUR, span, &call_stack, view_state, {
            clone!(body, call_stack, view_state);
            move || column().children(body_statements(body.iter(), &call_stack, &view_state))
        })
    } else {
        leaf_node(name, FUNCTION_COLOUR, span, &call_stack, view_state)
    }
}

fn expandable_node<Elem, Actions>(
    type_name: &str,
    colour: Colour,
    span: SrcSpan,
    call_stack: &CallStack,
    view_state: &CallTreeState<Actions>,
    mut expanded: impl FnMut() -> Elem + 'static,
) -> GenericElement
where
    Elem: Into<GenericElement>,
    Actions: CallTreeActions,
{
    let style = ButtonStyle::Solid(colour);
    let is_expanded = view_state.expanded(call_stack);

    column()
        .align_self(Align::Stretch)
        .child(
            item(colour)
                .align_self(Align::Start)
                .border_colour(border_colour(colour))
                .child(
                    button_group(type_name)
                        .dropdown(item_dropdown(
                            type_name, style, span, call_stack, view_state,
                        ))
                        .button(zoom_button(&is_expanded, style)),
                ),
        )
        .child(div().align_self(Align::Stretch).animated_expand(
            move || indented_block().child(expanded().into()),
            is_expanded,
        ))
        .into()
}

fn indented_block() -> Div {
    column()
        .border_on(Side::Start)
        .border_colour(Colour::Secondary)
        .align_items(Align::Start)
        .padding_on_side((Size3, Side::Start))
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

fn item(colour: Colour) -> Div {
    div()
        .position(Position::Relative)
        .class(class::item())
        .border_colour(border_colour(colour))
        .border_on(Side::Bottom)
        .background_colour(colour)
        .rounded_border(true)
}

fn leaf_node<Actions: CallTreeActions>(
    name: &str,
    colour: Colour,
    span: SrcSpan,
    call_stack: &CallStack,
    view_state: &CallTreeState<Actions>,
) -> GenericElement {
    column()
        .align_items(Align::Start)
        .child(item(colour).child(item_dropdown(
            name,
            ButtonStyle::Solid(colour),
            span,
            call_stack,
            view_state,
        )))
        .into()
}

fn item_dropdown<Actions: CallTreeActions>(
    name: &str,
    style: ButtonStyle,
    span: SrcSpan,
    call_stack: &CallStack,
    view_state: &CallTreeState<Actions>,
) -> Dropdown {
    let run_state = view_state.run_state(call_stack).map(|run_state| {
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
                let actions = view_state.actions().clone();
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

fn call<'a, Actions: CallTreeActions>(
    name: FunctionId,
    span: SrcSpan,
    args: &'a [Expression<FunctionId>],
    call_stack: CallStack,
    view_state: &'a CallTreeState<Actions>,
) -> impl Iterator<Item = GenericElement> + 'a {
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
        .chain(iter::once(function_node(
            name, span, call_stack, view_state,
        )))
}

fn expression<Actions: CallTreeActions>(
    expr: &Expression<FunctionId>,
    call_stack: &CallStack,
    view_state: &CallTreeState<Actions>,
) -> Vec<GenericElement> {
    match expr {
        Expression::Variable { .. } | Expression::Literal(_) => Vec::new(),
        Expression::Call { name, args, span } => {
            call(*name, *span, args, call_stack.clone(), view_state).collect()
        }
    }
}

fn body_statements<'a, Actions: CallTreeActions>(
    body: impl Iterator<Item = &'a Statement<FunctionId>> + 'a,
    call_stack: &'a CallStack,
    view_state: &'a CallTreeState<Actions>,
) -> impl Iterator<Item = GenericElement> + 'a {
    body.filter(|stmt| statement_is_expandable(stmt))
        .enumerate()
        .flat_map(move |(stmt_index, statement)| {
            clone!(mut call_stack);
            call_stack.push(StackFrame::Statement(stmt_index));

            match statement {
                Statement::Pass => Vec::new(),
                Statement::Expression(expr) => expression(expr, &call_stack, view_state),
                Statement::If {
                    if_span,
                    condition,
                    then_block,
                    else_block,
                } => vec![if_node(
                    *if_span,
                    condition.clone(),
                    then_block.clone(),
                    else_block,
                    &call_stack,
                    view_state,
                )],
            }
            .into_iter()
        })
}

const FUNCTION_COLOUR: Colour = Colour::Primary;
