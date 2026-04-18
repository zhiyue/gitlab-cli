use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::commits;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum CommitCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Diff(Target),
    Comments(Target),
    Statuses(Target),
    #[command(name = "cherry-pick")]
    CherryPick(PickArgs),
    Revert(PickArgs),
    Refs(Target),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long = "ref")]
    pub rref: Option<String>,
}

#[derive(Args, Debug)]
pub struct Target {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub sha: String,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub data: String,
}

#[derive(Args, Debug)]
pub struct PickArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub sha: String,
    #[arg(long)]
    pub branch: String,
}

pub async fn run(ctx: Context, cmd: CommitCmd) -> Result<()> {
    match cmd {
        CommitCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                commits::list(&a.project, a.rref.as_deref()),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        CommitCmd::Get(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(commits::get(&t.project, &t.sha))
                .await?;
            emit_object(&v)?;
        }
        CommitCmd::Create(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            if !confirm_or_skip(ctx.assume_yes, "create commit")? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx
                .client
                .send_json(commits::create(&a.project, &body))
                .await?;
            emit_object(&v)?;
        }
        CommitCmd::Diff(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(commits::diff(&t.project, &t.sha))
                .await?;
            emit_object(&v)?;
        }
        CommitCmd::Comments(t) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                commits::comments(&t.project, &t.sha),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        CommitCmd::Statuses(t) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                commits::statuses(&t.project, &t.sha),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        CommitCmd::CherryPick(a) => {
            if !confirm_or_skip(
                ctx.assume_yes,
                &format!("cherry-pick {} onto {}", a.sha, a.branch),
            )? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx
                .client
                .send_json(commits::cherry_pick(&a.project, &a.sha, &a.branch))
                .await?;
            emit_object(&v)?;
        }
        CommitCmd::Revert(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("revert {} on {}", a.sha, a.branch))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx
                .client
                .send_json(commits::revert(&a.project, &a.sha, &a.branch))
                .await?;
            emit_object(&v)?;
        }
        CommitCmd::Refs(t) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                commits::refs(&t.project, &t.sha),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
    }
    Ok(())
}
