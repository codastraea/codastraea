#![cfg(target_arch = "wasm32")]
use std::ptr;

// "env" is the default anyway.
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn host_func(param: i32);
}

#[unsafe(no_mangle)]
pub extern "C" fn hello() {
    unsafe {
        host_func(ptr::from_ref(Box::leak(Box::new(1))) as i32);
    }
}
