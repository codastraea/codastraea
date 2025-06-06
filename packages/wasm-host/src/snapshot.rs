use std::{
    collections::HashMap,
    ffi::c_void,
    io::{Read, Write},
};

use anyhow::{bail, Context, Result};
use flate2::{read::DeflateDecoder, write::DeflateEncoder, Compression};
use wasmtime::{AsContextMut, Func, Global, Instance, Memory, Ref, Table, Val};

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
    globals: Globals,
    memories: NamedVec<SnapshotMemory>,
    tables: NamedVec<Vec<Option<String>>>,
}

#[derive(Default)]
struct Globals {
    i32s: NamedVec<i32>,
    i64s: NamedVec<i64>,
    f32s: NamedVec<f32>,
    f64s: NamedVec<f64>,
    v128s: NamedVec<u128>,
    functions: NamedVec<Option<String>>,
    null_extern_ref_names: Vec<String>,
    null_any_ref_names: Vec<String>,
}

impl Globals {
    fn snapshot(
        &mut self,
        ctx: &mut impl AsContextMut,
        lookup_func_name: &FunctionNames,
        name: &str,
        global: Global,
    ) -> Result<()> {
        if global.ty(&mut *ctx).mutability().is_var() {
            let name = name.to_string();

            match global.get(&mut *ctx) {
                Val::I32(val) => self.i32s.push((name, val)),
                Val::I64(val) => self.i64s.push((name, val)),
                Val::F32(val) => self.f32s.push((name, f32::from_bits(val))),
                Val::F64(val) => self.f64s.push((name, f64::from_bits(val))),
                Val::V128(val) => self.v128s.push((name, val.as_u128())),
                Val::FuncRef(func) => self
                    .functions
                    .push((name, lookup_func_name.get(&mut *ctx, &func)?)),
                Val::ExternRef(None) => self.null_extern_ref_names.push(name),
                Val::AnyRef(None) => self.null_any_ref_names.push(name),
                Val::ExternRef(_) => {
                    bail!("Global '{name}': Mutable `ExternRef`s are not supported")
                }
                Val::AnyRef(_) => {
                    bail!("Global '{name}': Mutable `AnyRef`s are not supported")
                }
            }
        }

        Ok(())
    }

    fn restore(&self, ctx: &mut impl AsContextMut, instance: &Instance) -> Result<()> {
        Self::set_globals(ctx, instance, &self.i32s)?;
        Self::set_globals(ctx, instance, &self.i64s)?;
        Self::set_globals(ctx, instance, &self.f32s)?;
        Self::set_globals(ctx, instance, &self.f64s)?;
        Self::set_globals(ctx, instance, &self.v128s)?;

        for (name, func_name) in &self.functions {
            let func = get_function(&mut *ctx, instance, func_name)?;
            Self::set_global(ctx, instance, name, Val::FuncRef(func))?;
        }

        for name in &self.null_extern_ref_names {
            Self::set_global(ctx, instance, name, Val::ExternRef(None))?;
        }

        for name in &self.null_any_ref_names {
            Self::set_global(ctx, instance, name, Val::AnyRef(None))?;
        }

        Ok(())
    }

    fn set_globals<T: Copy + Into<Val>>(
        ctx: &mut impl AsContextMut,
        instance: &Instance,
        values: &[(String, T)],
    ) -> Result<()> {
        for (name, value) in values {
            Self::set_global(ctx, instance, name, (*value).into())?;
        }

        Ok(())
    }

    fn set_global(
        ctx: &mut impl AsContextMut,
        instance: &Instance,
        name: &str,
        value: Val,
    ) -> Result<()> {
        instance
            .get_global(&mut *ctx, name)
            .with_context(|| format!("Couldn't find global '{name}"))?
            .set(&mut *ctx, value)
            .with_context(|| format!("Couldn't set global '{name}"))
    }
}

type NamedVec<T> = Vec<(String, T)>;

struct SnapshotMemory {
    page_size: u64,
    uncompressed_len: usize,
    data: Vec<u8>,
}

