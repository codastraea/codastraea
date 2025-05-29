use std::pin::pin;

use arpy::ConcurrentRpcClient;
use arpy_reqwasm::websocket;
use clonelet::clone;
use futures::{stream, Stream};
use futures_signals::signal_vec::{MutableVec, MutableVecLockMut, SignalVec};
use gloo_net::websocket::futures::WebSocket;
use serpent_automation_executor::{
    library::FunctionId,
    run::{CallStack, RunState},
    syntax_tree::{Body, Expression, Statement},
};
use serpent_automation_server_api::{NodeUpdate, ThreadSubscription, WatchCallTree};
use silkenweb_task::spawn_local;
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

// TODO: Does this need to be `Clone`?
#[derive(Clone)]
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

    // TODO: Is `WatchCallTree` a good name? We're watching nodes.
    pub fn watch(
        &self,
        watch_call_tree: WatchCallTree,
    ) -> impl SignalVec<Item = NodeUpdate> + use<> + 'static {
        let nodes = MutableVec::new();

        spawn_local({
            clone!(nodes);
            let ws = self.ws.clone();

            async move {
                // TODO: Error handling
                let subscription = ws.subscribe(watch_call_tree, stream::empty());
                let ((), updates) = subscription.await.expect("TODO: Error handling");
                let mut updates = pin!(updates.map_while(Result::ok));

                while let Some(update) = updates.next().await {
                    MutableVecLockMut::apply_vec_diff(&mut nodes.lock_mut(), update)
                }
            }
        });

        nodes.signal_vec_cloned()
    }
}
