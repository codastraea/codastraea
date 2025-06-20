use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Arc, RwLock},
};

use anyhow::{bail, Context, Result};
use clonelet::clone;
use codastraea_server_api::NodeType;
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
    workflow_indices: WorkflowIndices,
}

type WorkflowIndices = Arc<RwLock<HashMap<WorkflowKey, u32>>>;

#[derive(Hash, Eq, PartialEq)]
struct WorkflowKey {
    module: String,
    name: String,
}

impl WorkflowKey {
    fn new(module: &str, name: &str) -> Self {
        Self {
            module: module.to_string(),
            name: name.to_string(),
        }
    }
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
        let workflow_indices = WorkflowIndices::default();
        define_register_workflow_index(workflow_indices.clone(), linker, memory_export)?;
        define_log(linker, memory_export)?;
        let thread = Arc::new(RwLock::new(Thread::empty()));
        define_trace_fn("begin", Thread::begin, &thread, linker, memory_export)?;
        define_trace_fn("end", Thread::end, &thread, linker, memory_export)?;

        for node_type in [
            NodeType::If,
            NodeType::Condition,
            NodeType::Then,
            NodeType::ElseIf,
            NodeType::Else,
        ] {
            define_trace("begin", Thread::begin, &thread, linker, &node_type)?;
            define_trace("end", Thread::end, &thread, linker, &node_type)?;
        }

        let mut store = Store::new(&engine, ());
        let instance = linker.instantiate(&mut store, &module)?;
        let register_workflows =
            instance.get_typed_func(&mut store, "__codastraea_register_workflows")?;
        let init_workflow = instance.get_typed_func(&mut store, "__codastraea_init_workflow")?;
        let run = instance.get_typed_func(&mut store, "__codastraea_run")?;

        Ok(Self {
            instance,
            store,
            register_workflows,
            init_workflow,
            run,
            thread,
            workflow_indices,
        })
    }

    pub fn snapshot(&mut self) -> Result<Snapshot> {
        // TODO: We also need to snapshot and restore the call stack.
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

    pub fn init_workflow(&mut self, module: &str, name: &str) -> Result<()> {
        let index = *self
            .workflow_indices
            .read()
            .unwrap()
            .get(&WorkflowKey::new(module, name))
            .with_context(|| format!("Unknown workflow {module}::{name}"))?;
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

fn define_register_workflow_index(
    workflow_indices: WorkflowIndices,
    linker: &mut Linker<()>,
    memory_export: ModuleExport,
) -> Result<()> {
    linker.func_wrap(
        LINKER_MODULE,
        "__codastraea_register_workflow_index",
        move |mut caller: Caller<'_, ()>,
              module_data: u32,
              module_len: u32,
              name_data: u32,
              name_len: u32,
              index: u32| {
            let memory = memory(&mut caller, memory_export)?;
            let module = read_string(memory, module_data, module_len)?;
            let name = read_string(memory, name_data, name_len)?;
            println!("Registering workflow index: {module}::{name} = {index}");
            workflow_indices
                .write()
                .unwrap()
                .insert(WorkflowKey::new(module, name), index);
            Ok(())
        },
    )?;
    Ok(())
}

fn define_log(linker: &mut Linker<()>, memory_export: ModuleExport) -> Result<()> {
    linker.func_wrap(
        LINKER_MODULE,
        "__codastraea_log",
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
    f: impl Fn(&mut Thread, &NodeType) + Send + Sync + 'static,
    thread: &Arc<RwLock<Thread>>,
    linker: &mut Linker<()>,
    memory_export: ModuleExport,
) -> Result<()> {
    clone!(thread);
    linker.func_wrap(
        LINKER_MODULE,
        &format!("__codastraea_fn_{fn_name}"),
        move |mut caller: Caller<()>,
              module_data: u32,
              module_len: u32,
              name_data: u32,
              name_len: u32| {
            let memory = memory(&mut caller, memory_export)?;
            let module = read_string(memory, module_data, module_len)?;
            let name = read_string(memory, name_data, name_len)?;
            println!("{fn_name} {module}::{name}");
            f(
                &mut thread.write().unwrap(),
                &NodeType::Call {
                    name: name.to_string(),
                },
            );
            Ok(())
        },
    )?;

    Ok(())
}

fn define_trace(
    event: &'static str,
    f: impl Fn(&mut Thread, &NodeType) + Send + Sync + 'static,
    thread: &Arc<RwLock<Thread>>,
    linker: &mut Linker<()>,
    node_type: &NodeType,
) -> Result<()> {
    let snake_name = node_type.as_snake_str();
    let ident = format!("__codastraea_{event}_{snake_name}");
    clone!(thread, node_type);

    linker.func_wrap(LINKER_MODULE, &ident, move || {
        println!("{event} {}", node_type.as_snake_str());
        f(&mut thread.write().unwrap(), &node_type);
    })?;

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
