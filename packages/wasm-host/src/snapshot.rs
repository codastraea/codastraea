use std::{
    collections::HashMap,
    ffi::c_void,
    io::{Read, Write},
};

use anyhow::{bail, Context, Result};
use flate2::{read::DeflateDecoder, write::DeflateEncoder, Compression};
use wasmtime::{AsContextMut, Instance, Memory, Ref, Val};

// # TODO
//
// ## ExternRef/AnyRef
//
// Need to investigate these.
//
// ## Shared Memory
//
// - Memories in Walrus just have a shared flag to identify them, so they should
//   already be exported by the instrumentation.
// - Should we be copying shared memory or rejecting anything with shared
//   memory? All threads should at least be stopped before a snapshot.

pub struct Snapshot {
    global_i32s: NamedVec<i32>,
    global_i64s: NamedVec<i64>,
    global_f32s: NamedVec<f32>,
    global_f64s: NamedVec<f64>,
    global_v128s: NamedVec<u128>,
    memories: NamedVec<SnapshotMemory>,
    tables: NamedVec<Vec<Option<String>>>,
}

type NamedVec<T> = Vec<(String, T)>;

struct SnapshotMemory {
    page_size: u64,
    uncompressed_len: usize,
    data: Vec<u8>,
}

impl Snapshot {
    pub fn new(ctx: &mut impl AsContextMut, instance: &Instance) -> Result<Self> {
        // TODO: Break this function up
        let mut global_i32s = Vec::new();
        let mut global_i64s = Vec::new();
        let mut global_f32s = Vec::new();
        let mut global_f64s = Vec::new();
        let mut global_v128s = Vec::new();
        let mut memories = Vec::new();
        let mut tables = Vec::new();

        let exported_names: Vec<String> = instance
            .exports(&mut *ctx)
            .map(|e| e.name().to_string())
            .collect();

        let lookup_func_name = func_name_lookup(ctx, instance, &exported_names);

        for name in exported_names {
            if let Some(global) = instance.get_global(&mut *ctx, &name) {
                if global.ty(&mut *ctx).mutability().is_var() {
                    let name = name.clone();

                    match global.get(&mut *ctx) {
                        Val::I32(val) => global_i32s.push((name, val)),
                        Val::I64(val) => global_i64s.push((name, val)),
                        Val::F32(val) => global_f32s.push((name, f32::from_bits(val))),
                        Val::F64(val) => global_f64s.push((name, f64::from_bits(val))),
                        Val::V128(val) => global_v128s.push((name, val.as_u128())),
                        Val::FuncRef(_) => {
                            bail!("Global '{name}': Mutable `FuncRef`s are not supported")
                        }
                        Val::ExternRef(_) => {
                            bail!("Global '{name}': Mutable `ExternRef`s are not supported")
                        }
                        Val::AnyRef(_) => {
                            bail!("Global '{name}': Mutable `AnyRef`s are not supported")
                        }
                    }
                }
            }

            if let Some(memory) = instance.get_memory(&mut *ctx, &name) {
                snapshot_memory(ctx, &mut memories, &name, memory)?;
            }

            if let Some(table) = instance.get_table(&mut *ctx, &name) {
                let mut table_data = Vec::new();

                for index in 0..table.size(&*ctx) {
                    let item = table
                        .get(&mut *ctx, index)
                        .expect("Index should be in bounds");

                    let item = match item {
                        Ref::Func(None) => None,
                        Ref::Func(Some(func)) => {
                            let name = lookup_func_name
                                .get(&unsafe { func.to_raw(&mut *ctx) })
                                .with_context(|| {
                                    format!("Table {name}[{index}]: function not found")
                                })?;

                            Some(name.clone())
                        }
                        Ref::Extern(_) => {
                            bail!("Table {name}[{index}]: `ExternRef`s are not supported")
                        }
                        Ref::Any(_) => {
                            bail!("Table {name}[{index}]: `AnyRef`s are not supported")
                        }
                    };

                    table_data.push(item);
                }

                tables.push((name.clone(), table_data));
            }
        }

        Ok(Self {
            global_i32s,
            global_i64s,
            global_f32s,
            global_f64s,
            global_v128s,
            memories,
            tables,
        })
    }

