use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use serpent_automation_wasm_host::runtime::Container;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The WASM module. This can be text or binary format
    file: PathBuf,
}

fn main() -> Result<()> {
    let wat_file = Args::parse().file;
    let mut container = Container::from_file(&wat_file)?;

    container.register_workflows()?;
    container.init_workflow(0)?;

    for _i in 0..5 {
        container.run()?;
        println!("Checkpoint (pre snapshot)");
    }

    let snapshot = container.snapshot()?;

    while container.run()? {
        println!("Checkpoint (post snapshot)");
    }

    drop(container);

    let mut container = Container::from_file(&wat_file)?;
    container.restore(&snapshot)?;

    while container.run()? {
        println!("Checkpoint (post restore)");
    }

    Ok(())
}
