use arpy::{FnSubscription, MsgId};
use futures_signals::signal_vec::VecDiff;
use serde::{Deserialize, Serialize};
use serpent_automation_executor::run::{CallStack, RunState};
// TODO: Put `NodeUpdate` in this crate so frontend doesn't need to depend on
// `wasm-host`

#[derive(MsgId, Serialize, Deserialize, Debug)]
pub struct ThreadSubscription;

impl FnSubscription for ThreadSubscription {
    type InitialReply = ();
    type Item = (CallStack, RunState);
    type Update = CallStack;
}

#[derive(MsgId, Serialize, Deserialize, Debug)]
pub struct WatchCallTree {
    path: Vec<usize>,
}

impl WatchCallTree {
    pub fn root() -> Self {
        Self::node(Vec::new())
    }

    pub fn node(path: Vec<usize>) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &[usize] {
        &self.path
    }
}

impl FnSubscription for WatchCallTree {
    type InitialReply = ();
    type Item = VecDiff<NodeUpdate>;
    type Update = ();
}

// TODO: This could be more efficient, as we mostly update `status`. `name`
// never changes, so only needs to be sent when we add a node. Maybe it should
// be an enum of `Status | All`. We need to use a stream of updates rather than
// relying on `SignalVec`.
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct NodeUpdate {
    pub name: String,
    pub status: NodeStatus,
    pub has_children: bool,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum NodeStatus {
    NotRun,
    Running,
    Complete,
}
