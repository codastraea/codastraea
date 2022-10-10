use std::{
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

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
use serpent_automation_executor::{
    library::Library,
    run::{run, RunTracer},
    syntax_tree::parse,
    CODE,
};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let lib = Library::link(parse(CODE).unwrap());
    // Unfortunately we need to use an `Arc` as axum requires `'static` lifetimes on
    // closures/futures.
    let tracer = Arc::new(RwLock::new(RunTracer::new()));

    thread::scope(|scope| {
        scope.spawn(|| ui(tracer.clone()));
        scope.spawn(|| loop {
            *(tracer.write().unwrap()) = RunTracer::new();
            run(&lib, &tracer)
        });
    });
}

#[tokio::main]
async fn ui(tracer: Arc<RwLock<RunTracer>>) {
    let handler = move |ws, user_agent| async move { ws_handler(tracer, ws, user_agent).await };
    let app = Router::new().route("/", get(handler));
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ws_handler(
    tracer: Arc<RwLock<RunTracer>>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        println!("`{}` connected", user_agent.as_str());
    }

    ws.on_upgrade(move |socket| handle_socket(tracer, socket))
}

async fn handle_socket(tracer: Arc<RwLock<RunTracer>>, mut socket: WebSocket) {
    println!("Upgraded to websocket");

    loop {
        println!("Sending run state");
        let tracer_snapshot = tracer.read().unwrap().clone();

        // TODO: Diff `RunTracer` and send a `RunTracerDelta`
        if socket
            .send(Message::Text(
                serde_json::to_string(&tracer_snapshot).unwrap(),
            ))
            .await
            .is_err()
        {
            println!("Client disconnected");
            return;
        }

        sleep(Duration::from_millis(100)).await;
    }
}
