use codastraea_frontend::ServerConnection;
use silkenweb::{
    elements::html::div,
    node::element::{ChildElement, Element, ParentElement},
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

pub fn app() -> impl ChildElement {
    let server_connection = ServerConnection::default();

    div()
        .class(css::full_height())
        .child(ThreadView::new(server_connection))
}
