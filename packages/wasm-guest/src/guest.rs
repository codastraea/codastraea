use std::{cell::RefCell, future::Future, pin::Pin};

use crate::{
    checkpoint::{checkpoint, until_checkpoint},
    log,
};

async fn counter() {
    log("Starting counting");

    for i in 0..10 {
        log(format!("{i}"));
        checkpoint().await;
    }

    log("Finished counting");
}

#[no_mangle]
pub extern "C" fn __enhedron_run() -> i32 {
    __ENHEDRON_MAIN.with_borrow_mut(|f| match until_checkpoint(f.as_mut()) {
        Some(_) => 0,
        None => 1,
    })
}

thread_local! {
    static __ENHEDRON_MAIN: RefCell<Pin<Box<dyn Future<Output = ()>>>> = RefCell::new(Box::pin(counter()));
}
