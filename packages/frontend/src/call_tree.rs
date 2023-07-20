use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use futures::{Future, StreamExt};
use futures_signals::signal::{Mutable, ReadOnlyMutable, Signal, SignalExt};
use serpent_automation_executor::{
    library::{FunctionId, Library},
    run::{CallStack, RunState, ThreadRunState},
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
    run_state_map: Rc<RefCell<BTreeMap<CallStack, Mutable<RunState>>>>,
}

impl CallTree {
    pub fn root(fn_id: FunctionId, library: &Rc<Library>) -> Self {
        let f = library.lookup(fn_id);

        let run_state = Mutable::new(RunState::NotRun);
        let mut run_state_map = BTreeMap::new();
        run_state_map.insert(CallStack::new(), run_state.clone());
        let run_state_map = Rc::new(RefCell::new(run_state_map));

        Self {
            span: f.span(),
            name: f.name().to_string(),
            run_state,
            body: Body::from_linked_body(library, f.body()),
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
        run_state: impl Signal<Item = ThreadRunState> + 'static,
    ) -> impl Future<Output = ()> + 'static {
        let run_state_map = self.run_state_map.clone();

        async move {
            let mut run_state = Box::pin(run_state.to_stream());

            while let Some(run_state) = run_state.next().await {
                let _run_state_map = run_state_map.borrow_mut();
                let _call_stack = run_state.current();

                // TODO: Implement
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
        library: &Rc<Library>,
        body: &syntax_tree::LinkedBody,
    ) -> TreeNode<Expandable<Self>> {
        match body {
            LinkedBody::Local(body) if is_expandable(body) => {
                let body = body.clone();
                TreeNode::Internal(Expandable::new({
                    let library = library.clone();
                    move || Self::from_body(&library, &body)
                }))
            }
            LinkedBody::Python | LinkedBody::Local(_) => TreeNode::Leaf,
        }
    }

    fn from_body(library: &Rc<Library>, body: &syntax_tree::Body<FunctionId>) -> Self {
        let mut stmts = Vec::new();

        for stmt in body.iter() {
            match stmt {
                syntax_tree::Statement::Pass => (),
                syntax_tree::Statement::Expression(expr) => stmts.extend(
                    Call::from_expression(library, expr)
                        .into_iter()
                        .map(Statement::Call),
                ),
                syntax_tree::Statement::If {
                    if_span,
                    condition,
                    then_block,
                    else_block,
                } => stmts.push(Statement::If(If::new(
                    library, *if_span, condition, then_block, else_block,
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
    fn new(library: &Rc<Library>, span: SrcSpan, name: FunctionId) -> Self {
        let function = &library.lookup(name);
        let name = function.name().to_string();
        let run_state = Mutable::new(RunState::NotRun);
        let body = Body::from_linked_body(library, function.body());

        Self {
            span,
            name,
            run_state,
            body,
        }
    }

    fn from_expression(
        library: &Rc<Library>,
        expr: &syntax_tree::Expression<FunctionId>,
    ) -> Vec<Call> {
        match expr {
            syntax_tree::Expression::Literal(_) | syntax_tree::Expression::Variable { .. } => {
                Vec::new()
            }
            syntax_tree::Expression::Call { span, name, args } => {
                let mut calls = Vec::new();

                for arg in args {
                    calls.extend(Self::from_expression(library, arg));
                }

                calls.push(Self::new(library, *span, *name));
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
        library: &Rc<Library>,
        span: SrcSpan,
        condition: &syntax_tree::Expression<FunctionId>,
        then_block: &syntax_tree::Body<FunctionId>,
        else_block: &Option<syntax_tree::ElseClause<FunctionId>>,
    ) -> Self {
        let calls = Call::from_expression(library, condition);
        Self {
            span,
            run_state: Mutable::new(RunState::NotRun),
            condition: if calls.is_empty() {
                TreeNode::Leaf
            } else {
                TreeNode::Internal(Expandable::new(|| calls))
            },
            then_block: Body::from_body(library, then_block),
            else_block: else_block
                .as_ref()
                .map(|else_block| Else::new(library, else_block)),
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
    fn new(library: &Rc<Library>, else_block: &ElseClause<FunctionId>) -> Self {
        Self {
            span: else_block.span(),
            run_state: Mutable::new(RunState::NotRun),
            body: Body::from_body(library, else_block.body()),
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
