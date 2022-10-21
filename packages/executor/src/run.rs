use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::library::FunctionId;

pub type CallStack = Vec<FunctionId>;

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
        if self.running.starts_with(call_stack) {
            RunState::Running
        } else if self.completed.contains(call_stack) {
            RunState::Successful
        } else {
            RunState::NotRun
        }
    }

    pub fn push(&mut self, id: FunctionId) {
        self.running.push(id);
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
