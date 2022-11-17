use bincode::Options;
use futures::StreamExt;
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use serpent_automation_executor::{
    library::FunctionId,
    run::ThreadCallStates,
    syntax_tree::{Body, Expression, Statement},
};

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

pub fn is_expandable(body: &Body<FunctionId>) -> bool {
    body.iter().any(statement_is_expandable)
}

pub async fn server_connection(receive_call_states: impl ReceiveCallStates) {
    log!("Connecting to websocket");
    let mut server_ws = WebSocket::open("ws://178.79.165.198:9090/").unwrap();

    while let Some(msg) = server_ws.next().await {
        log!(format!("Received: {:?}", msg));

        let call_states = match msg.unwrap() {
            Message::Text(text) => {
                let call_states: ThreadCallStates = serde_json_wasm::from_str(&text).unwrap();
                log!(format!("Deserialized `RunTracer` from `{text}`"));
                call_states
            }
            Message::Bytes(bytes) => bincode::options().deserialize(&bytes).unwrap(),
        };

        receive_call_states.set_call_states(call_states);
    }

    log!("WebSocket Closed")
}

pub trait ReceiveCallStates {
    fn set_call_states(&self, thread_state: ThreadCallStates);
}
