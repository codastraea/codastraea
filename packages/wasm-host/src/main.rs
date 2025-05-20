use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use serpent_automation_wasm_host::runtime;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The WASM module. This can be text or binary format
    file: PathBuf,
}

fn main() -> Result<()> {
    runtime::run(&Args::parse().file)
}
