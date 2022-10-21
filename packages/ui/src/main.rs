use std::rc::Rc;

use serpent_automation_executor::{library::Library, syntax_tree::parse, CODE};
use serpent_automation_frontend::{server_connection, StackFrameStates};
use serpent_automation_ui::app;
use silkenweb::{mount, task::spawn_local};

fn main() {
    let module = parse(CODE).unwrap();
    let library = Rc::new(Library::link(module));
    let stack_frame_states = StackFrameStates::new();

    spawn_local(server_connection(stack_frame_states.clone()));

    mount("app", app(&library, &stack_frame_states));
}