impl Snapshot {
    pub fn new(ctx: &mut impl AsContextMut, instance: &Instance) -> Result<Self> {
        let mut globals = Globals::default();
        let mut memories = Vec::new();
        let mut tables = Vec::new();

        let exported_names: Vec<String> = instance
            .exports(&mut *ctx)
            .map(|e| e.name().to_string())
            .collect();

        let lookup_func_name = FunctionNames::new(ctx, instance, &exported_names);

        for name in exported_names {
            if let Some(global) = instance.get_global(&mut *ctx, &name) {
                globals.snapshot(ctx, &lookup_func_name, &name, global)?;
            }

            if let Some(memory) = instance.get_memory(&mut *ctx, &name) {
                memories.push((
                    name.clone(),
                    snapshot_memory(ctx, memory).with_context(|| format!("Memory {name}"))?,
                ));
            }

            if let Some(table) = instance.get_table(&mut *ctx, &name) {
                tables.push((
                    name.clone(),
                    snapshot_table(ctx, &lookup_func_name, table)
                        .with_context(|| format!("Table {name}"))?,
                ));
            }
        }

        Ok(Self {
            globals,
            memories,
            tables,
        })
    }

    pub fn restore(&self, ctx: &mut impl AsContextMut, instance: &Instance) -> Result<()> {
        self.globals.restore(ctx, instance)?;
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
                let item = get_function(ctx, instance, func_name)?;
                let index = index.try_into().expect("Index should convert to u64");
                table.set(&mut *ctx, index, Ref::Func(item))?;
            }
        }

        Ok(())
    }
}

fn snapshot_table(
    ctx: &mut impl AsContextMut,
    lookup_func_name: &FunctionNames,
    table: Table,
) -> Result<Vec<Option<String>>> {
    let mut table_data = Vec::new();
    for index in 0..table.size(&*ctx) {
        let item = table
            .get(&mut *ctx, index)
            .expect("Index should be in bounds");

        let item = match item {
            Ref::Func(func) => lookup_func_name.get(&mut *ctx, &func)?,
            Ref::Extern(_) => bail!("`ExternRef`s are not supported"),
            Ref::Any(_) => bail!("`AnyRef`s are not supported"),
        };

        table_data.push(item);
    }

    Ok(table_data)
}

fn snapshot_memory(ctx: &mut impl AsContextMut, memory: Memory) -> Result<SnapshotMemory> {
    let mut compressor = DeflateEncoder::new(Vec::new(), Compression::default());
    compressor.write_all(memory.data(&mut *ctx))?;
    let uncompressed_len = memory.data_size(&mut *ctx);
    let data = compressor.finish()?;
    Ok(SnapshotMemory {
        page_size: memory.page_size(&mut *ctx),
        uncompressed_len,
        data,
    })
}

struct FunctionNames(HashMap<*mut c_void, String>);

impl FunctionNames {
    fn new(ctx: &mut impl AsContextMut, instance: &Instance, exported_names: &Vec<String>) -> Self {
        let mut lookup_func_name = HashMap::new();

        for name in exported_names {
            if let Some(func) = instance.get_func(&mut *ctx, name) {
                lookup_func_name.insert(unsafe { func.to_raw(&mut *ctx) }, name.clone());
            }
        }

        Self(lookup_func_name)
    }

    fn get(&self, ctx: impl AsContextMut, func: &Option<Func>) -> Result<Option<String>> {
        Ok(if let Some(func) = func {
            Some(
                self.0
                    .get(&unsafe { func.to_raw(ctx) })
                    .context("Function not found")?
                    .to_string(),
            )
        } else {
            None
        })
    }
}

fn get_function(
    ctx: &mut impl AsContextMut,
    instance: &Instance,
    func_name: &Option<String>,
) -> Result<Option<Func>> {
    Ok(if let Some(func_name) = func_name {
        Some(
            instance
                .get_func(&mut *ctx, func_name)
                .with_context(|| format!("Function '{func_name}' not found"))?,
        )
    } else {
        None
    })
}
