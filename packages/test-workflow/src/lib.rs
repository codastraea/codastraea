// TODO: MacOS and Windows don't like undefined symbols with the default linker
// flags. Need to find a better way around this.
#![cfg(not(any(target_os = "macos", target_os = "windows")))]

use serpent_automation_wasm_guest::{
    checkpoint::{checkpoint, set_fn},
    log, workflow,
};

#[workflow]
async fn counter() {
    for i in 0..10 {
        log(format!("{i}"));
        checkpoint().await;
    }
}

#[no_mangle]
pub extern "C" fn __enhedron_init_counter() {
    set_fn(counter());
}
