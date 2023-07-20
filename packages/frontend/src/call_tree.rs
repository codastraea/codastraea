use std::rc::Rc;

use serpent_automation_executor::{
    library::{FunctionId, Library},
    syntax_tree::{self, ElseClause, LinkedBody, SrcSpan},
};

use crate::{
    is_expandable,
    tree::{Expandable, TreeNode},
};

pub struct CallTree {
    span: Option<SrcSpan>,
    name: String,
    body: TreeNode<Expandable<Body>>,
}

impl CallTree {
    pub fn root(fn_id: FunctionId, library: &Rc<Library>) -> Self {
        let f = library.lookup(fn_id);

        Self {
            span: f.span(),
            name: f.name().to_string(),
            body: Body::from_linked_body(library, f.body()),
        }
    }

    pub fn span(&self) -> Option<SrcSpan> {
        self.span
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
    body: TreeNode<Expandable<Body>>,
}

impl Call {
    fn new(library: &Rc<Library>, span: SrcSpan, name: FunctionId) -> Self {
        let function = &library.lookup(name);
        let name = function.name().to_string();
        let body = Body::from_linked_body(library, function.body());

        Self { span, name, body }
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

    pub fn body(&self) -> &TreeNode<Expandable<Body>> {
        &self.body
    }
}

pub struct If {
    span: SrcSpan,
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
    body: Body,
}

impl Else {
    fn new(library: &Rc<Library>, else_block: &ElseClause<FunctionId>) -> Self {
        Self {
            span: else_block.span(),
            body: Body::from_body(library, else_block.body()),
        }
    }

    pub fn span(&self) -> SrcSpan {
        self.span
    }

    pub fn body(&self) -> &Body {
        &self.body
    }
}
