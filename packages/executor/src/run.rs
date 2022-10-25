use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::library::FunctionId;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum StackFrame {
    Function(FunctionId),
    Statement(usize),
    Argument(usize),
    NestedBlock(usize),
    BlockPredicate(usize),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CallStack(Vec<StackFrame>);

impl CallStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, item: StackFrame) {
        self.0.push(item)
    }

    pub fn pop(&mut self) {
        self.0.pop();
    }

    pub fn is_descendant_or_equal(&self, ancestor: &Self) -> bool {
        self.0.starts_with(&ancestor.0)
    }
}

#[serde_as]
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ThreadCallStates {
    running: CallStack,
    #[serde_as(as = "Vec<(_, _)>")]
    completed: HashMap<CallStack, RunState>,
}

impl ThreadCallStates {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_state(&self, call_stack: &CallStack) -> RunState {
        if self.running.is_descendant_or_equal(call_stack) {
            RunState::Running
        } else if let Some(run_state) = self.completed.get(call_stack) {
            *run_state
        } else {
            RunState::NotRun
        }
    }

    pub fn push(&mut self, item: StackFrame) {
        self.running.push(item);
    }

    pub fn pop(&mut self, run_state: RunState) {
        self.completed.insert(self.running.clone(), run_state);
        self.running.pop();
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum RunState {
    NotRun,
    Running,
    Successful,
    PredicateSuccessful(bool),
    Failed,
}
