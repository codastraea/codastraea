use anyhow::{bail, Context, Result};
use wasmtime::{AsContextMut, Instance, Val};

pub struct Snapshot {
    globals: Vec<(String, Val)>,
    memories: Vec<(String, SnapshotMemory)>,
}

struct SnapshotMemory {
    page_size: u64,
    data: Vec<u8>,
}

impl Snapshot {
    pub fn new(ctx: &mut impl AsContextMut, instance: &Instance) -> Self {
        let mut globals = Vec::new();
        let mut memories = Vec::new();

        let exported_names: Vec<String> = instance
            .exports(&mut *ctx)
            .map(|e| e.name().to_string())
            .collect();

        for name in exported_names {
            if let Some(global) = instance.get_global(&mut *ctx, &name) {
                if global.ty(&mut *ctx).mutability().is_var() {
                    globals.push((name.clone(), global.get(&mut *ctx)));
                }
            }

            if let Some(memory) = instance.get_memory(&mut *ctx, &name) {
                memories.push((
                    name,
                    SnapshotMemory {
                        page_size: memory.page_size(&mut *ctx),
                        data: memory.data(&mut *ctx).to_vec(),
                    },
                ))
            }
        }

        Self { globals, memories }
    }

    pub fn restore(&self, ctx: &mut impl AsContextMut, instance: &Instance) -> Result<()> {
        for (name, value) in &self.globals {
            instance
                .get_global(&mut *ctx, name)
                .with_context(|| format!("Couldn't find global '{name}"))?
                .set(&mut *ctx, *value)
                .with_context(|| format!("Couldn't set global '{name}"))?;
        }

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

            let snapshot_bytes: u64 = snapshot.data.len().try_into()?;
            let required_pages = snapshot_bytes.div_ceil(page_size);
            assert!(required_pages * page_size >= snapshot_bytes);

            let current_pages = memory.size(&mut *ctx);

            if current_pages < required_pages {
                memory.grow(&mut *ctx, required_pages - current_pages)?;
            }

            memory.write(&mut *ctx, 0, &snapshot.data)?;
        }

        Ok(())
    }
}
