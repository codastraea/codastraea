use std::{path::PathBuf, thread};

use anyhow::Result;
use arpy_axum::RpcRoute;
use arpy_server::WebSocketRouter;
use axum::{Router, Server};
use clap::Parser;
use clonelet::clone;
use futures::stream::BoxStream;
use futures_signals::signal_vec::SignalVecExt;
use serpent_automation_executor::{
    library::Library,
    run::{CallStack, ThreadRunState},
    syntax_tree::parse,
    CODE,
};
use serpent_automation_server_api::{ThreadSubscription, WatchCallTree};
use serpent_automation_wasm_host::runtime::Container;
use tokio::spawn;
use tokio_stream::wrappers::ReceiverStream;

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
    container.init_workflow(0)?;
    let call_tree = container.call_tree();

    thread::spawn({
        move || {
            // TODO: Handle errors
            while container.run().expect("TODO") {
                println!("Checkpoint");
            }
        }
    });

    let thread_run_state = ThreadRunState::default();
    thread::spawn({
        clone!(thread_run_state);

        move || {
            let lib = Library::link(parse(CODE).unwrap());
            lib.run(&thread_run_state);
        }
    });

    let ws = WebSocketRouter::new()
        .handle_subscription({
            move |updates: BoxStream<'static, CallStack>, _subscription: ThreadSubscription| {
                let (run_state_receiver, update_client) = thread_run_state.subscribe(updates);
                spawn(update_client);
                ((), ReceiverStream::new(run_state_receiver))
            }
        })
        .handle_subscription({
            move |_updates: BoxStream<'static, ()>, watch: WatchCallTree| {
                let updates = call_tree.watch(watch.path());
                ((), updates.to_stream())
            }
        });

    let app = Router::new().ws_rpc_route("/api", ws, 10000);
    Server::bind(&"0.0.0.0:9090".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
