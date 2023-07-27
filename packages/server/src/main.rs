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
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

fn main() {
    let lib = Library::link(parse(CODE).unwrap());

    // TODO: Need a strategy around channel size/lagging receivers
    let (trace_send, trace_receive) = broadcast::channel(1000);

    thread::scope(|scope| {
        scope.spawn(|| server(trace_receive));
        scope.spawn(|| loop {
            // TODO: Send call state updates
            lib.run(&trace_send);
            thread::sleep(Duration::from_secs(3));
            trace_send.send_replace(ThreadRunState::new());
        });
    });
}

#[tokio::main]
async fn server(call_states: broadcast::Receiver<(CallStack, RunState)>) {
    let ws = WebSocketRouter::new().handle_subscription({
        let call_states = call_states.resubscribe();

        move |_updates, _: ThreadSubscription| {
            // TODO: Handle errors, particularly `Lagged`.
            let call_states = BroadcastStream::new(call_states.resubscribe())
                .map_while(|call_state| call_state.ok());

            ((), call_states)
        }
    });
    let app = Router::new().ws_rpc_route("/api", ws, 10000);
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
