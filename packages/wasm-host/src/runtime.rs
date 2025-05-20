use anyhow::Result;
use wasmtime::{Caller, Engine, Linker, Module, Store};

use crate::instrument::instrument;

pub fn run(wat: &[u8]) -> Result<()> {
    let wat = instrument(wat);
    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;

    // Host functionality can be arbitrary Rust functions and is provided
    // to guests through a `Linker`.
    let mut linker = Linker::new(&engine);
    linker.func_wrap("env", "host_func", |caller: Caller<'_, u32>, param: i32| {
        println!("Got {} from WebAssembly", param);
        println!("my host state is: {}", caller.data());
    })?;

    // All wasm objects operate within the context of a "store". Each
    // `Store` has a type parameter to store host-specific data, which in
    // this case we're using `4` for.
    let mut store: Store<u32> = Store::new(&engine, 4);

    // Instantiation of a module requires specifying its imports and then
    // afterwards we can fetch exports by name, as well as asserting the
    // type signature of the function with `get_typed_func`.
    let instance = linker.instantiate(&mut store, &module)?;
    let hello = instance.get_typed_func::<(), ()>(&mut store, "hello")?;
    let memory = instance.get_memory(&mut store, "memory").unwrap();

    // And finally we can call the wasm!
    hello.call(&mut store, ())?;

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
