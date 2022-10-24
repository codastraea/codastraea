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
    prelude::{HtmlElement, HtmlElementEvents, ParentBuilder},
    task::on_animation_frame,
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

use crate::{css, ViewCallStates};

const BUTTON_STYLE: ButtonStyle = ButtonStyle::Outline(Colour::Secondary);

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

struct ExpandableBody {
    expanded: Mutable<bool>,
    body: Arc<Body<FunctionId>>,
}

impl ExpandableBody {
    fn new(body: LinkedBody, call_stack: &CallStack, view_state: &ThreadViewState) -> Option<Self> {
        match body {
            LinkedBody::Local(body) => is_expandable(&body).then(|| ExpandableBody {
                expanded: view_state.expanded(call_stack),
                body,
            }),
            LinkedBody::Python => None,
        }
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
    let expandable_body = ExpandableBody::new(f.body().clone(), &call_stack, view_state);
    let run_state = view_state.run_state(&call_stack);
    let header = function_header(
        f.name(),
        expandable_body.as_ref().map(|body| &body.expanded),
        run_state,
    );
    let mut main = row().align_items(Align::Center).child(header);

    if !is_last {
        main = main.child(horizontal_line()).child(arrow_right());
    }

    if let Some(ExpandableBody { expanded, body }) = expandable_body {
        column()
            .align_items(Align::Start)
            .child(main.align_self(Align::Stretch))
            .animated_expand(
                {
                    clone!(body, call_stack, view_state);
                    move || expanded_body(&body, &call_stack, &view_state).into()
                },
                expanded,
            )
            .into()
    } else {
        main.into()
    }
}

fn if_dropdown(name: &str, run_state: impl Signal<Item = RunState> + 'static) -> DropdownBuilder {
    item_dropdown(name, Colour::Primary, run_state)
}

fn fn_dropdown(name: &str, run_state: impl Signal<Item = RunState> + 'static) -> DropdownBuilder {
    item_dropdown(name, Colour::Secondary, run_state)
}

fn item_dropdown(
    name: &str,
    colour: Colour,
    run_state: impl Signal<Item = RunState> + 'static,
) -> DropdownBuilder {
    let run_state = run_state.map(|run_state| {
        match run_state {
            RunState::NotRun => Icon::circle().colour(Colour::Secondary),
            RunState::Running => Icon::play_circle_fill().colour(Colour::Primary),
            RunState::Successful => Icon::check_circle_fill().colour(Colour::Success),
            RunState::Failed => Icon::exclamation_triangle_fill().colour(Colour::Danger),
        }
        .margin_on_side((Some(Size::Size2), Side::End))
    });

    dropdown(
        icon_button("button", Sig(run_state), ButtonStyle::Outline(colour)).text(name),
        dropdown_menu().children([dropdown_item("Run"), dropdown_item("Pause")]),
    )
}

fn dropdown_item(name: &str) -> ABuilder {
    a().href("#").text(name)
}

fn horizontal_line() -> Element {
    div().class(css::HORIZONTAL_LINE).into()
}

fn arrow_right() -> Element {
    div()
        .class(css::ARROW_HEAD_RIGHT)
        .background_colour(Colour::Secondary)
        .into()
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

fn style_size(bound: &str, width: f64, height: f64) -> String {
    format!("overflow: hidden; {bound}-width: {width}px; {bound}-height: {height}px",)
}

fn style_max_size(width: f64, height: f64) -> String {
    style_size("max", width, height)
}

fn style_min_size(width: f64, height: f64) -> String {
    style_size("min", width, height)
}

pub trait AnimatedExpand {
    fn animated_expand(
        self,
        child: impl FnMut() -> Element + 'static,
        expanded: Mutable<bool>,
    ) -> Self;
}

impl AnimatedExpand for DivBuilder {
    fn animated_expand(
        self,
        mut body: impl FnMut() -> Element + 'static,
        expanded: Mutable<bool>,
    ) -> Self {
        let style = Mutable::new("".to_owned());
        let show_body = Mutable::new(false);
        let parent = self.handle().dom_element();

        let expanding_elem = div()
            .class(css::TRANSITION_ALL)
            .spawn_future(expanded.signal().for_each({
                clone!(show_body);
                move |expanded| {
                    if expanded {
                        show_body.set(true);
                    }
                    async {}
                }
            }))
            .effect_signal(expanded.signal(), {
                clone!(style, show_body);
                move |elem, expanded| {
                    let elem_bounds = elem.get_bounding_client_rect();

                    if expanded {
                        let initial_width = parent.get_bounding_client_rect().width();
                        let final_bounds = elem.get_bounding_client_rect();
                        style.set(style_max_size(initial_width, 0.0));

                        on_animation_frame({
                            clone!(style);
                            move || {
                                style.set(style_max_size(
                                    final_bounds.width(),
                                    final_bounds.height(),
                                ));
                            }
                        })
                    } else {
                        style.set(style_min_size(elem_bounds.width(), elem_bounds.height()));

                        on_animation_frame({
                            clone!(show_body);
                            move || show_body.set(false)
                        });
                    }
                }
            })
            .on_transitionend({
                clone!(style);
                move |_, _| style.set("".to_owned())
            })
            .style(Sig(style.signal_cloned()))
            .optional_child(Sig(show_body.signal().map({
                move |expanded| {
                    if expanded {
                        Some(body())
                    } else {
                        style.set(style_min_size(0.0, 0.0));
                        None
                    }
                }
            })));

        self.child(expanding_elem)
    }
}

fn expanded_body(
    body: &Body<FunctionId>,
    call_stack: &CallStack,
    view_state: &ThreadViewState,
) -> DivBuilder {
    let row = row()
        .align_items(Align::Start)
        .speech_bubble()
        .children(body_statements(body.iter(), call_stack, view_state));
    row
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
    let mut main = row()
        .align_items(Align::Center)
        .align_self(Align::Stretch)
        .child(
            button_group("If")
                .dropdown(if_dropdown("If", view_state.run_state(call_stack)))
                .button(zoom_button(&expanded))
                .rounded_pill_border(true),
        );

    if !is_last {
        main = main.child(horizontal_line()).child(arrow_right());
    }

    // TODO: Make call stack cheap to clone.
    clone!(call_stack, view_state);

    vec![column()
        .align_items(Align::Start)
        .child(main)
        .animated_expand(
            move || {
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
                    .into()
            },
            expanded,
        )
        .align_self(Align::Start)
        .into()]
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

    // TODO: Factor out expandable node container
    let container = column().align_items(Align::Start);

    if let Some(condition) = condition {
        if expression_is_expandable(condition) {
            let expanded = view_state.expanded(&call_stack);
            clone!(condition, call_stack, view_state);
            container
                .child(condition_main(
                    "condition",
                    is_last,
                    Some(&expanded),
                    view_state.run_state(&call_stack),
                ))
                .animated_expand(
                    move || {
                        row()
                            .align_items(Align::Start)
                            .speech_bubble()
                            .children(expression(&condition, true, &call_stack, &view_state))
                            .into()
                    },
                    expanded,
                )
        } else {
            // TODO: Condition text (maybe truncated), with tooltip (how does that work on
            // touch)
            container.child(condition_main(
                "condition",
                is_last,
                None,
                view_state.run_state(&call_stack),
            ))
        }
    } else {
        container.child(condition_main(
            "else",
            is_last,
            None,
            view_state.run_state(&call_stack),
        ))
    }
    .into()
}

fn condition_main(
    name: &str,
    is_last: bool,
    expanded: Option<&Mutable<bool>>,
    run_state: impl Signal<Item = RunState> + 'static,
) -> Element {
    let main = row()
        .align_items(Align::Center)
        .align_self(Align::Stretch)
        .child(condition_header(name, expanded, run_state));

    if !is_last {
        main.child(horizontal_line()).child(arrow_right())
    } else {
        main
    }
    .into()
}

fn function_header(
    name: &str,
    expanded: Option<&Mutable<bool>>,
    run_state: impl Signal<Item = RunState> + 'static,
) -> Element {
    node_header("Function", name, Colour::Secondary, expanded, run_state)
}

fn condition_header(
    name: &str,
    expanded: Option<&Mutable<bool>>,
    run_state: impl Signal<Item = RunState> + 'static,
) -> Element {
    node_header("Condition", name, Colour::Primary, expanded, run_state)
}

fn node_header(
    ty: &str,
    name: &str,
    colour: Colour,
    expanded: Option<&Mutable<bool>>,
    run_state: impl Signal<Item = RunState> + 'static,
) -> Element {
    if let Some(expanded) = expanded {
        button_group(format!("{ty} {name}"))
            .shadow(Shadow::Medium)
            .dropdown(item_dropdown(name, colour, run_state))
            .button(zoom_button(expanded))
            .into()
    } else {
        fn_dropdown(name, run_state).shadow(Shadow::Medium).into()
    }
}

fn zoom_button(expanded: &Mutable<bool>) -> silkenweb_bootstrap::button::ButtonBuilder {
    icon_button(
        "button",
        icon(Sig(expanded.signal().map(|expanded| {
            if expanded {
                IconType::ZoomOut
            } else {
                IconType::ZoomIn
            }
        }))),
        BUTTON_STYLE,
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

trait SpeechBubble: HtmlElement {
    fn speech_bubble(self) -> Self {
        self.class(css::SPEECH_BUBBLE_BELOW)
            .margin_on_side((Some(Size3), Side::Top))
            .margin_on_side((Some(Size3), Side::End))
            .padding(Size3)
            .border(true)
            .border_colour(Colour::Secondary)
            .rounded_border(true)
            .shadow(Shadow::Medium)
    }
}

impl<T: HtmlElement> SpeechBubble for T {}
