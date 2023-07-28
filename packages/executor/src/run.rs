use std::{
    cmp::Ordering,
    collections::HashSet,
    pin::pin,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use futures::Stream;
use serde::{Deserialize, Serialize};
// TODO: Split this out into a separate crate (or put it in the server crate)?
#[cfg(not(target_arch = "wasm32"))]
use tokio::spawn;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

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

    // TODO: We need a "slice" for CallStacks
    pub fn parent(&self) -> Option<CallStack> {
        let mut parent = self.0.clone();

        parent.pop()?;

        while let Some(top) = parent.last() {
            match top {
                StackFrame::Call(_) | StackFrame::NestedBlock(_, NestedBlock::Predicate) => {
                    return Some(Self(parent))
                }
                _ => (),
            }

            parent.pop();
        }

        Some(Self(parent))
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

#[derive(Clone)]
pub struct ThreadRunState(Arc<RwLock<SharedThreadRunState>>);

impl Default for ThreadRunState {
    fn default() -> Self {
        let (update_sender, _update_receiver) = broadcast::channel(1000);

        Self(Arc::new(RwLock::new(SharedThreadRunState {
            history: Vec::new(),
            last_completed: None,
            current: CallStack::new(),
            update_sender,
        })))
    }
}

struct SharedThreadRunState {
    history: Vec<(CallStack, RunState)>,
    last_completed: Option<CallStack>,
    current: CallStack,
    update_sender: broadcast::Sender<(CallStack, RunState)>,
}

impl SharedThreadRunState {
    fn update(&self, call_stack: CallStack, run_state: RunState) {
        // TODO: I think we want to ignore errors. What happens to the queue?
        let _ = self.update_sender.send((call_stack, run_state));
    }
}

impl ThreadRunState {
    pub fn run_state(&self, stack: &CallStack) -> RunState {
        let data = self.read();

        if data.current.starts_with(stack) {
            return RunState::Running;
        }

        if Some(stack) > data.last_completed.as_ref() {
            return RunState::NotRun;
        }

        let insert_index = match data
            .history
            .binary_search_by_key(&stack, |(call_stack, _)| call_stack)
        {
            Ok(match_index) => return data.history[match_index].1,
            Err(insert_index) => insert_index,
        };

        if insert_index == 0 {
            return RunState::NotRun;
        }

        let run_state = data.history[insert_index - 1].1;

        if run_state == RunState::PredicateSuccessful(false) {
            RunState::NotRun
        } else {
            run_state
        }
    }

    pub fn push(&self, item: StackFrame) {
        let mut data = self.write();
        data.current.push(item);
        data.update(data.current.clone(), RunState::Running);
    }

    pub fn pop_success(&self) {
        self.pop(RunState::Successful);
    }

    pub fn pop_failed(&self) {
        self.pop(RunState::Failed);
    }

    pub fn pop_predicate_success(&self, result: bool) {
        self.pop(RunState::PredicateSuccessful(result));
    }

    fn pop(&self, run_state: RunState) {
        let mut data = self.write();
        let store_run_state = if let Some((last, last_run_state)) = data.history.last() {
            assert!(last < &data.current);

            // We always need to store `PredicateSuccessful(false)` as that is used to
            // indicate the start of a gap of `NotRun`.
            *last_run_state == RunState::PredicateSuccessful(false) || *last_run_state != run_state
        } else {
            true
        };

        if store_run_state {
            let current = data.current.clone();
            data.history.push((current, run_state));
        }

        data.last_completed = Some(data.current.clone());
        data.update(data.current.clone(), run_state);
        data.current.pop();
    }

    // TODO: Split this out into a separate crate (or put it in the server crate)?
    #[cfg(not(target_arch = "wasm32"))]
    pub fn subscribe(
        &self,
        open_nodes: impl Stream<Item = CallStack> + Send + 'static,
    ) -> mpsc::Receiver<(CallStack, RunState)> {
        use futures::stream;

        let run_state_updates = BroadcastStream::new(self.read().update_sender.subscribe())
            .map_while(Result::ok)
            .map(|(call_stack, run_state)| UpdateClient::UpdateRunState(call_stack, run_state));
        let open_nodes = open_nodes.map(UpdateClient::OpenNode);

        let updates = stream::select(run_state_updates, open_nodes);

        // TODO: Channel bounds
        let (run_state_sender, run_state_receiver) = mpsc::channel(1000);

        spawn(Self::update_client(run_state_sender, updates));

        run_state_receiver
    }

    async fn update_client(
        send_run_state: mpsc::Sender<(CallStack, RunState)>,
        updates: impl Stream<Item = UpdateClient>,
    ) {
        let mut open_nodes = HashSet::new();
        let mut updates = pin!(updates);

        while let Some(update) = updates.next().await {
            match update {
                UpdateClient::UpdateRunState(call_stack, run_state) => {
                    if let Some(parent) = call_stack.parent() {
                        if open_nodes.contains(&parent) {
                            send_run_state
                                .send((call_stack.clone(), run_state))
                                .await
                                .unwrap();
                        }
                    }
                }
                UpdateClient::OpenNode(call_stack) => {
                    println!("Opening node {call_stack:?}");
                    open_nodes.insert(call_stack);
                    // TODO(next): Find all child node states and send them
                }
            }
        }
    }

    fn read(&self) -> RwLockReadGuard<'_, SharedThreadRunState> {
        self.0.read().unwrap()
    }

    fn write(&self) -> RwLockWriteGuard<'_, SharedThreadRunState> {
        self.0.write().unwrap()
    }
}

#[derive(Clone)]
enum UpdateClient {
    OpenNode(CallStack),
    UpdateRunState(CallStack, RunState),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum RunState {
    NotRun,
    Running,
    Successful,
    PredicateSuccessful(bool),
    Failed,
}
