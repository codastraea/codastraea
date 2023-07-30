use std::thread;

use arpy_axum::RpcRoute;
use arpy_server::WebSocketRouter;
use axum::{Router, Server};
use futures::stream::BoxStream;
use serpent_automation_executor::{
    library::Library,
    run::{CallStack, ThreadRunState},
    syntax_tree::parse,
    CODE,
};
use serpent_automation_server_api::ThreadSubscription;
use tokio::spawn;
use tokio_stream::wrappers::ReceiverStream;

#[tokio::main]
async fn main() {
    let thread_run_state = ThreadRunState::default();

    thread::spawn({
        let thread_run_state = thread_run_state.clone();

        move || {
            let lib = Library::link(parse(CODE).unwrap());
            lib.run(&thread_run_state);
        }
    });

    let ws = WebSocketRouter::new().handle_subscription({
        move |updates: BoxStream<'static, CallStack>, _subscription: ThreadSubscription| {
            let (run_state_receiver, update_client) = thread_run_state.subscribe(updates);
            spawn(update_client);
            ((), ReceiverStream::new(run_state_receiver))
        }
    });

    let app = Router::new().ws_rpc_route("/api", ws, 10000);
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
