use serpent_automation_wasm_guest::workflow;

#[workflow]
async fn counter<const X: usize>() {}

fn main() {}
