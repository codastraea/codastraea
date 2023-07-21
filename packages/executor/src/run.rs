use std::{cmp::Ordering, collections::BTreeMap};

use serde::{Deserialize, Serialize};

use crate::library::FunctionId;

// The order of the enum variants is important, as we rely on later call stacks
// to be greater than earlier ones.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum StackFrame {
    Statement(usize),
    Argument(usize),
    Function(FunctionId),
    BlockPredicate(usize),
    NestedBlock(usize),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CallStack(Vec<StackFrame>);

impl Ord for CallStack {
    fn cmp(&self, other: &Self) -> Ordering {
        for (i, j) in self.0.iter().zip(other.0.iter()) {
            let cmp = i.cmp(j);

            if cmp != Ordering::Equal {
                return cmp;
            }
        }

        other.0.len().cmp(&self.0.len())
    }
}

impl PartialOrd for CallStack {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// TODO: We need a `block_index` at each level to disambiguate.
impl CallStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn starts_with(&self, other: &Self) -> bool {
        self.0.starts_with(&other.0)
    }

    pub fn push(&mut self, item: StackFrame) {
        self.0.push(item)
    }

    pub fn push_cloned(&self, item: StackFrame) -> Self {
        let mut clone = self.clone();
        clone.push(item);
        clone
    }

    pub fn pop(&mut self) {
        self.0.pop();
    }
}

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct ThreadRunState {
    // TODO: Compress runs of `RunState::Success`
    // TODO: Register which callstacks we're interested in.
    history: BTreeMap<CallStack, RunState>,
    current: CallStack,
}

impl ThreadRunState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_state(&self, stack: &CallStack) -> RunState {
        if self.current.starts_with(stack) {
            return RunState::Running;
        }

        *self.history.get(stack).unwrap_or(&RunState::NotRun)
    }

    pub fn push(&mut self, item: StackFrame) {
        self.current.push(item);
    }

    pub fn pop_success(&mut self) {
        self.pop(RunState::Successful);
    }

    pub fn pop_failed(&mut self) {
        self.pop(RunState::Failed);
    }

    pub fn pop_predicate_success(&mut self, result: bool) {
        self.pop(RunState::PredicateSuccessful(result));
    }

    fn pop(&mut self, run_state: RunState) {
        if let Some((last, _)) = self.history.last_key_value() {
            assert!(last < &self.current);
        }

        self.history.insert(self.current.clone(), run_state);
        self.current.pop();
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum RunState {
    NotRun,
    Running,
    Successful,
    PredicateSuccessful(bool),
    Failed,
}
