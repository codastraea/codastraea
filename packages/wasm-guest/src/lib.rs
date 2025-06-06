use std::{cell::RefCell, future::Future, pin::Pin};

use checkpoint::until_checkpoint;

mod checkpoint;

pub use checkpoint::checkpoint;
/// Make a Workflow function.
///
/// This instruments a function to trace any control flow, so it can be used as
/// a workflow function.
///
/// *Workflow functions must be `async`, parameter-less and free (not inside an
/// `impl` block).*
pub use codastraea_wasm_guest_proc_macro::workflow;
#[doc(hidden)]
pub use inventory;

mod host;

#[doc(hidden)]
pub struct TraceFn {
    module: &'static str,
    name: &'static str,
}

impl TraceFn {
    pub fn new(module: &'static str, name: &'static str) -> Self {
        unsafe {
            host::__codastraea_fn_begin(
                wasm_ptr(module),
                wasm_len(module),
                wasm_ptr(name),
                wasm_len(name),
            )
        }

        Self { module, name }
    }
}

impl Drop for TraceFn {
    fn drop(&mut self) {
        unsafe {
            host::__codastraea_fn_end(
                wasm_ptr(self.module),
                wasm_len(self.module),
                wasm_ptr(self.name),
                wasm_len(self.name),
            )
        }
    }
}

#[doc(hidden)]
pub struct OnDrop<F: Fn()> {
    end: F,
}

impl<F: Fn()> Drop for OnDrop<F> {
    fn drop(&mut self) {
        (self.end)()
    }
}

impl<F: Fn()> OnDrop<F> {
    pub fn new(end: F) -> Self {
        Self { end }
    }
}

pub fn log(s: impl AsRef<str>) {
    let s = s.as_ref();
    unsafe { host::__codastraea_log(wasm_ptr(s), wasm_len(s)) };
}

fn wasm_ptr(s: &str) -> u32 {
    (s.as_ptr() as usize).try_into().unwrap()
}

fn wasm_len(s: &str) -> u32 {
    s.len().try_into().unwrap()
}

#[doc(hidden)]
pub struct Workflow {
    module: &'static str,
    name: &'static str,
    init: fn(),
}

inventory::collect!(Workflow);

impl Workflow {
    pub const fn new(module: &'static str, name: &'static str, init: fn()) -> Self {
        Self { module, name, init }
    }
}

#[no_mangle]
extern "C" fn __codastraea_register_workflows() -> u32 {
    log("Registering workflows");

    WORKFLOWS.with_borrow_mut(|workflows| {
        for (index, Workflow { module, name, init }) in
            inventory::iter::<Workflow>.into_iter().enumerate()
        {
            unsafe {
                host::__codastraea_register_workflow_index(
                    wasm_ptr(module),
                    wasm_len(module),
                    wasm_ptr(name),
                    wasm_len(name),
                    index.try_into().unwrap(),
                )
            }
            workflows.push(*init);
        }

        workflows.len().try_into().unwrap()
    })
}

#[no_mangle]
extern "C" fn __codastraea_init_workflow(index: u32) {
    let index = usize::try_from(index).unwrap();
    WORKFLOWS.with_borrow(|workflows| workflows[index]())
}

#[no_mangle]
extern "C" fn __codastraea_run() -> i32 {
    MAIN.with_borrow_mut(|f| match until_checkpoint(f.as_mut()) {
        Some(_) => 0,
        None => 1,
    })
}

#[doc(hidden)]
pub fn set_main_fn(f: impl Future<Output = ()> + 'static) {
    MAIN.set(Box::pin(f));
}

async fn noop() {}

thread_local! {
    static MAIN: RefCell<Pin<Box<dyn Future<Output = ()>>>> = RefCell::new(Box::pin(noop()));
    static WORKFLOWS: RefCell<Vec<fn()>> = const { RefCell::new(Vec::new()) };
}
