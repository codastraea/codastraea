use arpy::ConcurrentRpcClient;
use arpy_reqwasm::websocket;
use futures::StreamExt;
use gloo_console::log;
use gloo_net::websocket::futures::WebSocket;
use serpent_automation_executor::{
    library::FunctionId,
    syntax_tree::{Body, Expression, Statement},
};
use serpent_automation_server_api::ThreadSubscription;

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
    // TODO: Error handling
    log!("Subscribing to thread");
    let ws = websocket::Connection::new(WebSocket::open("ws://127.0.0.1:9090/api").unwrap());

    let mut thread_run_states = ws.subscribe(ThreadSubscription).await.unwrap();

    while let Some(thread_run_state) = thread_run_states.next().await {
        log!(format!("Received: {:?}", thread_run_state));
    }

    log!("Subscription closed");
}
