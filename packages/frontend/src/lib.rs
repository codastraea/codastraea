use arpy::ConcurrentRpcClient;
use arpy_reqwasm::websocket;
use codastraea_server_api::{NodeVecDiff, WatchCallTree};
use futures::{stream, Stream, StreamExt};
use gloo_net::websocket::futures::WebSocket;

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
    pub async fn watch(
        &self,
        watch_call_tree: WatchCallTree,
    ) -> impl Stream<Item = NodeVecDiff> + use<> + 'static {
        // TODO: Error handling
        let subscription = self.ws.subscribe(watch_call_tree, stream::empty()).await;
        let ((), updates) = subscription.expect("TODO: Error handling");
        Box::pin(updates.filter_map(|update| async { update.ok() }))
    }
}
