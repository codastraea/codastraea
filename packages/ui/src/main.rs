use std::{cell::RefCell, collections::HashMap, rc::Rc};

use serpent_automation_executor::{library::Library, syntax_tree::parse, CODE};
use serpent_automation_frontend::{server_connection, RunStates};
use serpent_automation_ui::app;
use silkenweb::{mount, task::spawn_local};

fn main() {
    let module = parse(CODE).unwrap();
    let library = Rc::new(Library::link(module));
    let run_states: RunStates = Rc::new(RefCell::new(HashMap::new()));

    spawn_local(server_connection(run_states.clone()));

    mount("app", app(&library, &run_states));
}
