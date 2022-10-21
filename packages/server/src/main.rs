use std::{thread, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        TypedHeader,
    },
    headers,
    response::IntoResponse,
    routing::get,
    Router, Server,
};
use serpent_automation_executor::{library::Library, run::ThreadState, syntax_tree::parse, CODE};
use tokio::{sync::watch, time::sleep};

#[tokio::main]
async fn main() {
    let lib = Library::link(parse(CODE).unwrap());

    let (trace_send, trace_receive) = watch::channel(ThreadState::new());

    thread::scope(|scope| {
        scope.spawn(|| server(trace_receive));
        scope.spawn(|| loop {
            lib.run(&trace_send);
            thread::sleep(Duration::from_secs(3));
            trace_send.send_replace(ThreadState::new());
        });
    });
}

#[tokio::main]
async fn server(thread_state: watch::Receiver<ThreadState>) {
    let handler =
        |ws, user_agent| async { upgrade_to_websocket(thread_state, ws, user_agent).await };
    let app = Router::new().route("/", get(handler));
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn upgrade_to_websocket(
    thread_state: watch::Receiver<ThreadState>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        println!("`{}` connected", user_agent.as_str());
    }

    ws.on_upgrade(|socket| handler(thread_state, socket))
}

async fn handler(mut thread_state: watch::Receiver<ThreadState>, mut socket: WebSocket) {
    println!("Upgraded to websocket");

    loop {
        thread_state.changed().await.unwrap();

        let serialize_tracer = serde_json::to_string(&*thread_state.borrow()).unwrap();
        println!("Sending run state");

        // TODO: Diff `RunTracer` and send a `RunTracerDelta`
        if socket.send(Message::Text(serialize_tracer)).await.is_err() {
            println!("Client disconnected");
            return;
        }

        sleep(Duration::from_millis(100)).await;
    }
}
