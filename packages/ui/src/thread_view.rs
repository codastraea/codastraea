use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use derive_more::Into;
use futures_signals::signal::{Mutable, Signal, SignalExt};
use serpent_automation_executor::{
    library::{FunctionId, Library},
    run::{CallStack, RunState, StackFrame},
    syntax_tree::{Body, Expression, LinkedBody, LinkedFunction, Statement},
};
use serpent_automation_frontend::{
    expression_is_expandable, is_expandable, statement_is_expandable,
};
use silkenweb::{
    clone,
    elements::{
        html::{a, div, ABuilder, DivBuilder},
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
    row,
    utility::{
        Align, Colour, SetAlign, SetBorder, SetColour, SetFlex, SetGap, SetSpacing, Shadow, Side,
        Size::{self, Size3},
    },
};

use crate::{animation::AnimatedExpand, css, speech_bubble::SpeechBubble, ViewCallStates};

#[derive(Into, Value)]
pub struct ThreadView(Node);

impl ThreadView {
    // TODO: Return Result<Self, LinkError>
    pub fn new(
        fn_id: FunctionId,
        library: &Rc<Library>,
        view_call_states: &ViewCallStates,
    ) -> Self {
        let view_state = ThreadViewState::new(view_call_states.clone(), library.clone());
        Self(function(fn_id, true, CallStack::new(), &view_state).into())
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

fn function(
    fn_id: FunctionId,
    is_last: bool,
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
            row()
                .align_items(Align::Start)
                .speech_bubble()
                .children(body_statements(body.iter(), &call_stack, &view_state))
        };

        // TODO: Split `function_header` into `expandable_header` and `leaf_header`?
        expandable_node(
            function_header(name, Some(&expanded), run_state),
            is_last,
            expanded,
            body,
        )
    } else {
        header_row(function_header(name, None, run_state), is_last).into()
    }
}

fn expandable_node<Elem>(
    header: impl Into<Element>,
    is_last: bool,
    is_expanded: Mutable<bool>,
    expanded: impl FnMut() -> Elem + 'static,
) -> Element
where
    Elem: Into<Element>,
{
    column()
        .align_items(Align::Start)
        .child(header_row(header, is_last).align_self(Align::Stretch))
        .animated_expand(expanded, is_expanded)
        .into()
}

fn header_row(header: impl Into<Element>, is_last: bool) -> DivBuilder {
    let main = row().align_items(Align::Center).child(header.into());

    if is_last {
        main
    } else {
        let horizontal_line = div().class(css::HORIZONTAL_LINE);
        let arrow_head = div()
            .class(css::ARROW_HEAD_RIGHT)
            .background_colour(Colour::Secondary);
        main.child(horizontal_line).child(arrow_head)
    }
}

fn if_dropdown(name: &str, run_state: impl Signal<Item = RunState> + 'static) -> DropdownBuilder {
    item_dropdown(name, ButtonStyle::Solid(Colour::Info), run_state)
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
        .margin_on_side((Some(Size::Size2), Side::End))
    });

    dropdown(
        icon_button("button", Sig(run_state), style).text(name),
        dropdown_menu().children([dropdown_item("Run"), dropdown_item("Pause")]),
    )
}

fn dropdown_item(name: &str) -> ABuilder {
    a().href("#").text(name)
}

fn call(
    name: FunctionId,
    args: &[Expression<FunctionId>],
    is_last: bool,
    call_stack: CallStack,
    view_state: &ThreadViewState,
) -> Vec<Element> {
    let mut elems: Vec<Element> = args
        .iter()
        .enumerate()
        .flat_map(|(arg_index, arg)| {
            // TODO: Push and pop call stack for efficiency
            clone!(mut call_stack);
            call_stack.push(StackFrame::Argument(arg_index));
            expression(arg, false, &call_stack, view_state)
        })
        .collect();

    elems.push(function(name, is_last, call_stack, view_state));

    elems
}

fn body_statements<'a>(
    body: impl Iterator<Item = &'a Statement<FunctionId>>,
    call_stack: &'a CallStack,
    view_state: &'a ThreadViewState,
) -> Vec<Element> {
    let body: Vec<_> = body
        .enumerate()
        .filter(|(_index, stmt)| statement_is_expandable(stmt))
        .collect();
    assert!(!body.is_empty());
    let last_index = body.len() - 1;

    body.iter()
        .enumerate()
        .flat_map(move |(index, (stmt_index, statement))| {
            let is_last = index == last_index;
            body_statement(statement, *stmt_index, is_last, call_stack, view_state)
        })
        .collect()
}

