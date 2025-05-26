use std::{cell::RefCell, future::Future, pin::Pin};

use checkpoint::until_checkpoint;

mod checkpoint;

pub use checkpoint::checkpoint;
#[doc(hidden)]
pub use inventory;
/// Make a Workflow function.
///
/// This instruments a function to trace any control flow, so it can be used as
/// a workflow function.
///
/// *Workflow functions must be `async`, parameter-less and free (not inside an
/// `impl` block).*
pub use serpent_automation_wasm_guest_proc_macro::workflow;

#[cfg(target_family = "wasm")]
unsafe extern "C" {
    fn __enhedron_fn_begin(module: u32, module_len: u32, name: u32, name_len: u32);
    fn __enhedron_fn_end(module: u32, module_len: u32, name: u32, name_len: u32);
}

#[cfg(not(target_family = "wasm"))]
unsafe extern "C" fn __enhedron_fn_begin(
    _module: u32,
    _module_len: u32,
    _name: u32,
    _name_len: u32,
) {
}
#[cfg(not(target_family = "wasm"))]
unsafe extern "C" fn __enhedron_fn_end(_module: u32, _module_len: u32, _name: u32, _name_len: u32) {
}

#[doc(hidden)]
pub struct TraceFn {
    module: &'static str,
    name: &'static str,
}

impl TraceFn {
    pub fn new(module: &'static str, name: &'static str) -> Self {
        let new = Self { module, name };
        unsafe {
            __enhedron_fn_begin(
                wasm_ptr(new.module),
                wasm_len(new.module),
                wasm_ptr(new.name),
                wasm_len(new.name),
            )
        }
        new
    }
}

impl Drop for TraceFn {
    fn drop(&mut self) {
        unsafe {
            __enhedron_fn_end(
                wasm_ptr(self.module),
                wasm_len(self.module),
                wasm_ptr(self.name),
                wasm_len(self.name),
            )
        }
    }
}

type CFunction = unsafe extern "C" fn();

#[doc(hidden)]
pub struct Trace {
    end: CFunction,
}

impl Drop for Trace {
    fn drop(&mut self) {
        unsafe { (self.end)() }
    }
}

impl Trace {
    pub fn new(begin: CFunction, end: CFunction) -> Self {
        unsafe { begin() }
        Self { end }
    }
}

#[cfg(target_family = "wasm")]
extern "C" {
    fn __enhedron_log(data: u32, len: u32);
}

#[cfg(not(target_family = "wasm"))]
unsafe extern "C" fn __enhedron_log(_data: u32, _len: u32) {}

pub fn log(s: impl AsRef<str>) {
    let s = s.as_ref();
    unsafe { __enhedron_log(wasm_ptr(s), wasm_len(s)) };
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
    log("Registering workflows");

    WORKFLOWS.with_borrow_mut(|workflows| {
        for Workflow { module, name, init } in inventory::iter::<Workflow> {
            log(format!("Registering workflow {module}::{name}"));
            workflows.push(*init);
        }

        workflows.len().try_into().unwrap()
    })
}

#[no_mangle]
extern "C" fn __enhedron_init_workflow(index: u32) {
    let index = usize::try_from(index).unwrap();
    WORKFLOWS.with_borrow(|workflows| unsafe { workflows[index]() })
}

#[doc(hidden)]
pub fn set_fn(f: impl Future<Output = ()> + 'static) {
    MAIN.set(Box::pin(f));
}

#[no_mangle]
extern "C" fn __enhedron_run() -> i32 {
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
