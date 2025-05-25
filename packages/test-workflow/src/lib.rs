// TODO: MacOS and Windows don't like undefined symbols with the default linker
// flags. Need to find a better way around this.
#![cfg(not(any(target_os = "macos", target_os = "windows")))]

use serpent_automation_wasm_guest::{checkpoint, log, workflow};

fn condition() -> bool {
    log("condition");
    true
}

#[workflow]
async fn counter() {
    if condition() {
        log("true");
    }

    for i in 0..10 {
        log(format!("{i}"));
        checkpoint().await;
    }
}