fn body_statement<'a>(
    statement: &'a Statement<FunctionId>,
    stmt_index: usize,
    is_last: bool,
    call_stack: &'a CallStack,
    view_state: &'a ThreadViewState,
) -> impl Iterator<Item = Element> + 'a {
    clone!(mut call_stack);
    call_stack.push(StackFrame::Statement(stmt_index));

    match statement {
        Statement::Pass => Vec::new(),
        Statement::Expression(expr) => expression(expr, is_last, &call_stack, view_state),
        Statement::If {
            condition,
            then_block,
            else_block,
        } => if_statement(
            condition.clone(),
            then_block.clone(),
            else_block.clone(),
            is_last,
            &call_stack,
            view_state,
        ),
    }
    .into_iter()
}

fn if_statement(
    condition: Arc<Expression<FunctionId>>,
    then_block: Arc<Body<FunctionId>>,
    else_block: Arc<Body<FunctionId>>,
    is_last: bool,
    call_stack: &CallStack,
    view_state: &ThreadViewState,
) -> Vec<Element> {
    let expanded = view_state.expanded(call_stack);

    // TODO: Draw `If` (i.e. call this) even if it's not expandable.
    let header_row = row()
        .align_items(Align::Center)
        .align_self(Align::Stretch)
        .child(
            button_group("If")
                .dropdown(if_dropdown("If", view_state.run_state(call_stack)))
                .button(zoom_button(&expanded, ButtonStyle::Solid(Colour::Info))),
        );

    // TODO: Make call stack cheap to clone.
    clone!(call_stack, view_state);

    vec![expandable_node(header_row, is_last, expanded, move || {
        column()
            .align_items(Align::Start)
            .align_self(Align::Start)
            .gap(Size3)
            .speech_bubble()
            .child(branch_body(
                Some(&condition),
                &then_block,
                0,
                &call_stack,
                &view_state,
            ))
            .child(branch_body(None, &else_block, 1, &call_stack, &view_state))
    })]
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

    // TODO: trait for adding arrow
    let body_elem = row().align_items(Align::Start).child(condition);

    if is_expandable {
        body_elem.children(body_statements(body.iter(), &call_stack, view_state))
    } else {
        body_elem
    }
    .into()
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
            let header = condition_header("condition", Some(&expanded), run_state);

            clone!(condition, call_stack, view_state);
            expandable_node(header, is_last, expanded, move || {
                row()
                    .align_items(Align::Start)
                    .speech_bubble()
                    .children(expression(&condition, true, &call_stack, &view_state))
            })
        } else {
            // TODO: Condition text (maybe truncated), with tooltip (how does that work on
            // touch)
            condition_main("condition", is_last, run_state)
        }
    } else {
        condition_main("else", is_last, run_state)
    }
}

fn condition_main(
    name: &str,
    is_last: bool,
    run_state: impl Signal<Item = RunState> + 'static,
) -> Element {
    header_row(condition_header(name, None, run_state), is_last).into()
}

fn function_header(
    name: &str,
    expanded: Option<&Mutable<bool>>,
    run_state: impl Signal<Item = RunState> + 'static,
) -> Element {
    node_header(
        "Function",
        name,
        ButtonStyle::Outline(Colour::Secondary),
        expanded,
        run_state,
    )
}

fn condition_header(
    name: &str,
    expanded: Option<&Mutable<bool>>,
    run_state: impl Signal<Item = RunState> + 'static,
) -> Element {
    node_header(
        "Condition",
        name,
        ButtonStyle::Solid(Colour::Info),
        expanded,
        run_state,
    )
}

fn node_header(
    ty: &str,
    name: &str,
    style: ButtonStyle,
    expanded: Option<&Mutable<bool>>,
    run_state: impl Signal<Item = RunState> + 'static,
) -> Element {
    if let Some(expanded) = expanded {
        button_group(format!("{ty} {name}"))
            .shadow(Shadow::Medium)
            .dropdown(item_dropdown(name, style, run_state))
            .button(zoom_button(expanded, style))
            .into()
    } else {
        item_dropdown(name, style, run_state)
            .shadow(Shadow::Medium)
            .into()
    }
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

fn expression(
    expr: &Expression<FunctionId>,
    is_last: bool,
    call_stack: &CallStack,
    view_state: &ThreadViewState,
) -> Vec<Element> {
    match expr {
        Expression::Variable { .. } | Expression::Literal(_) => Vec::new(),
        Expression::Call { name, args } => {
            call(*name, args, is_last, call_stack.clone(), view_state)
        }
    }
}
