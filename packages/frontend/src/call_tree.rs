use std::{cell::RefCell, rc::Rc};

use futures::{Future, StreamExt};
use futures_signals::signal::{Mutable, ReadOnlyMutable, Signal, SignalExt};
use serpent_automation_executor::{
    library::{FunctionId, Library},
    run::{CallStack, RunState, StackFrame, ThreadRunState},
    syntax_tree::{self, ElseClause, LinkedBody, SrcSpan},
};

use crate::{
    is_expandable,
    tree::{Expandable, TreeNode},
};

pub struct CallTree {
    span: Option<SrcSpan>,
    name: String,
    run_state: Mutable<RunState>,
    body: TreeNode<Expandable<Body>>,
    run_state_map: RunStateMap,
}

#[derive(Clone)]
struct RunStateMap(Rc<RefCell<RunStateVec>>);

impl RunStateMap {
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(Vec::new())))
    }

    pub fn insert(&self, call_stack: CallStack) -> Mutable<RunState> {
        let run_state = Mutable::new(RunState::NotRun);
        self.0.borrow_mut().push((call_stack, run_state.clone()));
        run_state
    }
}

type RunStateVec = Vec<(CallStack, Mutable<RunState>)>;

impl CallTree {
    pub fn root(fn_id: FunctionId, library: &Rc<Library>) -> Self {
        let f = library.lookup(fn_id);

        let mut call_stack = CallStack::new();
        call_stack.push(StackFrame::Call(fn_id));
        let run_state_map = RunStateMap::new();
        let run_state = run_state_map.insert(call_stack.clone());

        Self {
            span: f.span(),
            name: f.name().to_string(),
            run_state,
            body: Body::from_linked_body(&run_state_map, call_stack, library, f.body()),
            run_state_map,
        }
    }

    pub fn span(&self) -> Option<SrcSpan> {
        self.span
    }

    pub fn run_state(&self) -> ReadOnlyMutable<RunState> {
        self.run_state.read_only()
    }

    // TODO: If we send deltas, change this to a `Stream` or
    // `SignalMap`/`SignalVec`, as signals can loose values.
    pub fn update_run_state(
        &self,
        run_state: impl Signal<Item = ThreadRunState> + 'static,
    ) -> impl Future<Output = ()> + 'static {
        let run_states = self.run_state_map.clone();

