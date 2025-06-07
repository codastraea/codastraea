use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use anyhow::{ensure, Context, Result};
use codastraea_server_api::{
    CallTreeChildNodeId, CallTreeNodeId, NewNode, NodeStatus, NodeType, NodeVecDiff,
};
use futures::{
    stream::{self, BoxStream},
    Stream,
};
use futures_channel::mpsc;
use slotmap::SlotMap;

#[derive(Clone)]
pub struct NodeStore(Arc<RwLock<NodeStoreData>>);

impl NodeStore {
    /// Watch a group of nodes.
    ///
    /// An empty stream is returned if `id` is not found.
    pub fn watch(&self, id: CallTreeNodeId) -> BoxStream<'static, NodeVecDiff> {
        let data = self.0.read().unwrap();

        if let CallTreeNodeId::Child(child_id) = id {
            if let Some(child) = data.children.get(child_id) {
                Box::pin(child.watch())
            } else {
                Box::pin(stream::empty())
            }
        } else {
            Box::pin(data.root.watch())
        }
    }

    fn new(root: NodeVec) -> Self {
        Self(Arc::new(RwLock::new(NodeStoreData {
            root,
            children: SlotMap::with_key(),
        })))
    }

    fn insert(&self, nodes: NodeVec) -> CallTreeChildNodeId {
        self.0.write().unwrap().children.insert(nodes)
    }
}

struct NodeStoreData {
    root: NodeVec,
    children: SlotMap<CallTreeChildNodeId, NodeVec>,
}

pub struct Thread {
    call_stack: Vec<StackFrame>,
    node_store: NodeStore,
}

impl Thread {
    pub fn empty() -> Self {
        let root = NodeVec::default();
        let call_stack = vec![StackFrame::new(root.clone())];
        let node_store = NodeStore::new(root);

        Self {
            call_stack,
            node_store,
        }
    }

    pub fn node_store(&self) -> NodeStore {
        self.node_store.clone()
    }

    // TODO: Error handling for `fn_begin` and `fn_end`. We should really display an
    // error node in the call tree.
    pub fn begin(&mut self, typ: &NodeType) {
        self.try_begin(typ).expect("TODO: Handle errors")
    }

    pub fn end(&mut self, typ: &NodeType) {
        self.try_end(typ).expect("TODO: Handle errors")
    }

    fn try_begin(&mut self, typ: &NodeType) -> Result<()> {
        let new_top = NodeVec::default();
        let id = self.node_store.insert(new_top.clone());
        let node = Node {
            id,
            typ: typ.clone(),
            status: NodeStatus::Running,
            sub_tree: new_top.clone(),
        };
        let top = self.top_mut()?;
        let mut top_children = top.nodes.write();
        let was_empty = top_children.is_empty();
        top_children.push(node);
        drop(top_children);

        if let Some(top_parent_index) = self.call_stack.len().checked_sub(2) {
            if was_empty {
                // TODO: Don't assume the running node is the last one in the call stack.
                let parent_nodes = &self.call_stack[top_parent_index].nodes;
                let index = parent_nodes
                    .read()
                    .len()
                    .checked_sub(1)
                    .context("Expected at least one running node")?;

                parent_nodes
                    .write()
                    .notify(|| NodeVecDiff::SetHasChildren { index });
            }
        }

        self.call_stack.push(StackFrame { nodes: new_top });
        Ok(())
    }

    fn try_end(&mut self, typ: &NodeType) -> Result<()> {
        self.pop()?;
        let mut nodes = self.top_mut()?.nodes.write();
        let current = nodes
            .values
            .last_mut()
            .context("There should be a node on the call stack")?;
        ensure!(&current.typ == typ);
        ensure!(current.status == NodeStatus::Running);
        current.status = NodeStatus::Complete;
        let last_index = nodes
            .len()
            .checked_sub(1)
            .context("There should be a node on the call stack")?;
        nodes.notify(|| NodeVecDiff::SetStatus {
            index: last_index,
            status: NodeStatus::Complete,
        });
        Ok(())
    }

    fn top_mut(&mut self) -> Result<&mut StackFrame> {
        self.call_stack
            .last_mut()
            .context("Call stack should never be empty")
    }

    fn pop(&mut self) -> Result<()> {
        self.call_stack
            .pop()
            .context("Call stack should never be empty")?;
        Ok(())
    }
}

#[derive(Default)]
struct NodeVecState {
    values: Vec<Node>,
    watchers: Vec<mpsc::UnboundedSender<NodeVecDiff>>,
}

impl NodeVecState {
    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn push(&mut self, node: Node) {
        self.notify(|| NodeVecDiff::Push(NewNode::from(&node)));
        self.values.push(node);
    }

    fn notify(&mut self, mut change: impl FnMut() -> NodeVecDiff) {
        self.watchers
            .retain(|watcher| watcher.unbounded_send(change()).is_ok());
    }
}

#[derive(Default, Clone)]
struct NodeVec(Arc<RwLock<NodeVecState>>);

impl NodeVec {
    fn read(&self) -> RwLockReadGuard<NodeVecState> {
        self.0.read().unwrap()
    }

    fn write(&self) -> RwLockWriteGuard<NodeVecState> {
        self.0.write().unwrap()
    }

    fn watch(&self) -> impl Stream<Item = NodeVecDiff> {
        let (sender, receiver) = mpsc::unbounded();
        let mut state = self.write();

        if !state.is_empty() {
            sender
                .unbounded_send(NodeVecDiff::Replace(
                    state.values.iter().map(NewNode::from).collect(),
                ))
                .unwrap();
        }

        state.watchers.push(sender);
        receiver
    }
}

struct StackFrame {
    nodes: NodeVec,
    // TODO: Don't assume the running node is the last one in the list. Add an index.
}

impl StackFrame {
    fn new(nodes: NodeVec) -> Self {
        Self { nodes }
    }
}

#[derive(Clone)]
struct Node {
    id: CallTreeChildNodeId,
    typ: NodeType,
    status: NodeStatus,
    sub_tree: NodeVec,
}

impl<'a> From<&'a Node> for NewNode {
    fn from(value: &'a Node) -> Self {
        let has_children = !value.sub_tree.read().is_empty();
        Self {
            id: value.id,
            typ: value.typ.clone(),
            status: value.status,
            has_children,
        }
    }
}
