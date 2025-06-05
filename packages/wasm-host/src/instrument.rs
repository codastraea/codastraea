use std::collections::HashSet;

use id_arena::Id;
use walrus::{ExportItem, Function, Global, Memory, Module, ModuleExports, Result, Table};

pub fn instrument(wasm_module: &[u8]) -> Result<Vec<u8>> {
    let mut module = Module::from_buffer(wasm_module)?;
    let exports = &mut module.exports;

    export_internal_items(
        exports,
        "global",
        module.globals.iter().filter(|g| g.mutable).map(Global::id),
        |item| match item {
            ExportItem::Global(id) => Some(id),
            _ => None,
        },
    );

    export_internal_items(
        exports,
        "memory",
        module.memories.iter().map(Memory::id),
        |item| match item {
            ExportItem::Memory(id) => Some(id),
            _ => None,
        },
    );

    export_internal_items(
        exports,
        "table",
        module.tables.iter().map(Table::id),
        |item| match item {
            ExportItem::Table(id) => Some(id),
            _ => None,
        },
    );

    export_internal_items(
        exports,
        "function",
        module.funcs.iter().map(Function::id),
        |item| match item {
            ExportItem::Function(id) => Some(id),
            _ => None,
        },
    );

    Ok(module.emit_wasm())
}

fn export_internal_items<T>(
    exports: &mut ModuleExports,
    type_name: &str,
    ids: impl Iterator<Item = Id<T>>,
    into_id: impl Fn(ExportItem) -> Option<Id<T>>,
) where
    Id<T>: Into<ExportItem>,
{
    let exported_ids: HashSet<Id<T>> = exports.iter().filter_map(|e| into_id(e.item)).collect();
    let internal_ids = ids.filter(|id| !exported_ids.contains(id));

    for id in internal_ids {
        let name = format!("__enhedron_{type_name}_{}", id.index());
        exports.add(&name, id.into());
    }
}

/// This exists solely to produce a compile error if something is added to
/// `ExportItem`.
fn _export_items(item: ExportItem) {
    match item {
        ExportItem::Function(_) => {}
        ExportItem::Table(_) => {}
        ExportItem::Memory(_) => {}
        ExportItem::Global(_) => {}
    }
}
