pub mod checkpoint;
pub use serpent_automation_wasm_guest_proc_macro::workflow;

unsafe extern "C" {
    fn __enhedron_begin_fn(module: u32, module_len: u32, name: u32, name_len: u32);
    fn __enhedron_end_fn(module: u32, module_len: u32, name: u32, name_len: u32);
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
            __enhedron_begin_fn(
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
            __enhedron_end_fn(
                wasm_ptr(self.module),
                wasm_len(self.module),
                wasm_ptr(self.name),
                wasm_len(self.name),
            )
        }
    }
}

type CTrace = unsafe extern "C" fn();

#[doc(hidden)]
pub struct Trace {
    end: CTrace,
}

impl Drop for Trace {
    fn drop(&mut self) {
        unsafe { (self.end)() }
    }
}

impl Trace {
    pub fn new(begin: CTrace, end: CTrace) -> Self {
        unsafe { begin() }
        Self { end }
    }
}

extern "C" {
    fn __enhedron_log(data: u32, len: u32);
}

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
