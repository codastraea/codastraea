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
use bincode::Options;
use serpent_automation_executor::{
    library::Library, run::ThreadCallStates, syntax_tree::parse, CODE,
};
use tokio::{sync::watch, time::sleep};

#[tokio::main]
async fn main() {
    let lib = Library::link(parse(CODE).unwrap());

    let (trace_send, trace_receive) = watch::channel(ThreadCallStates::new());

    thread::scope(|scope| {
        scope.spawn(|| server(trace_receive));
        scope.spawn(|| loop {
            lib.run(&trace_send);
            thread::sleep(Duration::from_secs(3));
            trace_send.send_replace(ThreadCallStates::new());
        });
    });
}

#[tokio::main]
async fn server(call_states: watch::Receiver<ThreadCallStates>) {
    let handler =
        |ws, user_agent| async { upgrade_to_websocket(call_states, ws, user_agent).await };
    let app = Router::new().route("/", get(handler));
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn upgrade_to_websocket(
    call_states: watch::Receiver<ThreadCallStates>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        println!("`{}` connected", user_agent.as_str());
    }

    ws.on_upgrade(|socket| handler(call_states, socket))
}

async fn handler(mut call_states: watch::Receiver<ThreadCallStates>, mut socket: WebSocket) {
    println!("Upgraded to websocket");

    loop {
        call_states.changed().await.unwrap();

        let serialize_tracer = bincode::options()
            .serialize(&*call_states.borrow())
            .unwrap();
        println!("Sending run state");

        // TODO: Diff `RunTracer` and send a `RunTracerDelta`
        if socket
            .send(Message::Binary(serialize_tracer))
            .await
            .is_err()
        {
            println!("Client disconnected");
            return;
        }

        sleep(Duration::from_millis(100)).await;
    }
}
