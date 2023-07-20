use std::rc::Rc;

use futures_signals::signal::Mutable;
use serpent_automation_executor::{
    library::Library, run::ThreadRunState, syntax_tree::parse, CODE,
};
use serpent_automation_frontend::server_connection;
use serpent_automation_ui::app;
use silkenweb::{mount, task::spawn_local};

fn main() {
    let module = parse(CODE).unwrap();
    let library = Rc::new(Library::link(module));
    let run_state = Mutable::new(ThreadRunState::new());

    spawn_local(server_connection(run_state.clone()));

    mount("app", app(run_state.signal_cloned(), &library));
}
