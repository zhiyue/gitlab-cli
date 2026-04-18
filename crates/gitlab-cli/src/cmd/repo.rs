use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::repos;
use std::io::Write;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};

#[derive(Subcommand, Debug)]
pub enum RepoCmd {
    Tree(TreeArgs),
    Archive(ArchiveArgs),
    Compare(CompareArgs),
    Contributors(PrjArg),
    #[command(name = "merge-base")]
    MergeBase(MergeBaseArgs),
}

#[derive(Args, Debug)]
pub struct TreeArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub path: Option<String>,
    #[arg(long = "ref")]
    pub rref: Option<String>,
    #[arg(long)]
    pub recursive: bool,
}

#[derive(Args, Debug)]
pub struct ArchiveArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub sha: Option<String>,
    #[arg(long)]
    pub format: Option<String>,
}

#[derive(Args, Debug)]
pub struct CompareArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub from: String,
    #[arg(long)]
    pub to: String,
}

#[derive(Args, Debug)]
pub struct PrjArg {
    #[arg(long)]
    pub project: String,
}

#[derive(Args, Debug)]
pub struct MergeBaseArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long = "ref", num_args = 1.., required = true)]
    pub refs: Vec<String>,
}

pub async fn run(ctx: Context, cmd: RepoCmd) -> Result<()> {
    match cmd {
        RepoCmd::Tree(a) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                repos::tree(
                    &a.project,
                    a.path.as_deref(),
                    a.rref.as_deref(),
                    a.recursive,
                ),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        RepoCmd::Archive(a) => {
            let (_s, _h, bytes) = ctx
                .client
                .send_raw(repos::archive(
                    &a.project,
                    a.sha.as_deref(),
                    a.format.as_deref(),
                ))
                .await?;
            std::io::stdout().write_all(&bytes).ok();
        }
        RepoCmd::Compare(a) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(repos::compare(&a.project, &a.from, &a.to))
                .await?;
            emit_object(&v)?;
        }
        RepoCmd::Contributors(p) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                repos::contributors(&p.project),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        RepoCmd::MergeBase(a) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(repos::merge_base(&a.project, &a.refs))
                .await?;
            emit_object(&v)?;
        }
    }
    Ok(())
}
