use std::cmp::{min, Ordering};

use serde::{Deserialize, Serialize};

use crate::library::FunctionId;

// The order of the enum variants is important, as we rely on later call stacks
// to be greater than earlier ones.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum StackFrame {
    Statement(usize),
    Argument(usize),
    Call(FunctionId),
    NestedBlock(usize, NestedBlock),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum NestedBlock {
    Predicate,
    Body,
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

impl CallStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn starts_with(&self, other: &Self) -> bool {
        self.0.starts_with(&other.0)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn common_prefix(&self, other: &Self) -> Self {
        let mut result = Self::new();

        for (i, j) in self.0.iter().zip(other.0.iter()) {
            if i != j {
                return result;
            }

            result.0.push(*i);
        }

        result
    }

    pub fn common_prefix_len(&self, other: &Self) -> usize {
        for (len, (i, j)) in self.0.iter().zip(other.0.iter()).enumerate() {
            if i != j {
                return len;
            }
        }

        min(self.0.len(), other.0.len())
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
    history: Vec<(CallStack, RunState)>,
    last_completed: Option<CallStack>,
    current: CallStack,
}

impl ThreadRunState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last_completed(&self) -> &Option<CallStack> {
        &self.last_completed
    }

    pub fn current(&self) -> &CallStack {
        &self.current
    }

    pub fn run_state(&self, stack: &CallStack) -> RunState {
        if self.current.starts_with(stack) {
            return RunState::Running;
        }

        if Some(stack) > self.last_completed.as_ref() {
            return RunState::NotRun;
        }

        let insert_index = match self
            .history
            .binary_search_by_key(&stack, |(call_stack, _)| call_stack)
        {
            Ok(match_index) => return self.history[match_index].1,
            Err(insert_index) => insert_index,
        };

        if insert_index == 0 {
            return RunState::NotRun;
        }

        let run_state = self.history[insert_index - 1].1;

        if run_state == RunState::PredicateSuccessful(false) {
            RunState::NotRun
        } else {
            run_state
        }
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
        let store_run_state = if let Some((last, last_run_state)) = self.history.last() {
            assert!(last < &self.current);

            // We always need to store `PredicateSuccessful(false)` as that is used to
            // indicate the start of a gap of `NotRun`.
            *last_run_state == RunState::PredicateSuccessful(false) || *last_run_state != run_state
        } else {
            true
        };

        if store_run_state {
            self.history.push((self.current.clone(), run_state));
        }

        self.last_completed = Some(self.current.clone());
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
