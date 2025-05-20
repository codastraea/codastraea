use std::collections::HashSet;

use id_arena::Id;
use walrus::{ExportItem, Global, Memory, Module, ModuleExports, Result};

pub fn instrument(wasm_module: &[u8]) -> Result<Vec<u8>> {
    let mut module = Module::from_buffer(wasm_module)?;

    export_private_items(
        &mut module.exports,
        "global",
        module.globals.iter().map(Global::id),
        |item| match item {
            ExportItem::Global(id) => Some(id),
            _ => None,
        },
    );

    export_private_items(
        &mut module.exports,
        "memory",
        module.memories.iter().map(Memory::id),
        |item| match item {
            ExportItem::Memory(id) => Some(id),
            _ => None,
        },
    );

    Ok(module.emit_wasm())
}

fn export_private_items<T>(
    exports: &mut ModuleExports,
    type_name: &str,
    ids: impl Iterator<Item = Id<T>>,
    filter: impl Fn(ExportItem) -> Option<Id<T>>,
) where
    Id<T>: Into<ExportItem>,
{
    let already_exported: HashSet<Id<T>> = exports.iter().filter_map(|e| filter(e.item)).collect();
    let private_ids = ids.filter(move |id| !already_exported.contains(id));

    for id in private_ids {
        let name = format!("__enhedron_{type_name}_{}", id.index());
        exports.add(&name, id.into());
    }
}
