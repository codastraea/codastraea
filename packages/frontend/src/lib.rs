use std::pin::pin;

use arpy::ConcurrentRpcClient;
use arpy_reqwasm::websocket;
use clonelet::clone;
use futures::stream;
use futures_signals::signal_vec::{MutableVec, MutableVecLockMut, SignalVec};
use gloo_net::websocket::futures::WebSocket;
use serpent_automation_server_api::{NodeUpdate, WatchCallTree};
use silkenweb_task::spawn_local;
use tokio_stream::StreamExt;

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
