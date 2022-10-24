use futures::StreamExt;
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use serpent_automation_executor::{
    library::FunctionId,
    run::ThreadCallStates,
    syntax_tree::{Body, Expression, Statement},
};

fn expression_is_expandable(expression: &Expression<FunctionId>) -> bool {
    match expression {
        Expression::Variable { .. } | Expression::Literal(_) => false,
        Expression::Call { .. } => true,
    }
}

fn body_is_expandable(body: &Body<FunctionId>) -> bool {
    body.iter().any(statement_is_expandable)
}

pub fn statement_is_expandable(stmt: &Statement<FunctionId>) -> bool {
    match stmt {
        Statement::Pass => false,
        Statement::Expression(e) => expression_is_expandable(e),
        Statement::If {
            condition,
            then_block,
            else_block,
        } => {
            expression_is_expandable(condition)
                || body_is_expandable(then_block)
                || body_is_expandable(else_block)
        }
    }
}

pub fn is_expandable(body: &Body<FunctionId>) -> bool {
    body.iter().any(statement_is_expandable)
}

pub async fn server_connection(receive_call_states: impl ReceiveCallStates) {
    log!("Connecting to websocket");
    let mut server_ws = WebSocket::open("ws://127.0.0.1:9090/").unwrap();

    while let Some(msg) = server_ws.next().await {
        log!(format!("Received: {:?}", msg));

        match msg.unwrap() {
            Message::Text(text) => {
                let call_states: ThreadCallStates = serde_json_wasm::from_str(&text).unwrap();
                log!(format!("Deserialized `RunTracer` from `{text}`"));
                receive_call_states.set_call_states(call_states);
            }
            Message::Bytes(_) => log!("Unknown binary message"),
        }
    }

    log!("WebSocket Closed")
}

pub trait ReceiveCallStates {
    fn set_call_states(&self, thread_state: ThreadCallStates);
}
