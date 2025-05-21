use std::{fs, path::Path};

use anyhow::{bail, Context, Result};
use wasmtime::{Caller, Engine, Extern, Instance, Linker, Module, Store, TypedFunc};

use crate::{instrument::instrument, snapshot::Snapshot};

pub fn run(wat_file: &Path) -> Result<()> {
    let mut container = Container::from_file(wat_file)?;

    for _i in 0..5 {
        container.run()?;
        println!("Checkpoint (pre snapshot)");
    }

    let snapshot = container.snapshot();

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
        let mut linker = Linker::new(&engine);
        linker.func_wrap(
            "env",
            "__enhedron_log",
            move |mut caller: Caller<'_, ()>, data: u32, len: u32| {
                let data: usize = data.try_into().unwrap();
                let len: usize = len.try_into().unwrap();
                let Some(Extern::Memory(memory)) = caller.get_module_export(&module_export) else {
                    bail!("failed to find host memory")
                };
                let data = memory
                    .data(&caller)
                    .get(data..)
                    .context("`data` out of bounds")?
                    .get(..len)
                    .context("`len` out of bounds")?;
                let string = str::from_utf8(data).context("Invalid utf-8")?;
                println!("Log: {}", string);
                Ok(())
            },
        )?;

        let mut store = Store::new(&engine, ());
        let instance = linker.instantiate(&mut store, &module)?;
        let run = instance.get_typed_func::<(), i32>(&mut store, "__enhedron_run")?;

        Ok(Self {
            instance,
            store,
            run,
        })
    }

    pub fn snapshot(&mut self) -> Snapshot {
        Snapshot::new(&mut self.store, &self.instance)
    }

    pub fn restore(&mut self, snapshot: &Snapshot) -> Result<()> {
        snapshot.restore(&mut self.store, &self.instance)
    }

    pub fn run(&mut self) -> Result<bool> {
        Ok(self.run.call(&mut self.store, ())? != 0)
    }
}
