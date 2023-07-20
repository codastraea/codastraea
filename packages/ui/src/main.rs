use std::rc::Rc;

use serpent_automation_executor::{library::Library, syntax_tree::parse, CODE};
use serpent_automation_frontend::server_connection;
use serpent_automation_ui::app;
use silkenweb::{mount, task::spawn_local};

fn main() {
    let module = parse(CODE).unwrap();
    let library = Rc::new(Library::link(module));

    spawn_local(server_connection());

    mount("app", app(&library));
}
