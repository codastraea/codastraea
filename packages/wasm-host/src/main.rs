use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use serpent_automation_wasm_host::runtime;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The WASM module. This can be text or binary format
    file: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let wat = fs::read(&args.file).context(format!("Opening file {:?}", args.file))?;
    runtime::run(&wat)
}
