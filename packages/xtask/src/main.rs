use clap::Parser;
use xtask_base::{
    ci::{StandardVersions, CI},
    cmd, in_workspace, CommonCmds,
};

#[derive(Parser)]
enum Commands {
    UI {
        #[arg(long)]
        release: bool,
    },
    Serve {
        #[arg(long)]
        release: bool,
    },
    #[clap(flatten)]
    Common(CommonCmds),
}

fn main() {
    in_workspace(|workspace| {
        type Cmds = Commands;

        match Cmds::parse() {
            Cmds::UI { release } => {
                let release = release.then_some("--release");
                cmd!("trunk serve {release...} --open")
                    .dir("packages/ui")
                    .run()?
            }
            Cmds::Serve { release } => {
                let release = release.then_some("--release");
                cmd!("cargo build {release...} --target wasm32-unknown-unknown")
                    .dir("packages/test-workflow")
                    .run()?;
                cmd!("cargo run {release...} -- ../../target/wasm32-unknown-unknown/debug/serpent_automation_test_workflow.wasm")
                    .dir("packages/server")
                    .run()?
            }
            Cmds::Common(cmds) => cmds.sub_command::<Cmds>(
                workspace,
                [],
                CI::standard_workflow(
                    StandardVersions {
                        rustc_stable_version: "1.87.0",
                        rustc_nightly_version: "nightly-2025-03-15",
                        udeps_version: "0.1.55",
                    },
                    &[],
                ),
                |_| Ok(()),
            )?,
        }

        Ok(())
    });
}
