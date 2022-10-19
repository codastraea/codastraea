use std::{cell::RefCell, collections::HashMap, rc::Rc};

use futures::StreamExt;
use futures_signals::signal::Mutable;
use gloo_console::log;
use gloo_net::websocket::{futures::WebSocket, Message};
use serpent_automation_executor::{
    library::FunctionId,
    run::{CallStack, FnStatus, RunTracer},
    syntax_tree::{Expression, Statement},
};

fn expression_is_expandable(expression: &Expression<FunctionId>) -> bool {
    match expression {
        Expression::Variable { .. } => false,
        Expression::Call { .. } => true,
    }
}

pub fn statement_is_expandable(stmt: &Statement<FunctionId>) -> bool {
    match stmt {
        Statement::Pass => false,
        Statement::Expression(e) => expression_is_expandable(e),
    }
}

pub fn is_expandable(stmts: &[Statement<FunctionId>]) -> bool {
    stmts.iter().any(statement_is_expandable)
}

pub async fn server_connection(run_states: RunStates) {
    log!("Connecting to websocket");
    let mut server_ws = WebSocket::open("ws://127.0.0.1:9090/").unwrap();

    while let Some(msg) = server_ws.next().await {
        log!(format!("Received: {:?}", msg));

        match msg.unwrap() {
            Message::Text(text) => {
                let run_tracer: RunTracer = serde_json_wasm::from_str(&text).unwrap();
                log!(format!("Deserialized `RunTracer` from `{text}`"));

                // TODO: Share current `run_tracer` so we can get status of newly expanded
                // function bodies.
                for (call_stack, status) in run_states.borrow().iter() {
                    log!(format!("call stack {:?}", call_stack));
                    status.set_neq(run_tracer.status(call_stack));
                }
            }
            Message::Bytes(_) => log!("Unknown binary message"),
        }
    }

    log!("WebSocket Closed")
}

// TODO: Struct for this
pub type RunStates = Rc<RefCell<HashMap<CallStack, Mutable<FnStatus>>>>;
