use clap::{Parser, Subcommand};
use gitlab_cli::context::{CliInputs, Context};
use gitlab_cli::errout::report_error;
use gitlab_cli::globals::GlobalArgs;
use gitlab_cli::tracing_setup;

#[derive(Parser)]
#[command(name = "gitlab", version, about = "gitlab-cli for GitLab 14.0.5-ee", propagate_version = true)]
struct Cli {
    #[command(flatten)]
    globals: GlobalArgs,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Version,
    Me,
    Config {
        #[command(subcommand)]
        cmd: gitlab_cli::cmd::config::ConfigCmd,
    },
    #[command(name = "api")]
    Api(gitlab_cli::cmd::api::ApiArgs),
    Project {
        #[command(subcommand)]
        cmd: gitlab_cli::cmd::project::ProjectCmd,
    },
    Group {
        #[command(subcommand)]
        cmd: gitlab_cli::cmd::group::GroupCmd,
    },
    Mr {
        #[command(subcommand)]
        cmd: gitlab_cli::cmd::mr::MrCmd,
    },
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    tracing_setup::init(cli.globals.verbose.as_deref());

    let config_text = read_config_text(&cli.globals);

    let result: Result<(), anyhow::Error> = match cli.command {
        Command::Config { cmd } => gitlab_cli::cmd::config::run(cmd, cli.globals.config.clone()),
        other => {
            let ctx = match Context::build(CliInputs { globals: cli.globals.clone(), config_text }) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{{\"error\":{{\"code\":\"invalid_args\",\"message\":\"{e}\",\"retryable\":false}}}}");
                    return std::process::ExitCode::from(2);
                }
            };
            let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
            rt.block_on(async {
                match other {
                    Command::Version => gitlab_cli::cmd::version::run(ctx).await,
                    Command::Me => gitlab_cli::cmd::me::run(ctx).await,
                    Command::Config { .. } => unreachable!(),
                    Command::Api(args) => gitlab_cli::cmd::api::run(ctx, args).await,
                    Command::Project { cmd } => gitlab_cli::cmd::project::run(ctx, cmd).await,
                    Command::Group { cmd } => gitlab_cli::cmd::group::run(ctx, cmd).await,
                    Command::Mr { cmd } => gitlab_cli::cmd::mr::run(ctx, cmd).await,
                }
            })
        }
    };

    match result {
        Ok(()) => std::process::ExitCode::from(0),
        Err(e) => {
            if let Some(ge) = e.downcast_ref::<gitlab_core::error::GitlabError>() {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let code = report_error(ge) as u8;
                std::process::ExitCode::from(code)
            } else {
                eprintln!("{{\"error\":{{\"code\":\"unknown\",\"message\":\"{e}\",\"retryable\":false}}}}");
                std::process::ExitCode::from(1)
            }
        }
    }
}

fn read_config_text(globals: &GlobalArgs) -> String {
    let path = globals
        .config
        .clone()
        .or_else(gitlab_core::config::Config::default_config_path);
    path.and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default()
}
