use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::pipelines;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Subcommand, Debug)]
pub enum PipelineCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Retry(Target),
    Cancel(Target),
    Delete(Target),
    Variables(Target),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(Args, Debug)]
pub struct Target {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub id: u64,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long = "ref")]
    pub rref: String,
}

pub async fn run(ctx: Context, cmd: PipelineCmd) -> Result<()> {
    match cmd {
        PipelineCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                pipelines::list(&a.project, a.status.as_deref()),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        PipelineCmd::Get(t) => {
            let v: serde_json::Value = ctx.client.send_json(pipelines::get(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
        PipelineCmd::Create(a) => {
            let spec = pipelines::create(&a.project, &a.rref);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(),
                    path: spec.path.clone(),
                    query: spec.query.clone(),
                    body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, "create pipeline")? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        PipelineCmd::Retry(t) => {
            let v: serde_json::Value = ctx.client.send_json(pipelines::retry(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
        PipelineCmd::Cancel(t) => {
            let v: serde_json::Value = ctx.client.send_json(pipelines::cancel(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
        PipelineCmd::Delete(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete pipeline {}", t.id))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(pipelines::delete(&t.project, t.id)).await?;
        }
        PipelineCmd::Variables(t) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                pipelines::variables(&t.project, t.id),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
    }
    Ok(())
}
