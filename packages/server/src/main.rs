use std::thread;

use arpy_axum::RpcRoute;
use arpy_server::WebSocketRouter;
use axum::{Router, Server};
use futures::stream::BoxStream;
use serpent_automation_executor::{
    library::Library,
    run::{new_thread, CallStack},
    syntax_tree::parse,
    CODE,
};
use serpent_automation_server_api::ThreadSubscription;
use tokio::spawn;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

#[tokio::main]
async fn main() {
    let ws = WebSocketRouter::new().handle_subscription({
        move |updates: BoxStream<'static, CallStack>, _subscription: ThreadSubscription| {
            // TODO: Naming
            let (mut thread_run_state, mut thread_run_state_updater) = new_thread();
            // TODO: Handle errors, particularly `Lagged`.
            let receive_run_state = thread_run_state_updater.subscribe(updates);

            spawn(async move { thread_run_state_updater.update_clients().await });
            thread::spawn(move || {
                let lib = Library::link(parse(CODE).unwrap());
                lib.run(&mut thread_run_state);
            });

            // TODO: Handle errors, particularly `Lagged`.
            let receive_run_state =
                BroadcastStream::new(receive_run_state).map_while(|call_state| call_state.ok());

            ((), receive_run_state)
        }
    });

    let app = Router::new().ws_rpc_route("/api", ws, 10000);
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
