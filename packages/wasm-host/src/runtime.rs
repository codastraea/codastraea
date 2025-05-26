use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use wasmtime::{Caller, Engine, Extern, Instance, Linker, Module, ModuleExport, Store, TypedFunc};

use crate::{instrument::instrument, snapshot::Snapshot};

pub fn run(wat_file: &Path) -> Result<()> {
    let mut container = Container::from_file(wat_file)?;

    container.register_workflows()?;
    container.init_workflow(0)?;

    for _i in 0..5 {
        container.run()?;
        println!("Checkpoint (pre snapshot)");
    }

    let snapshot = container.snapshot()?;

    while container.run()? {
        println!("Checkpoint (post snapshot)");
    }

    drop(container);

    let mut container = Container::from_file(wat_file)?;
    container.restore(&snapshot)?;

    while container.run()? {
        println!("Checkpoint (post restore)");
    }

    Ok(())
}

pub struct Container {
    instance: Instance,
    store: Store<()>,
    init_workflow: TypedFunc<u32, ()>,
    register_workflows: TypedFunc<(), u32>,
    run: TypedFunc<(), i32>,
}

impl Container {
    pub fn from_file(wat_file: &Path) -> Result<Self> {
        let wat = fs::read(wat_file).context(format!("Opening file {wat_file:?}"))?;
        let wat = instrument(&wat)?;
        let engine = Engine::default();
        let module = Module::new(&engine, wat)?;

        let Some(module_export) = module.get_export_index("memory") else {
            bail!("failed to find `memory` export in module");
        };
        let linker_module = "env";
        let mut linker = Linker::new(&engine);
        linker.func_wrap(
            linker_module,
            "__enhedron_log",
            move |mut caller: Caller<'_, ()>, data: u32, len: u32| {
                let message = read_string(memory(&module_export, &mut caller)?, data, len)?;
                println!("Log: {message}");
                Ok(())
            },
        )?;
        linker.func_wrap(
            linker_module,
            "__enhedron_fn_begin",
            move |mut caller: Caller<'_, ()>,
                  module_data: u32,
                  module_len: u32,
                  name_data: u32,
                  name_len: u32| {
                let memory = memory(&module_export, &mut caller)?;
                let module = read_string(memory, module_data, module_len)?;
                let name = read_string(memory, name_data, name_len)?;
                println!("Begin {module}::{name}");
                Ok(())
            },
        )?;
        linker.func_wrap(
            linker_module,
            "__enhedron_fn_end",
            move |mut caller: Caller<'_, ()>,
                  module_data: u32,
                  module_len: u32,
                  name_data: u32,
                  name_len: u32| {
                let memory = memory(&module_export, &mut caller)?;
                let module = read_string(memory, module_data, module_len)?;
                let name = read_string(memory, name_data, name_len)?;
                println!("End {module}::{name}");
                Ok(())
            },
        )?;

        let tracers = module
            .imports()
            .filter(|import| import.module() == linker_module);
        for tracer in tracers {
            let name = tracer.name();

            if let Some(trace_type) = name.strip_prefix("__enhedron_begin_") {
                let trace_type = trace_type.to_string();
                linker.func_wrap(linker_module, name, move || println!("{trace_type} begin"))?;
            }

            if let Some(trace_type) = name.strip_prefix("__enhedron_end_") {
                let trace_type = trace_type.to_string();
                linker.func_wrap(linker_module, name, move || println!("{trace_type} end"))?;
            }
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
            init_workflow,
            register_workflows,
            run,
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
}

fn memory<'a, 'b: 'a>(
    module_export: &ModuleExport,
    caller: &'a mut Caller<'b, ()>,
) -> Result<&'a [u8]> {
    let Some(Extern::Memory(memory)) = caller.get_module_export(module_export) else {
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
