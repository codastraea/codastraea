use arpy::ConcurrentRpcClient;
use arpy_reqwasm::websocket;
use futures::Stream;
use gloo_net::websocket::futures::WebSocket;
use serpent_automation_executor::{
    library::FunctionId,
    run::{CallStack, RunState},
    syntax_tree::{Body, Expression, Statement},
};
use serpent_automation_server_api::ThreadSubscription;
use tokio_stream::StreamExt;

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

pub struct ServerConnection {
    ws: websocket::Connection,
}

impl Default for ServerConnection {
    fn default() -> Self {
        let ws = websocket::Connection::new(WebSocket::open("ws://127.0.0.1:9090/api").unwrap());

        Self { ws }
    }
}

impl ServerConnection {
    pub async fn subscribe(
        &self,
        opened_nodes: impl Stream<Item = CallStack> + 'static,
    ) -> impl Stream<Item = (CallStack, RunState)> {
        // TODO: Error handling
        let ((), subscription) = self
            .ws
            .subscribe(ThreadSubscription, opened_nodes)
            .await
            .unwrap();

        subscription.map_while(Result::ok)
    }
}
