use arpy::{FnSubscription, MsgId};
use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum CallTreeNodeId {
    Root,
    Child(CallTreeChildNodeId),
}

new_key_type! {pub struct CallTreeChildNodeId;}

#[derive(MsgId, Serialize, Deserialize, Debug)]
pub struct WatchCallTree {
    node_id: CallTreeNodeId,
}

impl WatchCallTree {
    pub fn root() -> Self {
        Self {
            node_id: CallTreeNodeId::Root,
        }
    }

    pub fn node(node_id: CallTreeChildNodeId) -> Self {
        Self {
            node_id: CallTreeNodeId::Child(node_id),
        }
    }

    pub fn id(&self) -> CallTreeNodeId {
        self.node_id
    }
}

impl FnSubscription for WatchCallTree {
    type InitialReply = ();
    type Item = NodeVecDiff;
    type Update = ();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewNode {
    pub id: CallTreeChildNodeId,
    pub typ: NodeType,
    pub status: NodeStatus,
    pub has_children: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum NodeType {
    Call { name: String },
    If,
    Condition,
    Then,
    ElseIf,
    Else,
}

impl NodeType {
    pub fn as_snake_str(&self) -> &str {
        match self {
            Self::Call { name } => name,
            Self::If => "if",
            Self::Condition => "condition",
            Self::Then => "then",
            Self::ElseIf => "else_if",
            Self::Else => "else",
        }
    }

    pub fn as_display_name(&self) -> &str {
        match self {
            Self::Call { name } => name,
            Self::If => "if",
            Self::Condition => "condition",
            Self::Then => "then",
            Self::ElseIf => "else if",
            Self::Else => "else",
        }
    }

    pub fn is_control_flow(&self) -> bool {
        match self {
            Self::Call { .. } | Self::Condition | Self::Then => false,
            Self::If | Self::ElseIf | Self::Else => true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NodeVecDiff {
    Replace(Vec<NewNode>),
    Push(NewNode),
    SetStatus { index: usize, status: NodeStatus },
    SetHasChildren { index: usize },
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
