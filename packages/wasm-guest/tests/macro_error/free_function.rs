use serpent_automation_wasm_guest::workflow;

pub struct X;

impl X {
    #[workflow]
    async fn counter(&self) {}
}

fn main() {}
