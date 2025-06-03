use arpy::{FnSubscription, MsgId};
use serde::{Deserialize, Serialize};

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
    type Item = NodeVecDiff;
    type Update = ();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewNode {
    pub name: String,
    pub status: NodeStatus,
    pub has_children: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NodeVecDiff {
    Replace(Vec<NewNode>),
    Push(NewNode),
    SetStatus { index: usize, status: NodeStatus },
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum NodeStatus {
    NotRun,
    Running,
    Complete,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SrcSpan {
    line: usize,
    column: usize,
    len: usize,
}

impl SrcSpan {
    pub fn start() -> Self {
        Self {
            line: 1,
            column: 1,
            len: 0,
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
