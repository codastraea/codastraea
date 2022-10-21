use std::rc::Rc;

use serpent_automation_executor::{library::Library, syntax_tree::parse, CODE};
use serpent_automation_frontend::server_connection;
use serpent_automation_ui::{app, ViewCallStates};
use silkenweb::{mount, task::spawn_local};

fn main() {
    let module = parse(CODE).unwrap();
    let library = Rc::new(Library::link(module));
    let view_call_states = ViewCallStates::new();

    spawn_local(server_connection(view_call_states.clone()));

    mount("app", app(&library, &view_call_states));
}
