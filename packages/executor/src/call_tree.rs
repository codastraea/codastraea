use std::{cell::RefCell, rc::Rc};

// TODO: Rename `StackFrame` to `Block`?
use crate::run::{RunState, StackFrame as BlockPos};

pub struct CallTree {
    tree: Option<CallTreeNode>,
}

type CallTreeNode = Rc<RefCell<Vec<StackFrame>>>;

pub struct Stack {
    nodes: Vec<CallTreeNode>,
}

impl Stack {
    pub fn push(&mut self, block_pos: BlockPos) {
        let frame = Rc::new(RefCell::new(vec![StackFrame::new(block_pos)]));

        if let Some(top) = self.nodes.last() {
            top.borrow_mut().last_mut().unwrap().child.tree = Some(frame.clone());
        }

        self.nodes.push(frame)
    }

    pub fn pop(&mut self, run_state: RunState) {
        self.nodes
            .pop()
            .unwrap()
            .borrow_mut()
            .last_mut()
            .unwrap()
            .run_state = run_state;
    }

    pub fn into_call_tree(self) -> CallTree {
        CallTree {
            tree: self.nodes.into_iter().next(),
        }
    }
}

struct StackFrame {
    _block_pos: BlockPos,
    child: CallTree,
    run_state: RunState,
}

impl StackFrame {
    pub fn new(block_pos: BlockPos) -> Self {
        Self {
            _block_pos: block_pos,
            child: CallTree { tree: None },
            run_state: RunState::Running,
        }
    }
}
