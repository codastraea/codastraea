use std::{cell::RefCell, collections::HashMap, rc::Rc};

use serpent_automation_executor::{library::Library, syntax_tree::parse, CODE};
use serpent_automation_frontend::{server_connection, FunctionStates};
use serpent_automation_ui::app;
use silkenweb::{mount, task::spawn_local};

fn main() {
    let module = parse(CODE).unwrap();
    let library = Rc::new(Library::link(module));
    let fn_states: FunctionStates = Rc::new(RefCell::new(HashMap::new()));

    spawn_local(server_connection(fn_states.clone()));

    mount("app", app(&library, &fn_states));
}
