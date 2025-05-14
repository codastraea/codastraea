use std::rc::Rc;

use serpent_automation_executor::library::Library;
use serpent_automation_frontend::{call_tree::CallTree, ServerConnection};
use silkenweb::{
    node::element::ChildElement,
    prelude::{html::div, Element, ParentElement},
    task::spawn_local,
};
use thread_view::ThreadView;

macro_rules! css_module {
    ($path:literal) => {
        silkenweb::css!(
            path = concat("css/", $path, ".css"),
            auto_mount,
            transpile = (modules)
        );
    };
}

mod call_tree_view;
mod source_view;
mod thread_view;
mod css {
    css_module!("shared");

    pub use class::*;
}

use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub fn app(library: &Rc<Library>) -> impl ChildElement {
    let main_id = library.main_id().unwrap();
    let (opened_nodes_sender, opened_nodes_receiver) = mpsc::unbounded_channel();
    let call_tree = CallTree::root(main_id, library, opened_nodes_sender);

    let opened_nodes_receiver = UnboundedReceiverStream::new(opened_nodes_receiver);
    spawn_local(call_tree.update_run_state(ServerConnection::default(), opened_nodes_receiver));

    div()
        .class(css::full_height())
        .child(ThreadView::new(call_tree))
}
