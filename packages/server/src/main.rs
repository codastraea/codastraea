use std::{thread, time::Duration};

use arpy_axum::RpcRoute;
use arpy_server::WebSocketRouter;
use axum::{Router, Server};
use futures_signals::{
    signal::Broadcaster,
    signal_map::{MutableBTreeMap, SignalMap, SignalMapExt},
};
use serpent_automation_executor::{
    library::Library,
    run::{CallStack, RunState, ThreadRunState},
    syntax_tree::parse,
    CODE,
};
use serpent_automation_server_api::ThreadSubscription;
use tokio::{
    spawn,
    sync::{mpsc, watch},
};
use tokio_stream::wrappers::{ReceiverStream, WatchStream};

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

// TODO: Take a tokio broadcast channel and disconnect on lagging.
#[tokio::main]
async fn server(call_states: MutableBTreeMap<CallStack, RunState>) {
    let ws = WebSocketRouter::new().handle_subscription({
        let call_states = call_states.clone().entries_cloned();

        move |_updates, _: ThreadSubscription| ((), ReceiverStream::new(recv_call_states))
    });
    let app = Router::new().ws_rpc_route("/api", ws, 10000);
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
