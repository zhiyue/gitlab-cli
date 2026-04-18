use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::discussions;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OnKind { Issue, Mr, Commit }

impl OnKind {
    fn to_core(self) -> discussions::Kind {
        match self {
            OnKind::Issue => discussions::Kind::Issue,
            OnKind::Mr => discussions::Kind::Mr,
            OnKind::Commit => discussions::Kind::Commit,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum DiscussionCmd {
    List(TargetArgs),
    Get(IdArgs),
    Resolve(IdArgs),
    Unresolve(IdArgs),
}

#[derive(Args, Debug)]
pub struct TargetArgs {
    #[arg(long)] pub project: String,
    #[arg(long)] pub on: OnKind,
    #[arg(long)] pub target: String,
}

#[derive(Args, Debug)]
pub struct IdArgs {
    #[arg(long)] pub project: String,
    #[arg(long)] pub on: OnKind,
    #[arg(long)] pub target: String,
    #[arg(long)] pub id: String,
}

pub async fn run(ctx: Context, cmd: DiscussionCmd) -> Result<()> {
    match cmd {
        DiscussionCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, discussions::list(&a.project, a.on.to_core(), &a.target));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        DiscussionCmd::Get(a) => {
            let v: serde_json::Value = ctx.client.send_json(discussions::get(&a.project, a.on.to_core(), &a.target, &a.id)).await?;
            emit_object(&v)?;
        }
        DiscussionCmd::Resolve(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("resolve discussion {}", a.id))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(discussions::resolve(&a.project, a.on.to_core(), &a.target, &a.id)).await?;
            emit_object(&v)?;
        }
        DiscussionCmd::Unresolve(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("unresolve discussion {}", a.id))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(discussions::unresolve(&a.project, a.on.to_core(), &a.target, &a.id)).await?;
            emit_object(&v)?;
        }
    }
    Ok(())
}
