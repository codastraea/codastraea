use std::{path::PathBuf, thread};

use anyhow::Result;
use arpy_axum::RpcRoute;
use arpy_server::WebSocketRouter;
use axum::{Router, Server};
use clap::Parser;
use codastraea_server_api::WatchCallTree;
use codastraea_wasm_host::runtime::Container;
use futures::stream::BoxStream;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The WASM module. This can be text or binary format
    file: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let wat_file = Args::parse().file;
    let mut container = Container::from_file(&wat_file)?;

    container.register_workflows()?;
    container.init_workflow("codastraea_test_workflow", "counter")?;
    let node_store = container.node_store();

    thread::spawn({
        move || {
            // TODO: Handle errors
            while container.run().expect("TODO") {
                println!("Checkpoint");
            }
        }
    });

    let ws = WebSocketRouter::new().handle_subscription({
        move |_updates: BoxStream<'static, ()>, watch: WatchCallTree| {
            let updates = node_store.watch(watch.id());
            ((), updates)
        }
    });

    let app = Router::new().ws_rpc_route("/api", ws, 10000);
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