    fn set_globals<T: Copy + Into<Val>>(
        ctx: &mut impl AsContextMut,
        instance: &Instance,
        values: &[(String, T)],
    ) -> Result<()> {
        for (name, value) in values {
            instance
                .get_global(&mut *ctx, name)
                .with_context(|| format!("Couldn't find global '{name}"))?
                .set(&mut *ctx, (*value).into())
                .with_context(|| format!("Couldn't set global '{name}"))?;
        }

        Ok(())
    }

    pub fn restore(&self, ctx: &mut impl AsContextMut, instance: &Instance) -> Result<()> {
        self.restore_globals(ctx, instance)?;
        self.restore_tables(ctx, instance)?;
        self.restore_memories(ctx, instance)?;

        Ok(())
    }

    fn restore_memories(&self, ctx: &mut impl AsContextMut, instance: &Instance) -> Result<()> {
        for (name, snapshot) in &self.memories {
            let memory = instance
                .get_memory(&mut *ctx, name)
                .with_context(|| format!("Couldn't find memory '{name}"))?;
            let page_size = memory.page_size(&mut *ctx);

            if page_size != snapshot.page_size {
                bail!(
                    "Page size of instance memory {name} is {page_size} and doesn't match {}",
                    snapshot.page_size
                );
            }

            let snapshot_bytes: u64 = snapshot.uncompressed_len.try_into()?;
            let required_pages = snapshot_bytes.div_ceil(page_size);
            assert!(required_pages * page_size >= snapshot_bytes);

            let current_pages = memory.size(&mut *ctx);

            if current_pages < required_pages {
                memory.grow(&mut *ctx, required_pages - current_pages)?;
            }

            let mut decoder = DeflateDecoder::new(&snapshot.data[..]);
            decoder.read_exact(&mut memory.data_mut(&mut *ctx)[..snapshot.uncompressed_len])?;

            memory.write(&mut *ctx, 0, &snapshot.data)?;
        }

        Ok(())
    }

    fn restore_tables(&self, ctx: &mut impl AsContextMut, instance: &Instance) -> Result<()> {
        for (name, snapshot_table) in &self.tables {
            let table = instance
                .get_table(&mut *ctx, name)
                .with_context(|| format!("Couldn't find table {name}"))?;
            let snapshot_len: u64 = snapshot_table
                .len()
                .try_into()
                .expect("Table len should convert to u64");
            let table_size = table.size(&*ctx);

            if let Some(delta) = snapshot_len.checked_sub(table_size) {
                table.grow(&mut *ctx, delta, Ref::Func(None))?;
            }

            for (index, func_name) in snapshot_table.iter().enumerate() {
                let item = if let Some(func_name) = func_name {
                    let func = instance
                        .get_func(&mut *ctx, func_name)
                        .with_context(|| format!("Function '{func_name}' not found"))?;
                    Ref::Func(Some(func))
                } else {
                    Ref::Func(None)
                };

                let index = index.try_into().expect("Index should convert to u64");
                table.set(&mut *ctx, index, item)?;
            }
        }

        Ok(())
    }

    fn restore_globals(&self, ctx: &mut impl AsContextMut, instance: &Instance) -> Result<()> {
        Self::set_globals(ctx, instance, &self.global_i32s)?;
        Self::set_globals(ctx, instance, &self.global_i64s)?;
        Self::set_globals(ctx, instance, &self.global_f32s)?;
        Self::set_globals(ctx, instance, &self.global_f64s)?;
        Self::set_globals(ctx, instance, &self.global_v128s)?;
        Ok(())
    }
}

fn snapshot_memory(
    ctx: &mut impl AsContextMut,
    memories: &mut NamedVec<SnapshotMemory>,
    name: &str,
    memory: Memory,
) -> Result<()> {
    let mut compressor = DeflateEncoder::new(Vec::new(), Compression::default());
    compressor.write_all(memory.data(&mut *ctx))?;
    let uncompressed_len = memory.data_size(&mut *ctx);
    let data = compressor.finish()?;
    memories.push((
        name.to_string(),
        SnapshotMemory {
            page_size: memory.page_size(&mut *ctx),
            uncompressed_len,
            data,
        },
    ));
    Ok(())
}

fn func_name_lookup(
    ctx: &mut impl AsContextMut,
    instance: &Instance,
    exported_names: &Vec<String>,
) -> HashMap<*mut c_void, String> {
    let mut lookup_func_name = HashMap::new();

    for name in exported_names {
        if let Some(func) = instance.get_func(&mut *ctx, name) {
            lookup_func_name.insert(unsafe { func.to_raw(&mut *ctx) }, name.clone());
        }
    }

    lookup_func_name
}