        async move {
            let mut run_state = Box::pin(run_state.to_stream());

            while let Some(run_state) = run_state.next().await {
                for (stack, set_run_state) in run_states.0.borrow().iter() {
                    set_run_state.set_neq(run_state.run_state(stack));
                }
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn body(&self) -> &TreeNode<Expandable<Body>> {
        &self.body
    }
}

#[derive(Clone)]
pub struct Body(Rc<Vec<Statement>>);

impl Body {
    fn from_linked_body(
        run_state_map: &RunStateMap,
        call_stack: CallStack,
        library: &Rc<Library>,
        body: &syntax_tree::LinkedBody,
    ) -> TreeNode<Expandable<Self>> {
        match body {
            LinkedBody::Local(body) if is_expandable(body) => {
                let body = body.clone();

                TreeNode::Internal(Expandable::new({
                    let run_state_map = run_state_map.clone();
                    let library = library.clone();

                    move || Self::from_body(&run_state_map, call_stack, &library, &body)
                }))
            }
            LinkedBody::Python | LinkedBody::Local(_) => TreeNode::Leaf,
        }
    }

    fn from_body(
        run_state_map: &RunStateMap,
        call_stack: CallStack,
        library: &Rc<Library>,
        body: &syntax_tree::Body<FunctionId>,
    ) -> Self {
        let mut stmts = Vec::new();

        for (index, stmt) in body.iter().enumerate() {
            let call_stack = call_stack.push_cloned(StackFrame::Statement(index));

            match stmt {
                syntax_tree::Statement::Pass => (),
                syntax_tree::Statement::Expression(expr) => stmts.extend(
                    Call::from_expression(run_state_map, call_stack, library, expr)
                        .into_iter()
                        .map(Statement::Call),
                ),
                syntax_tree::Statement::If {
                    if_span,
                    condition,
                    then_block,
                    else_block,
                } => stmts.push(Statement::If(If::new(
                    run_state_map,
                    call_stack,
                    library,
                    *if_span,
                    condition,
                    then_block,
                    else_block,
                ))),
            }
        }

        Self(Rc::new(stmts))
    }

    pub fn iter(&self) -> impl Iterator<Item = &'_ Statement> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub enum Statement {
    Call(Call),
    If(If),
}

#[derive(Clone)]
pub struct Call {
    span: SrcSpan,
    name: String,
    run_state: Mutable<RunState>,
    body: TreeNode<Expandable<Body>>,
}

impl Call {
    fn new(
        run_state_map: &RunStateMap,
        call_stack: CallStack,
        library: &Rc<Library>,
        span: SrcSpan,
        name: FunctionId,
    ) -> Self {
        let function = &library.lookup(name);

        Self {
            span,
            name: function.name().to_string(),
            run_state: run_state_map.insert(call_stack.clone()),
            body: Body::from_linked_body(run_state_map, call_stack, library, function.body()),
        }
    }

    fn from_expression(
        run_state_map: &RunStateMap,
        call_stack: CallStack,
        library: &Rc<Library>,
        expr: &syntax_tree::Expression<FunctionId>,
    ) -> Vec<Call> {
        match expr {
            syntax_tree::Expression::Literal(_) | syntax_tree::Expression::Variable { .. } => {
                Vec::new()
            }
            syntax_tree::Expression::Call { span, name, args } => {
                let mut calls = Vec::new();

                for (index, arg) in args.iter().enumerate() {
                    calls.extend(Self::from_expression(
                        run_state_map,
                        call_stack.push_cloned(StackFrame::Argument(index)),
                        library,
                        arg,
                    ));
                }

                calls.push(Self::new(
                    run_state_map,
                    call_stack.push_cloned(StackFrame::Call(*name)),
                    library,
                    *span,
                    *name,
                ));
                calls
            }
        }
    }

    pub fn span(&self) -> SrcSpan {
        self.span
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn run_state(&self) -> ReadOnlyMutable<RunState> {
        self.run_state.read_only()
    }

    pub fn body(&self) -> &TreeNode<Expandable<Body>> {
        &self.body
    }
}

pub struct If {
    span: SrcSpan,
    run_state: Mutable<RunState>,
    condition: TreeNode<Expandable<Vec<Call>>>,
    then_block: Body,
    else_block: Option<Else>,
}

impl If {
    fn new(
        run_state_map: &RunStateMap,
        call_stack: CallStack,
        library: &Rc<Library>,
        span: SrcSpan,
        condition: &syntax_tree::Expression<FunctionId>,
        then_block: &syntax_tree::Body<FunctionId>,
        else_block: &Option<syntax_tree::ElseClause<FunctionId>>,
    ) -> Self {
        let calls = Call::from_expression(
            run_state_map,
            call_stack.push_cloned(StackFrame::BlockPredicate(0)),
            library,
            condition,
        );

        Self {
            span,
            run_state: run_state_map.insert(call_stack.clone()),
            condition: if calls.is_empty() {
                TreeNode::Leaf
            } else {
                TreeNode::Internal(Expandable::new(|| calls))
            },
            then_block: Body::from_body(
                run_state_map,
                call_stack.push_cloned(StackFrame::NestedBlock(0)),
                library,
                then_block,
            ),
            else_block: else_block.as_ref().map(|else_block| {
                Else::new(
                    run_state_map,
                    call_stack.push_cloned(StackFrame::NestedBlock(1)),
                    library,
                    else_block,
                )
            }),
        }
    }

    pub fn span(&self) -> SrcSpan {
        self.span
    }

    pub fn run_state(&self) -> ReadOnlyMutable<RunState> {
        self.run_state.read_only()
    }

    pub fn condition(&self) -> &TreeNode<Expandable<Vec<Call>>> {
        &self.condition
    }

    pub fn then_block(&self) -> &Body {
        &self.then_block
    }

    pub fn else_block(&self) -> &Option<Else> {
        &self.else_block
    }
}

pub struct Else {
    span: SrcSpan,
    run_state: Mutable<RunState>,
    body: Body,
}

impl Else {
    fn new(
        run_state_map: &RunStateMap,
        call_stack: CallStack,
        library: &Rc<Library>,
        else_block: &ElseClause<FunctionId>,
    ) -> Self {
        Self {
            span: else_block.span(),
            run_state: run_state_map.insert(call_stack.clone()),
            body: Body::from_body(run_state_map, call_stack, library, else_block.body()),
        }
    }

    pub fn run_state(&self) -> ReadOnlyMutable<RunState> {
        self.run_state.read_only()
    }

    pub fn span(&self) -> SrcSpan {
        self.span
    }

    pub fn body(&self) -> &Body {
        &self.body
    }
}
