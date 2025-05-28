use futures_signals::signal_vec::{MutableVec, SignalVec, SignalVecExt};

pub struct CallTree {
    children: MutableVec<Node>,
}

impl CallTree {
    pub fn empty() -> Self {
        Self {
            children: MutableVec::new(),
        }
    }

    // TODO: pub(crate)
    // TODO: Need to think about how to track the currently running node. Probably a
    // clone of `children` + an index.
    pub fn fn_begin(&self, name: String) {
        self.children.lock_mut().push_cloned(Node {
            name,
            status: Status::Running,
            sub_tree: Self::empty(),
        })
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

#[derive(Copy, Clone)]
pub enum Status {
    NotRun,
    Running,
    Complete,
}
