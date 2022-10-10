use std::{sync::RwLock, thread, time::Duration};

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
    let tracer = RwLock::new(RunTracer::new());

    thread::scope(|scope| {
        scope.spawn(|| ui(&tracer));
        scope.spawn(|| loop {
            run(&lib, &tracer)
        });
    });
}

#[tokio::main]
async fn ui(_tracer: &RwLock<RunTracer>) {
    let app = Router::new().route("/", get(ws_handler));
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        println!("`{}` connected", user_agent.as_str());
    }

    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    println!("Upgraded to websocket");

    let mut count = 0;

    loop {
        println!("Sending count {count}");

        if socket
            .send(Message::Text(format!("{count}")))
            .await
            .is_err()
        {
            println!("Client disconnected");
            return;
        }

        sleep(Duration::from_secs(3)).await;
        count += 1;
    }
}
