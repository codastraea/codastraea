#![cfg(target_arch = "wasm32")]

// "env" is the default anyway.
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn host_func(param: i32);
}

#[unsafe(no_mangle)]
pub extern "C" fn hello() {
    // Release mode builds will optimize this out, so use debug mode to see it's
    // effects
    let _ = Box::leak(Box::new(1));
    unsafe {
        host_func(1);
    }
}
