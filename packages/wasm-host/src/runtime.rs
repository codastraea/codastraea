use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{Arc, RwLock},
};

use anyhow::{Context, Result};
use codastraea_wasm_bindings::host::{Main, MainImports};
use wasmtime::{
    component::{Component, Linker},
    Engine, Store,
};

use crate::{
    instrument::instrument,
    snapshot::Snapshot,
    thread::{NodeStore, Thread},
};

pub struct Container {
    bindings: Main,
    store: Store<State>,
    thread: Arc<RwLock<Thread>>,
    workflow_indices: WorkflowIndices,
}

type WorkflowIndices = Arc<RwLock<HashMap<WorkflowKey, u64>>>;

#[derive(Hash, Eq, PartialEq)]
struct WorkflowKey {
    module: String,
    name: String,
}

struct State {
    thread: Arc<RwLock<Thread>>,
    workflow_indices: WorkflowIndices,
}

impl MainImports for State {
    fn register_workflow_index(&mut self, module: String, name: String, index: u64) {
        println!("Registering workflow index: {module}::{name} = {index}");
        self.workflow_indices
            .write()
            .unwrap()
            .insert(WorkflowKey::new(module, name), index);
    }

    fn begin_fn(&mut self, module: String, name: String) {
        println!("Begin {module}::{name}");
        self.thread.write().unwrap().fn_begin(&name)
    }

    fn end_fn(&mut self, module: String, name: String) {
        println!("End {module}::{name}");
        self.thread.write().unwrap().fn_end(&name)
    }

    fn begin_if_condition(&mut self) {
        println!("begin_if_condition")
    }

    fn end_if_condition(&mut self) {
        println!("end_if_condition")
    }

    fn begin_else_if_condition(&mut self) {
        println!("begin_else_if_condition")
    }

    fn end_else_if_condition(&mut self) {
        println!("end_else_if_condition")
    }

    fn begin_then(&mut self) {
        println!("begin_then")
    }

    fn end_then(&mut self) {
        println!("end_then")
    }

    fn begin_else(&mut self) {
        println!("begin_else")
    }

    fn end_else(&mut self) {
        println!("end_else")
    }

    fn log(&mut self, message: String) {
        println!("{message}");
    }
}

impl WorkflowKey {
    fn new(module: String, name: String) -> Self {
        Self { module, name }
    }
}

impl Container {
    pub fn from_file(wat_file: &Path) -> Result<Self> {
        let wat = fs::read(wat_file).context(format!("Opening file {wat_file:?}"))?;
        let wat = instrument(&wat)?;
        let engine = Engine::default();
        let component = Component::from_binary(&engine, &wat)?;
        let linker = &mut Linker::new(&engine);

        Main::add_to_linker(linker, |state: &mut State| state)?;

        let workflow_indices = WorkflowIndices::default();
        let thread = Arc::new(RwLock::new(Thread::empty()));
        let mut store = Store::new(
            &engine,
            State {
                thread: thread.clone(),
                workflow_indices: workflow_indices.clone(),
            },
        );
        let instance = linker.instantiate(&mut store, &component)?;
        let bindings = Main::instantiate(&mut store, &component, &linker)?;

        Ok(Self {
            bindings,
            store,
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
        let workflow_count = self.bindings.call_register_workflows(&mut self.store)?;
        println!("Registered {workflow_count} workflows");
        Ok(())
    }

    pub fn init_workflow(&mut self, module: &str, name: &str) -> Result<()> {
        let index = *self
            .workflow_indices
            .read()
            .unwrap()
            .get(&WorkflowKey::new(module.to_string(), name.to_string()))
            .with_context(|| format!("Unknown workflow {module}::{name}"))?;
        self.bindings
            .call_initialize_workflow(&mut self.store, index)?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<bool> {
        Ok(self.bindings.call_run_workflow(&mut self.store)?)
    }

    pub fn node_store(&self) -> NodeStore {
        self.thread.read().unwrap().node_store()
    }
}

const LINKER_MODULE: &str = "env";
