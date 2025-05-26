use serpent_automation_wasm_guest::workflow;

pub struct X;

impl X {
    #[workflow]
    fn counter<'a, const X: usize, T>(&self, x: usize) {}
}

fn main() {}
