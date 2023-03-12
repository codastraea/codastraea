pub struct CallTree {}

impl CallTree {
    pub fn root() -> Self {
        Self {}
    }

    pub fn children(&self) -> &[CallTreeNode] {
        todo!()
    }
}

pub struct If {}

pub enum CallTreeNode {
    CallTree(CallTree),
    If(If)
}