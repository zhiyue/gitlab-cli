use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::jobs;
use std::io::Write;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum JobCmd {
    List(ListArgs),
    Get(Target),
    Play(Target),
    Retry(Target),
    Cancel(Target),
    Erase(Target),
    Trace(Target),
    Artifacts(Target),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub pipeline: Option<u64>,
    #[arg(long)]
    pub scope: Option<String>,
}

#[derive(Args, Debug)]
pub struct Target {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub id: u64,
}

pub async fn run(ctx: Context, cmd: JobCmd) -> Result<()> {
    match cmd {
        JobCmd::List(a) => {
            let req = match a.pipeline {
                Some(pid) => jobs::list_pipeline(&a.project, pid),
                None => jobs::list_project(&a.project, a.scope.as_deref()),
            };
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, req);
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        JobCmd::Get(t) => {
            let v: serde_json::Value = ctx.client.send_json(jobs::get(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
        JobCmd::Play(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("play job {}", t.id))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(jobs::play(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
        JobCmd::Retry(t) => {
            let v: serde_json::Value = ctx.client.send_json(jobs::retry(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
        JobCmd::Cancel(t) => {
            let v: serde_json::Value = ctx.client.send_json(jobs::cancel(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
        JobCmd::Erase(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("erase job {}", t.id))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(jobs::erase(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
        JobCmd::Trace(t) => {
            let (_s, _h, bytes) = ctx.client.send_raw(jobs::trace(&t.project, t.id)).await?;
            std::io::stdout().write_all(&bytes).ok();
        }
        JobCmd::Artifacts(t) => {
            let (_s, _h, bytes) = ctx
                .client
                .send_raw(jobs::artifacts(&t.project, t.id))
                .await?;
            std::io::stdout().write_all(&bytes).ok();
        }
    }
    Ok(())
}
