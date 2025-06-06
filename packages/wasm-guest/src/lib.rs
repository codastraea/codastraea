use std::{cell::RefCell, future::Future, pin::Pin};

use checkpoint::until_checkpoint;

mod checkpoint;

pub use checkpoint::checkpoint;
pub use codastraea_wasm_bindings::guest;
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

#[doc(hidden)]
pub struct TraceFn {
    module: &'static str,
    name: &'static str,
}

impl TraceFn {
    pub fn new(module: &'static str, name: &'static str) -> Self {
        guest::begin_fn(module, name);
        Self { module, name }
    }
}

impl Drop for TraceFn {
    fn drop(&mut self) {
        guest::end_fn(self.module, self.name)
    }
}

type CFunction = unsafe extern "C" fn();

#[doc(hidden)]
pub struct Trace {
    end: fn(),
}

impl Drop for Trace {
    fn drop(&mut self) {
        (self.end)()
    }
}

impl Trace {
    pub fn new(begin: impl FnOnce(), end: fn()) -> Self {
        begin();
        Self { end }
    }
}

#[doc(hidden)]
pub struct Workflow {
    module: &'static str,
    name: &'static str,
    init: CFunction,
}

inventory::collect!(Workflow);

impl Workflow {
    pub const fn new(module: &'static str, name: &'static str, init: CFunction) -> Self {
        Self { module, name, init }
    }
}

#[no_mangle]
extern "C" fn __enhedron_register_workflows() -> u32 {
    guest::log("Registering workflows");

    WORKFLOWS.with_borrow_mut(|workflows| {
        for (index, Workflow { module, name, init }) in
            inventory::iter::<Workflow>.into_iter().enumerate()
        {
            guest::register_workflow_index(
                module,
                name,
                index.try_into().expect("Index should convert to u64"),
            );
            workflows.push(*init);
        }

        workflows.len().try_into().unwrap()
    })
}

#[no_mangle]
extern "C" fn __codastraea_init_workflow(index: u32) {
    let index = usize::try_from(index).unwrap();
    WORKFLOWS.with_borrow(|workflows| unsafe { workflows[index]() })
}

#[doc(hidden)]
pub fn set_fn(f: impl Future<Output = ()> + 'static) {
    MAIN.set(Box::pin(f));
}

#[no_mangle]
extern "C" fn __codastraea_run() -> i32 {
    MAIN.with_borrow_mut(|f| match until_checkpoint(f.as_mut()) {
        Some(_) => 0,
        None => 1,
    })
}

async fn noop() {}

thread_local! {
    static MAIN: RefCell<Pin<Box<dyn Future<Output = ()>>>> = RefCell::new(Box::pin(noop()));
    static WORKFLOWS: RefCell<Vec<CFunction>> = const { RefCell::new(Vec::new()) };
}
