use std::rc::Rc;

use serpent_automation_executor::{library::Library, syntax_tree::parse, CODE};
use serpent_automation_frontend::server_connection;
use serpent_automation_ui::app;
use silkenweb::{mount, task::spawn_local};
use tokio::sync::mpsc;

fn main() {
    let module = parse(CODE).unwrap();
    let library = Rc::new(Library::link(module));
    let (send_run_state, recv_run_state) = mpsc::channel(1);

    spawn_local(server_connection(send_run_state));

    mount("app", app(recv_run_state, &library));
}
