use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::{
    run::ThreadRunState,
    syntax_tree::{run_call, IdMap, LinkedFunction, Module},
};

pub struct Library {
    main_id: Option<FunctionId>,
    lookup_map: Vec<LinkedFunction>,
}

impl Library {
    /// Constructor
    ///
    /// Translate all `String` function id's to a [`FunctionId`] that is fast to
    /// lookup
    pub fn link(module: Module) -> Self {
        let symbol_table_len = module.functions().len();
        let mut id_map = Self::symbol_table(&module);
        let unresolved_symbols: BTreeSet<String> = module
            .functions()
            .iter()
            .flat_map(|f| f.unresolved_symbols(&id_map))
            .collect();

        assert!(symbol_table_len == id_map.len());

        for (index, python_name) in unresolved_symbols.iter().enumerate() {
            id_map.insert(python_name.clone(), FunctionId(index + symbol_table_len));
        }

        let main_id = id_map.get("main").copied();
        let lookup_map = module
            .functions()
            .iter()
            .map(|f| f.translate_ids(&id_map))
            .chain(unresolved_symbols.into_iter().map(LinkedFunction::python))
            .collect();

        Self {
            main_id,
            lookup_map,
        }
    }

    /// Lookup a function id
    ///
    /// Any [`FunctionId`]'s in the return value will be valid lookup with this
    /// function.
    ///
    /// # Panic
    ///
    /// If `id` was not found.
    pub fn lookup(&self, id: FunctionId) -> &LinkedFunction {
        &self.lookup_map[id.0]
    }

    /// Lookup a function called "main"
    ///
    /// Returns `None` if not found.
    pub fn main(&self) -> Option<&LinkedFunction> {
        self.main_id.map(|main| self.lookup(main))
    }

    /// The id of a function called "main"
    ///
    /// Returns `None` if there was no "main" function.
    pub fn main_id(&self) -> Option<FunctionId> {
        self.main_id
    }

    pub fn run(&self, call_states: &watch::Sender<ThreadRunState>) {
        if let Some(main_id) = self.main_id() {
            run_call(main_id, &[], self, call_states);
        }
    }

    fn symbol_table(module: &Module) -> HashMap<String, FunctionId> {
        let mut id_map = IdMap::new();

        for function in module.functions() {
            let name = function.name();
            let id = FunctionId(id_map.len());

            if id_map.insert(name.to_owned(), id).is_some() {
                // TODO: Error
                panic!("Duplicte symbol: {name}");
            }
        }
        id_map
    }
}

/// An id for a function that is fast to lookup.
#[derive(Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct FunctionId(usize);
