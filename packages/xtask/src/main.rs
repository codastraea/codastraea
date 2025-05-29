use clap::Parser;
use xtask_base::{
    ci::{StandardVersions, CI},
    cmd, in_workspace, CommonCmds,
};

#[derive(Parser)]
enum Commands {
    UI,
    // TODO: Add a release flag
    Serve,
    #[clap(flatten)]
    Common(CommonCmds),
}

fn main() {
    in_workspace(|workspace| {
        type Cmds = Commands;

        match Cmds::parse() {
            Cmds::UI => cmd!("trunk serve --open").dir("packages/ui").run()?,
            Cmds::Serve => {
                cmd!("cargo build --target wasm32-unknown-unknown")
                    .dir("packages/test-workflow")
                    .run()?;
                cmd!("cargo run -- ../../target/wasm32-unknown-unknown/debug/serpent_automation_test_workflow.wasm")
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
