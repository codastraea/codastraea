use std::io::{Read, Write};

use anyhow::{bail, Context, Result};
use flate2::{read::DeflateDecoder, write::DeflateEncoder, Compression};
use wasmtime::{AsContextMut, Instance, Val};

// # TODO
//
// ## Function
//
// We'd need to export every function during instrumentation, then build a map
// of raw pointer to function name + any host functions. Then we can look at
// global function values and determine the function name. We'd then store the
// function name as the globals value. Raw pointer values change rom run to run.
// Maybe they represent the address of the compiled funcion on the host? Anyway,
// we definitely can't just store the raw pointer.
//
// To restore a snapshot, we'd need a map of name to raw pointer.
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
//
// ## Table
//
// These are just arrays of Func/ExternRef/AnyRefs, so once we handle all those
// types, tables should be easy.

pub struct Snapshot {
    global_i32s: Vec<(String, i32)>,
    global_i64s: Vec<(String, i64)>,
    global_f32s: Vec<(String, f32)>,
    global_f64s: Vec<(String, f64)>,
    global_v128s: Vec<(String, u128)>,
    memories: Vec<(String, SnapshotMemory)>,
}

struct SnapshotMemory {
    page_size: u64,
    uncompressed_len: usize,
    data: Vec<u8>,
}

impl Snapshot {
    pub fn new(ctx: &mut impl AsContextMut, instance: &Instance) -> Result<Self> {
        let mut global_i32s = Vec::new();
        let mut global_i64s = Vec::new();
        let mut global_f32s = Vec::new();
        let mut global_f64s = Vec::new();
        let mut global_v128s = Vec::new();
        let mut memories = Vec::new();

        let exported_names: Vec<String> = instance
            .exports(&mut *ctx)
            .map(|e| e.name().to_string())
            .collect();

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
                        Val::FuncRef(_) => bail!("Mutable `FuncRef`s are not supported"),
                        Val::ExternRef(_) => bail!("Mutable `ExternRef`s are not supported"),
                        Val::AnyRef(_) => bail!("Mutable `AnyRef`s are not supported"),
                    }
                }
            }

            if let Some(memory) = instance.get_memory(&mut *ctx, &name) {
                let mut compressor = DeflateEncoder::new(Vec::new(), Compression::default());
                compressor.write_all(memory.data(&mut *ctx))?;
                let uncompressed_len = memory.data_size(&mut *ctx);
                let data = compressor.finish()?;
                memories.push((
                    name,
                    SnapshotMemory {
                        page_size: memory.page_size(&mut *ctx),
                        uncompressed_len,
                        data,
                    },
                ))
            }
        }

        Ok(Self {
            global_i32s,
            global_i64s,
            global_f32s,
            global_f64s,
            global_v128s,
            memories,
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
        Self::set_globals(ctx, instance, &self.global_i32s)?;
        Self::set_globals(ctx, instance, &self.global_i64s)?;
        Self::set_globals(ctx, instance, &self.global_f32s)?;
        Self::set_globals(ctx, instance, &self.global_f64s)?;
        Self::set_globals(ctx, instance, &self.global_v128s)?;

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
}
