use std::{thread, time::Duration};

use arpy_axum::RpcRoute;
use arpy_server::WebSocketRouter;
use axum::{Router, Server};
use serpent_automation_executor::{
    library::Library, run::ThreadRunState, syntax_tree::parse, CODE,
};
use serpent_automation_server_api::ThreadSubscription;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;

fn main() {
    let lib = Library::link(parse(CODE).unwrap());

    let (trace_send, trace_receive) = watch::channel(ThreadRunState::new());

    thread::scope(|scope| {
        scope.spawn(|| server(trace_receive));
        scope.spawn(|| loop {
            lib.run(&trace_send);
            thread::sleep(Duration::from_secs(3));
            // TODO: Only send if not updated
            trace_send.send_replace(ThreadRunState::new());
        });
    });
}

#[tokio::main]
async fn server(call_states: watch::Receiver<ThreadRunState>) {
    let ws = WebSocketRouter::new().handle_subscription({
        let call_states = call_states.clone();
        move |_: ThreadSubscription| WatchStream::from_changes(call_states.clone())
    });
    let app = Router::new().ws_rpc_route("/api", ws, 10000);
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
