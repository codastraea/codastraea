use serde::{Deserialize, Serialize};

use crate::syntax_tree::{Function, IdMap, Module};

pub struct Library {
    main_id: Option<FunctionId>,
    lookup_map: Vec<Function<FunctionId>>,
}

impl Library {
    pub fn link(module: Module) -> Self {
        let mut id_map = IdMap::new();
        let mut main_id = None;

        for function in module.functions() {
            let name = function.name();
            let id = FunctionId(id_map.len());
            id_map.insert(name.to_owned(), id);

            if name == "main" {
                main_id = Some(id);
            }
        }

        let lookup_map = module
            .functions()
            .iter()
            .map(|f| f.translate_ids(&id_map))
            .collect();

        Self {
            main_id,
            lookup_map,
        }
    }

    pub fn lookup(&self, id: FunctionId) -> &Function<FunctionId> {
        &self.lookup_map[id.0]
    }

    pub fn main(&self) -> Option<&Function<FunctionId>> {
        self.main_id.map(|main| self.lookup(main))
    }

    pub fn main_id(&self) -> Option<FunctionId> {
        self.main_id
    }
}

#[derive(Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct FunctionId(usize);
