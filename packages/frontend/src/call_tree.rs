use std::{cell::RefCell, collections::BTreeMap, pin::pin, rc::Rc};

use clonelet::clone;
use futures::{Future, Stream, StreamExt};
use futures_signals::signal::{Mutable, ReadOnlyMutable};
use gloo_console::info;
use serpent_automation_executor::{
    library::{FunctionId, Library},
    run::{CallStack, NestedBlock, RunState, StackFrame},
    syntax_tree::{self, ElseClause, LinkedBody, SrcSpan},
};
use tokio::sync::mpsc;

use crate::{
    is_expandable,
    tree::{Expandable, TreeNode},
    ServerConnection,
};

pub struct CallTree {
    span: Option<SrcSpan>,
    name: String,
    run_state: Mutable<RunState>,
    body: TreeNode<Expandable<Body>>,
    run_state_map: RunStateMap,
}

#[derive(Clone)]
struct RunStateMap {
    run_state_map: Rc<RefCell<BTreeMap<CallStack, Mutable<RunState>>>>,
}

impl RunStateMap {
    pub fn new() -> Self {
        Self {
            run_state_map: Rc::new(RefCell::new(BTreeMap::new())),
        }
    }

    pub fn update_run_state(&self, call_stack: CallStack, new_run_state: RunState) {
        self.run_state_map
            .borrow_mut()
            .entry(call_stack)
            .and_modify(|run_state| run_state.set(new_run_state));
    }

    pub fn insert(&self, call_stack: CallStack) -> Mutable<RunState> {
        let run_state = Mutable::new(RunState::NotRun);
        self.run_state_map
            .borrow_mut()
            .insert(call_stack, run_state.clone());
        run_state
    }
}

// TODO: Rename this
#[derive(Clone)]
struct Builder {
    library: Rc<Library>,
    opened_nodes: mpsc::UnboundedSender<CallStack>,
    run_state_map: RunStateMap,
}

impl CallTree {
    pub fn root(
        fn_id: FunctionId,
        library: &Rc<Library>,
        opened_nodes: mpsc::UnboundedSender<CallStack>,
    ) -> Self {
        let f = library.lookup(fn_id);

        let mut call_stack = CallStack::new();
        opened_nodes.send(call_stack.clone()).unwrap();
        call_stack.push(StackFrame::Call(fn_id));
        let run_state_map = RunStateMap::new();
        let run_state = run_state_map.insert(call_stack.clone());
        let builder = Builder {
            library: library.clone(),
            opened_nodes,
            run_state_map: run_state_map.clone(),
        };

        Self {
            span: f.span(),
            name: f.name().to_string(),
            run_state,
            body: Body::from_linked_body(call_stack, &builder, f.body()),
            run_state_map,
        }
    }

    pub fn span(&self) -> Option<SrcSpan> {
        self.span
    }

    pub fn run_state(&self) -> ReadOnlyMutable<RunState> {
        self.run_state.read_only()
    }

    pub fn update_run_state(
        &self,
        server_connection: ServerConnection,
        opened_nodes: impl Stream<Item = CallStack> + 'static,
    ) -> impl Future<Output = ()> + 'static {
        clone!(self.run_state_map);

        async move {
            info!("Subscribing to thread state updates");
            let run_state_updates = server_connection.subscribe(opened_nodes).await;
            let mut run_state_updates = pin!(run_state_updates);

            while let Some((call_stack, new_run_state)) = run_state_updates.next().await {
                info!(format!("Updating node"));
                run_state_map.update_run_state(call_stack, new_run_state);
            }

            info!("Finished subscribing to thread state updates");
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
        call_stack: CallStack,
        builder: &Builder,
        body: &syntax_tree::LinkedBody,
    ) -> TreeNode<Expandable<Self>> {
        match body {
            LinkedBody::Local(body) if is_expandable(body) => {
                clone!(body);

                TreeNode::Internal(Expandable::new({
                    clone!(builder);

                    move || {
                        // TODO: `send`s fail if there's no-one listening, which happens if we're
                        // not connected to the server. We need a queue that buffers sends even when
                        // there's no listeners, or make sure the server connection end re-uses the
                        // queue on reconnect.
                        builder.opened_nodes.send(call_stack.clone()).unwrap();
                        Self::from_body(call_stack, &builder, &body)
                    }
                }))
            }
            LinkedBody::Python | LinkedBody::Local(_) => TreeNode::Leaf,
        }
    }

    fn from_body(
        call_stack: CallStack,
        builder: &Builder,
        body: &syntax_tree::Body<FunctionId>,
    ) -> Self {
        let mut stmts = Vec::new();

        for (index, stmt) in body.iter().enumerate() {
            let call_stack = call_stack.push_cloned(StackFrame::Statement(index));

            match stmt {
                syntax_tree::Statement::Pass => (),
                syntax_tree::Statement::Expression(expr) => stmts.extend(
                    Call::from_expression(call_stack, builder, expr)
                        .into_iter()
                        .map(Statement::Call),
                ),
                syntax_tree::Statement::If {
                    if_span,
                    condition,
                    then_block,
                    else_block,
                } => stmts.push(Statement::If(If::new(
                    call_stack, builder, *if_span, condition, then_block, else_block,
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
    fn new(call_stack: CallStack, builder: &Builder, span: SrcSpan, name: FunctionId) -> Self {
        let function = &builder.library.lookup(name);

        Self {
            span,
            name: function.name().to_string(),
            run_state: builder.run_state_map.insert(call_stack.clone()),
            body: Body::from_linked_body(call_stack, builder, function.body()),
        }
    }

    fn from_expression(
        call_stack: CallStack,
        builder: &Builder,
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
                        call_stack.push_cloned(StackFrame::Argument(index)),
                        builder,
                        arg,
                    ));
                }

                calls.push(Self::new(
                    call_stack.push_cloned(StackFrame::Call(*name)),
                    builder,
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
        call_stack: CallStack,
        builder: &Builder,
        span: SrcSpan,
        condition: &syntax_tree::Expression<FunctionId>,
        then_block: &syntax_tree::Body<FunctionId>,
        else_block: &Option<syntax_tree::ElseClause<FunctionId>>,
    ) -> Self {
        // TODO: Tidy this
        let condition_call_stack =
            call_stack.push_cloned(StackFrame::NestedBlock(0, NestedBlock::Predicate));

        let calls = Call::from_expression(condition_call_stack.clone(), builder, condition);
        let run_state = builder.run_state_map.insert(condition_call_stack.clone());
        let then_block = Body::from_body(
            call_stack.push_cloned(StackFrame::NestedBlock(0, NestedBlock::Body)),
            builder,
            then_block,
        );

        Self {
            span,
            run_state,
            condition: if calls.is_empty() {
                TreeNode::Leaf
            } else {
                clone!(builder);

                TreeNode::Internal(Expandable::new(move || {
                    builder.opened_nodes.send(condition_call_stack).unwrap();
                    calls
                }))
            },
            then_block,
            else_block: else_block
                .as_ref()
                .map(|else_block| Else::new(1, call_stack, builder, else_block)),
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
        block_index: usize,

        mut call_stack: CallStack,
        builder: &Builder,
        else_block: &ElseClause<FunctionId>,
    ) -> Self {
        let run_state = builder.run_state_map.insert(
            call_stack.push_cloned(StackFrame::NestedBlock(block_index, NestedBlock::Predicate)),
        );

        call_stack.push(StackFrame::NestedBlock(block_index, NestedBlock::Body));

        Self {
            span: else_block.span(),
            run_state,
            body: Body::from_body(call_stack, builder, else_block.body()),
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
