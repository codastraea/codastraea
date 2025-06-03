// TODO: Tidy this file
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use futures_channel::mpsc;
use futures_core::Stream;
use serpent_automation_server_api::{NewNode, NodeStatus, NodeVecDiff};

pub struct Thread {
    call_tree: CallTree,
    call_stack: Vec<StackFrame>,
}

// TODO: We should never panic with dodgy WASM
impl Thread {
    pub fn empty() -> Self {
        let call_tree = CallTree::empty();
        let call_stack = vec![StackFrame::new(call_tree.children.clone())];
        Self {
            call_tree,
            call_stack,
        }
    }

    pub fn call_tree(&self) -> &CallTree {
        &self.call_tree
    }

    pub fn fn_begin(&mut self, name: &str) {
        let top = self.top_mut();
        let new_top = NodeVec::default();
        let node = Node {
            name: name.to_string(),
            status: NodeStatus::Running,
            sub_tree: CallTree {
                children: new_top.clone(),
            },
        };
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
pub struct CallTree {
    children: NodeVec,
}

impl CallTree {
    pub fn empty() -> Self {
        Self {
            children: NodeVec::default(),
        }
    }

    pub fn watch(&self, path: &[usize]) -> impl Stream<Item = NodeVecDiff> {
        // TODO: What if `path` doesn't exist?
        if let Some((head, tail)) = path.split_first() {
            self.children.read().values[*head].sub_tree.watch(tail)
        } else {
            self.children.watch()
        }
    }
}

#[derive(Clone)]
struct Node {
    // TODO: This should be `node_type : Call name | If | Condition | Then | Else | ...`
    name: String,
    status: NodeStatus,
    sub_tree: CallTree,
}

impl<'a> From<&'a Node> for NewNode {
    fn from(value: &'a Node) -> Self {
        let has_children = !value.sub_tree.children.read().values.is_empty();
        Self {
            name: value.name.clone(),
            status: value.status,
            has_children,
        }
    }
}
