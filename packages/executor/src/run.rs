use serde::{Deserialize, Serialize};

use crate::library::FunctionId;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum StackFrame {
    Function(FunctionId),
    Statement(usize),
    Argument(usize),
    NestedBlock(usize),
    BlockPredicate(usize),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CallStack(Vec<StackFrame>);

// TODO: We need a `block_index` at each level to disambiguate.
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
}

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct ThreadRunState {
    current: CallStack,
}

impl ThreadRunState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, item: StackFrame) {
        self.current.push(item);
    }

    // TODO: Is run state the right thing to send? We want to know if it's running
    // or failed.
    pub fn pop(&mut self, _run_state: RunState) {
        // TODO: Need to notify client of failed run states
        self.current.pop();
    }

    pub fn current(&self) -> &CallStack {
        &self.current
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum RunState {
    NotRun,
    Running,
    Successful,
    Failed,
}
