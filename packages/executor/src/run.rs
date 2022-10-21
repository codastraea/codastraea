use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::library::FunctionId;

pub type CallStack = Vec<FunctionId>;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ThreadState {
    running: CallStack,
    completed: HashSet<CallStack>,
}

impl ThreadState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status(&self, call_stack: &CallStack) -> FnStatus {
        if self.running.starts_with(call_stack) {
            FnStatus::Running
        } else if self.completed.contains(call_stack) {
            FnStatus::Successful
        } else {
            FnStatus::NotRun
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
pub enum FnStatus {
    NotRun,
    Running,
    Successful,
    Failed,
}
