use std::collections::HashSet;

use id_arena::Id;
use walrus::{ExportItem, Global, Memory, Module, ModuleExports, Result};

pub fn instrument(wasm_module: &[u8]) -> Result<Vec<u8>> {
    let mut module = Module::from_buffer(wasm_module)?;

    export_internal_items(
        &mut module.exports,
        "global",
        module.globals.iter().map(Global::id),
        |item| match item {
            ExportItem::Global(id) => Some(id),
            _ => None,
        },
    );

    export_internal_items(
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

fn export_internal_items<T>(
    exports: &mut ModuleExports,
    type_name: &str,
    ids: impl Iterator<Item = Id<T>>,
    filter: impl Fn(ExportItem) -> Option<Id<T>>,
) where
    Id<T>: Into<ExportItem>,
{
    let exported_ids: HashSet<Id<T>> = exports.iter().filter_map(|e| filter(e.item)).collect();
    let internal_ids = ids.filter(|id| !exported_ids.contains(id));

    for id in internal_ids {
        let name = format!("__enhedron_{type_name}_{}", id.index());
        exports.add(&name, id.into());
    }
}
