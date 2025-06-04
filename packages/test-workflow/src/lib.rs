use serpent_automation_wasm_guest::{checkpoint, log, workflow};

fn condition() -> bool {
    log("condition");
    true
}

// TODO: How do we select which workflow to run? It currently just selects the
// first registered workflow (which seems to be the last defined workflow)
#[workflow]
async fn grandchild_fn() {}

#[workflow]
async fn child_fn() {
    grandchild_fn().await;
    grandchild_fn().await;
}

#[workflow]
async fn counter() {
    if condition() {
        if !condition() {
            log("false");
        } else if condition() {
            log("else if")
        } else {
            log("else");
        }

        log("true");
    }

    for i in 0..10 {
        log(format!("{i}"));
        child_fn().await;
        checkpoint().await;
    }
}
