use std::{
    fs,
    path::Path,
    sync::{Arc, RwLock},
};

use anyhow::{bail, Context, Result};
use clonelet::clone;
use wasmtime::{Caller, Engine, Extern, Instance, Linker, Module, ModuleExport, Store, TypedFunc};

use crate::{
    instrument::instrument,
    snapshot::Snapshot,
    thread::{NodeStore, Thread},
};

pub struct Container {
    instance: Instance,
    store: Store<()>,
    register_workflows: TypedFunc<(), u32>,
    init_workflow: TypedFunc<u32, ()>,
    run: TypedFunc<(), i32>,
    thread: Arc<RwLock<Thread>>,
}

impl Container {
    pub fn from_file(wat_file: &Path) -> Result<Self> {
        let wat = fs::read(wat_file).context(format!("Opening file {wat_file:?}"))?;
        let wat = instrument(&wat)?;
        let engine = Engine::default();
        let module = Module::new(&engine, wat)?;

        let Some(memory_export) = module.get_export_index("memory") else {
            bail!("failed to find `memory` export in module");
        };
        let linker = &mut Linker::new(&engine);
        define_log(linker, memory_export)?;
        let thread = Arc::new(RwLock::new(Thread::empty()));
        define_trace_fn("begin", Thread::fn_begin, &thread, linker, memory_export)?;
        define_trace_fn("end", Thread::fn_end, &thread, linker, memory_export)?;

        for event in ["if_condition", "else_if_condition", "then", "else"] {
            define_trace(linker, event)?;
        }

        let mut store = Store::new(&engine, ());
        let instance = linker.instantiate(&mut store, &module)?;
        let register_workflows =
            instance.get_typed_func(&mut store, "__enhedron_register_workflows")?;
        let init_workflow = instance.get_typed_func(&mut store, "__enhedron_init_workflow")?;
        let run = instance.get_typed_func(&mut store, "__enhedron_run")?;

        Ok(Self {
            instance,
            store,
            register_workflows,
            init_workflow,
            run,
            thread,
        })
    }

    pub fn snapshot(&mut self) -> Result<Snapshot> {
        Snapshot::new(&mut self.store, &self.instance)
    }

    pub fn restore(&mut self, snapshot: &Snapshot) -> Result<()> {
        snapshot.restore(&mut self.store, &self.instance)
    }

    pub fn register_workflows(&mut self) -> Result<()> {
        let workflow_count = self.register_workflows.call(&mut self.store, ())?;
        println!("Registered {workflow_count} workflows");
        Ok(())
    }

    pub fn init_workflow(&mut self, index: u32) -> Result<()> {
        self.init_workflow.call(&mut self.store, index)?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<bool> {
        Ok(self.run.call(&mut self.store, ())? != 0)
    }

    pub fn node_store(&self) -> NodeStore {
        self.thread.read().unwrap().node_store()
    }
}

fn define_log(linker: &mut Linker<()>, memory_export: ModuleExport) -> Result<()> {
    linker.func_wrap(
        LINKER_MODULE,
        "__enhedron_log",
        move |mut caller: Caller<'_, ()>, data: u32, len: u32| {
            let message = read_string(memory(&mut caller, memory_export)?, data, len)?;
            println!("Log: {message}");
            Ok(())
        },
    )?;
    Ok(())
}

fn define_trace_fn(
    fn_name: &'static str,
    f: impl Fn(&mut Thread, &str) + Send + Sync + 'static,
    thread: &Arc<RwLock<Thread>>,
    linker: &mut Linker<()>,
    memory_export: ModuleExport,
) -> Result<()> {
    clone!(thread);
    linker.func_wrap(
        LINKER_MODULE,
        &format!("__enhedron_fn_{fn_name}"),
        move |mut caller: Caller<()>,
              module_data: u32,
              module_len: u32,
              name_data: u32,
              name_len: u32| {
            let memory = memory(&mut caller, memory_export)?;
            let module = read_string(memory, module_data, module_len)?;
            let name = read_string(memory, name_data, name_len)?;
            println!("{fn_name} {module}::{name}");
            f(&mut thread.write().unwrap(), name);
            Ok(())
        },
    )?;

    Ok(())
}

fn define_trace(linker: &mut Linker<()>, name: &'static str) -> Result<()> {
    for event in ["begin", "end"] {
        let ident = format!("__enhedron_{event}_{name}");

        linker.func_wrap(LINKER_MODULE, &ident, move || println!("{event} {name}"))?;
    }

    Ok(())
}

fn memory<'a>(caller: &'a mut Caller<()>, memory_export: ModuleExport) -> Result<&'a [u8]> {
    let Some(Extern::Memory(memory)) = caller.get_module_export(&memory_export) else {
        bail!("failed to find host memory")
    };
    Ok(memory.data(caller))
}

fn read_string(memory: &[u8], data: u32, len: u32) -> Result<&str> {
    let data: usize = data.try_into().unwrap();
    let len: usize = len.try_into().unwrap();
    let data = memory
        .get(data..)
        .context("`data` out of bounds")?
        .get(..len)
        .context("`len` out of bounds")?;
    let string = str::from_utf8(data).context("Invalid utf-8")?;
    Ok(string)
}

const LINKER_MODULE: &str = "env";
