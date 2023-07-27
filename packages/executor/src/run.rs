use std::{
    cmp::{min, Ordering},
    collections::HashSet,
    pin::pin,
    sync::{Arc, RwLock},
};

use futures::{Stream, StreamExt};
use futures_signals::signal_map::{MutableBTreeMap, SignalMap};
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};
use tokio::{spawn, sync::mpsc};

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

    // TODO: We need a "slice" for CallStacks
    pub fn parent(&self) -> Option<CallStack> {
        let mut parent = self.0.clone();
        parent.pop().map(|_| Self(parent))
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

#[derive(Default)]
pub struct ThreadRunState {
    history: Vec<(CallStack, RunState)>,
    last_completed: Option<CallStack>,
    current: CallStack,
    updater: ThreadRunStateUpdater,
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
        self.updater.update(self.current.clone(), RunState::Running);
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
        self.updater.update(self.current.clone(), run_state);
        self.current.pop();
    }
}

new_key_type! {struct ClientId; }

struct ThreadRunStateUpdater {
    clients: Arc<RwLock<SlotMap<ClientId, Arc<Client>>>>,
    update_receiver: mpsc::UnboundedReceiver<(CallStack, RunState)>,
    update_sender: mpsc::UnboundedSender<(CallStack, RunState)>,
}

impl Default for ThreadRunStateUpdater {
    fn default() -> Self {
        // TODO: Use bounded channel?
        let (update_sender, update_receiver) = mpsc::unbounded_channel();

        Self {
            clients: Default::default(),
            update_receiver,
            update_sender,
        }
    }
}

impl ThreadRunStateUpdater {
    pub fn update(&self, call_stack: CallStack, run_state: RunState) {
        self.update_sender.send((call_stack, run_state)).unwrap()
    }

    pub async fn update_clients(&mut self) {
        // TODO: Update receiver should receive new subscriptions and updates, so we
        // don't have any ordering problems. It should put new subscriptions in the map
        // and send the initial values.
        while let Some((call_stack, run_state)) = self.update_receiver.recv().await {
            if let Some(parent) = call_stack.parent() {
                for client in self.clients.read().unwrap().values() {
                    if client.open_nodes.read().unwrap().contains(&parent) {
                        client
                            .view_nodes
                            .lock_mut()
                            .insert_cloned(call_stack.clone(), run_state);
                    }
                }
            }
        }
    }

    pub fn subscribe(
        &mut self,
        open_nodes: impl Stream<Item = CallStack> + Send + 'static,
    ) -> impl SignalMap<Key = CallStack, Value = RunState> {
        let client = Arc::new(Client::default());
        let id = self.clients.write().unwrap().insert(client.clone());

        spawn({
            // TODO: Use clone! from silkenweb
            let client = client.clone();
            let clients = self.clients.clone();

            async move {
                let mut open_nodes = pin!(open_nodes);

                while let Some(node) = open_nodes.next().await {
                    // TODO: Don't write to open nodes here. Just send a message to `update_clients`
                    // saying we're interested.
                    client.open_nodes.write().unwrap().insert(node);
                }

                clients.write().unwrap().remove(id);
            }
        });

        client.view_nodes.signal_map_cloned()
    }
}

#[derive(Default)]
struct Client {
    open_nodes: RwLock<HashSet<CallStack>>,
    view_nodes: MutableBTreeMap<CallStack, RunState>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum RunState {
    NotRun,
    Running,
    Successful,
    PredicateSuccessful(bool),
    Failed,
}
