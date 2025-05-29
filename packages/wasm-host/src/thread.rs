use futures_signals::signal_vec::{MutableVec, SignalVec, SignalVecExt};
use serpent_automation_server_api::{NodeStatus, NodeUpdate};

pub struct Thread {
    call_tree: CallTree,
    call_stack: Vec<StackFrame>,
}

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

    pub fn fn_begin(&mut self, name: String) {
        let top = self.top_mut();
        let new_top = MutableVec::new();
        top.nodes.lock_mut().push_cloned(Node {
            name,
            status: NodeStatus::Running,
            sub_tree: CallTree {
                children: new_top.clone(),
            },
        });
        self.call_stack.push(StackFrame { nodes: new_top })
    }

    // TODO: Doc panic if name != top of stack
    pub fn fn_end(&mut self, name: &str) {
        self.pop();
        let mut nodes = self.top_mut().nodes.lock_mut();
        let last_index = nodes
            .len()
            .checked_sub(1)
            .expect("There should be a node on the call stack");
        let mut current = nodes[last_index].clone();
        // TODO: These should be errors rather than asserts. We shouldn't crash with
        // dodgy wasm.
        assert_eq!(&current.name, name);
        assert_eq!(current.status, NodeStatus::Running);
        current.status = NodeStatus::Complete;
        nodes.set_cloned(last_index, current);
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

struct StackFrame {
    nodes: MutableVec<Node>,
    // TODO: Don't assume the running node is the last one in the list. Add an index.
}

impl StackFrame {
    fn new(nodes: MutableVec<Node>) -> Self {
        Self { nodes }
    }
}

#[derive(Clone)]
pub struct CallTree {
    children: MutableVec<Node>,
}

impl CallTree {
    pub fn empty() -> Self {
        Self {
            children: MutableVec::new(),
        }
    }

    pub fn watch(&self, path: &[usize]) -> impl SignalVec<Item = NodeUpdate> {
        // TODO: What if `path` doesn't exist?
        if let Some((head, tail)) = path.split_first() {
            self.children.lock_ref()[*head].sub_tree.watch(tail)
        } else {
            self.children.signal_vec_cloned().map(|node| NodeUpdate {
                name: node.name,
                status: node.status,
                has_children: !node.sub_tree.children.lock_ref().is_empty(),
            })
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
