use std::{fs, path::Path};

use anyhow::{Context, Result};
use wasmtime::{Caller, Engine, Extern, Linker, Module, Store};

use crate::instrument::instrument;

pub fn run(wat_file: &Path) -> Result<()> {
    let wat = fs::read(wat_file).context(format!("Opening file {wat_file:?}"))?;
    let wat = instrument(&wat)?;
    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;

    let Some(module_export) = module.get_export_index("memory") else {
        anyhow::bail!("failed to find `memory` export in module");
    };
    let mut linker = Linker::new(&engine);
    linker.func_wrap(
        "env",
        "__enhedron_log",
        move |mut caller: Caller<'_, ()>, data: u32, len: u32| {
            let data: usize = data.try_into().unwrap();
            let len: usize = len.try_into().unwrap();
            let Some(Extern::Memory(memory)) = caller.get_module_export(&module_export) else {
                anyhow::bail!("failed to find host memory")
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
    let memory = instance.get_memory(&mut store, "memory").unwrap();

    while run.call(&mut store, ())? != 0 {
        println!("Checkpoint");
    }

    println!(
        "Memory: data_size = {}, len = {}",
        memory.data_size(&mut store),
        memory.data(&mut store).len()
    );

    let mut total_non_zero = 0;
    let mut buffer = vec![0; memory.data_size(&mut store)];
    memory.read(&mut store, 0, &mut buffer).unwrap();

    for byte in buffer {
        if byte != 0 {
            total_non_zero += 1;
        }
    }

    println!("Non zero bytes: {total_non_zero}");

    let exports = instance
        .exports(&mut store)
        .map(|export| export.name().to_string())
        .collect::<Vec<_>>();

    for name in exports.iter() {
        let export = instance.get_export(&mut store, name).unwrap();
        println!("export {} = {:?}", name, export.ty(&mut store))
    }

    Ok(())
}
