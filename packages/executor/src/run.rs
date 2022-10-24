use std::collections::HashSet;

use serde::{Deserialize, Serialize};

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

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ThreadCallStates {
    running: CallStack,
    completed: HashSet<CallStack>,
}

impl ThreadCallStates {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_state(&self, call_stack: &CallStack) -> RunState {
        if self.running.is_descendant_or_equal(call_stack) {
            RunState::Running
        } else if self.completed.contains(call_stack) {
            RunState::Successful
        } else {
            RunState::NotRun
        }
    }

    pub fn push(&mut self, item: StackFrame) {
        self.running.push(item);
    }

    pub fn pop(&mut self) {
        self.completed.insert(self.running.clone());
        self.running.pop();
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum RunState {
    NotRun,
    Running,
    Successful,
    Failed,
}
