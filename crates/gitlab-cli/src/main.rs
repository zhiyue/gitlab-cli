use clap::{Parser, Subcommand};

mod errout;
mod globals;
mod output;

use globals::GlobalArgs;

#[derive(Parser)]
#[command(
    name = "gitlab",
    version,
    about = "gitlab-cli: agent-first CLI for GitLab EE 14.0.5",
    long_about = None,
    propagate_version = true
)]
struct Cli {
    #[command(flatten)]
    globals: GlobalArgs,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Print GitLab instance version
    Version,
    /// Print current user
    Me,
}

fn main() -> std::process::ExitCode {
    let _cli = Cli::parse();
    std::process::ExitCode::from(0)
}
