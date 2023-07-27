use std::{thread, time::Duration};

use arpy_axum::RpcRoute;
use arpy_server::WebSocketRouter;
use axum::{Router, Server};
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
    let (send_run_state, receive_run_state) = broadcast::channel(1000);

    thread::scope(|scope| {
        scope.spawn(|| server(receive_run_state));
        scope.spawn(|| loop {
            let mut thread_run_state = ThreadRunState::new();
            lib.run(&mut thread_run_state);
            thread::sleep(Duration::from_secs(3));
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
