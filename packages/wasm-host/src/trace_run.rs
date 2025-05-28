use futures_signals::signal_vec::{MutableVec, SignalVec, SignalVecExt};

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
            status: Status::Running,
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
        let index = nodes
            .len()
            .checked_sub(1)
            .expect("There should be a node on the call stack");
        let mut current = nodes[index].clone();
        assert_eq!(&current.name, name);
        assert_eq!(current.status, Status::Running);
        current.status = Status::Complete;
        nodes.set_cloned(index, current);
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
        if let Some((head, tail)) = path.split_first() {
            self.children.lock_ref()[*head].sub_tree.watch(tail)
        } else {
            self.children.signal_vec_cloned().map(|node| NodeUpdate {
                name: node.name,
                status: node.status,
                has_children: node.sub_tree.children.lock_ref().is_empty(),
            })
        }
    }
}

struct Node {
    // TODO: This should be `node_type : Call name | If | Condition | Then | Else | ...`
    name: String,
    status: Status,
    sub_tree: CallTree,
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            status: self.status,
            sub_tree: CallTree {
                children: self.sub_tree.children.clone(),
            },
        }
    }
}

// TODO: This could be more efficient, as we mostly update `status`. `name`
// never changes, so only needs to be sent when we add a node. Maybe it should
// be an enum of `Status | All`
#[derive(Clone)]
pub struct NodeUpdate {
    pub name: String,
    pub status: Status,
    pub has_children: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Status {
    NotRun,
    Running,
    Complete,
}
