use serpent_automation_wasm_guest::{checkpoint, log, workflow};

fn condition() -> bool {
    log("condition");
    true
}

// TODO: If we change the order of definition of these functions, the server
// seems to deadlock.
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
