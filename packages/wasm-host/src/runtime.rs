use std::{fs, path::Path};

use anyhow::{Context, Result};
use wasmtime::{Caller, Engine, Linker, Module, Store};

use crate::instrument::instrument;

pub fn run(wat_file: &Path) -> Result<()> {
    let wat = fs::read(wat_file).context(format!("Opening file {wat_file:?}"))?;
    let wat = instrument(&wat)?;
    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;

    let mut linker = Linker::new(&engine);
    linker.func_wrap("env", "host_func", |_caller: Caller<'_, ()>, param: i32| {
        println!("Got {param} from WebAssembly");
    })?;

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
