use bincode::Options;
use futures::StreamExt;
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use serpent_automation_executor::{
    library::FunctionId,
    run::ThreadRunState,
    syntax_tree::{Body, Expression, Statement},
};

pub mod call_tree;
pub mod tree;

pub fn expression_is_expandable(expression: &Expression<FunctionId>) -> bool {
    match expression {
        Expression::Variable { .. } | Expression::Literal(_) => false,
        Expression::Call { .. } => true,
    }
}

pub fn statement_is_expandable(stmt: &Statement<FunctionId>) -> bool {
    match stmt {
        Statement::Pass => false,
        Statement::Expression(e) => expression_is_expandable(e),
        Statement::If { .. } => true,
    }
}

// TODO: Make this a method on `Body`
pub fn is_expandable(body: &Body<FunctionId>) -> bool {
    body.iter().any(statement_is_expandable)
}

pub async fn server_connection() {
    log!("Connecting to websocket");
    let mut server_ws = WebSocket::open("ws://127.0.0.1:9090/").unwrap_or_else(|e| {
        log!(format!("Error: {}", e));
        // TODO: Handle error
        panic!("Error connecting to websocket");
    });

    while let Some(msg) = server_ws.next().await {
        log!(format!("Received: {:?}", msg));

        let _run_state = match msg.unwrap() {
            Message::Text(text) => {
                let run_state: ThreadRunState = serde_json_wasm::from_str(&text).unwrap();
                log!(format!("Deserialized `RunTracer` from `{text}`"));
                run_state
            }
            Message::Bytes(bytes) => bincode::options().deserialize(&bytes).unwrap(),
        };
    }

    log!("WebSocket Closed")
}
