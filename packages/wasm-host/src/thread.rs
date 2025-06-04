// TODO: Tidy this file
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use futures_channel::mpsc;
use futures_core::Stream;
use serpent_automation_server_api::{
    CallTreeChildNodeId, CallTreeNodeId, NewNode, NodeStatus, NodeVecDiff,
};
use slotmap::SlotMap;

#[derive(Clone)]
pub struct NodeStore(Arc<RwLock<NodeStoreData>>);

impl NodeStore {
    pub fn watch(&self, id: CallTreeNodeId) -> impl Stream<Item = NodeVecDiff> {
        let data = self.0.read().unwrap();

        if let CallTreeNodeId::Child(child_id) = id {
            // TODO: empty stream if not found
            data.children
                .get(child_id)
                .expect("TODO: return empty stream")
                .watch()
        } else {
            data.root.watch()
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

// TODO: We should never panic with dodgy WASM
impl Thread {
    pub fn empty() -> Self {
        let root = NodeVec::default();
        let call_stack = vec![StackFrame::new(root.clone())];
        let node_store = NodeStore::new(root.clone());

        Self {
            call_stack,
            node_store,
        }
    }

    pub fn node_store(&self) -> NodeStore {
        self.node_store.clone()
    }

    pub fn fn_begin(&mut self, name: &str) {
        let new_top = NodeVec::default();
        let id = self.node_store.insert(new_top.clone());
        let node = Node {
            id,
            name: name.to_string(),
            status: NodeStatus::Running,
            sub_tree: new_top.clone(),
        };
        let top = self.top_mut();
        top.nodes.notify(|| NodeVecDiff::Push(NewNode::from(&node)));
        let mut top_children = top.nodes.write();

        let was_empty = top_children.values.is_empty();
        top_children.values.push(node);
        drop(top_children);

        if let Some(top_parent_index) = self.call_stack.len().checked_sub(2) {
            if was_empty {
                // TODO: Don't assume the running node is the last one in the call stack.
                let parent_nodes = &self.call_stack[top_parent_index].nodes;
                let index = parent_nodes
                    .read()
                    .values
                    .len()
                    .checked_sub(1)
                    .expect("Expected at least one running node");

                parent_nodes.notify(|| NodeVecDiff::SetHasChildren { index });
            }
        }
        self.call_stack.push(StackFrame { nodes: new_top })
    }

    pub fn fn_end(&mut self, name: &str) {
        self.pop();
        let nodes = &self.top_mut().nodes;
        let mut write_nodes = nodes.write();
        let current = write_nodes
            .values
            .last_mut()
            .expect("There should be a node on the call stack");
        // TODO: These should be errors rather than asserts. We shouldn't crash with
        // dodgy wasm.
        assert_eq!(current.name, name);
        assert_eq!(current.status, NodeStatus::Running);
        current.status = NodeStatus::Complete;
        let last_index = write_nodes
            .values
            .len()
            .checked_sub(1)
            .expect("There should be a node on the call stack");
        drop(write_nodes);
        nodes.notify(|| NodeVecDiff::SetStatus {
            index: last_index,
            status: NodeStatus::Complete,
        });
    }

    fn top_mut(&mut self) -> &mut StackFrame {
        self.call_stack
            .last_mut()
            .expect("Call stack should never be empty")
    }

    fn pop(&mut self) {
        self.call_stack
            .pop()
            .expect("Call stack should never be empty");
    }
}

#[derive(Default)]
struct NodeVecState {
    values: Vec<Node>,
    watchers: Vec<mpsc::UnboundedSender<NodeVecDiff>>,
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

        if !state.values.is_empty() {
            sender
                .unbounded_send(NodeVecDiff::Replace(
                    state.values.iter().map(NewNode::from).collect(),
                ))
                .unwrap();
        }

        state.watchers.push(sender);
        receiver
    }

    fn notify(&self, mut change: impl FnMut() -> NodeVecDiff) {
        self.write()
            .watchers
            .retain(|watcher| watcher.unbounded_send(change()).is_ok());
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
    // TODO: This should be `node_type : Call name | If | Condition | Then | Else | ...`
    name: String,
    status: NodeStatus,
    sub_tree: NodeVec,
}

impl<'a> From<&'a Node> for NewNode {
    fn from(value: &'a Node) -> Self {
        let has_children = !value.sub_tree.read().values.is_empty();
        Self {
            id: value.id,
            name: value.name.clone(),
            status: value.status,
            has_children,
        }
    }
}
