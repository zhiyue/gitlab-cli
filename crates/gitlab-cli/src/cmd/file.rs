use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::resources::files;
use std::io::Write;

use crate::context::Context;
use crate::output::emit_object;
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum FileCmd {
    Get(GetArgs),
    Raw(GetArgs),
    Blame(GetArgs),
    Create(WriteArgs),
    Update(WriteArgs),
    Delete(DeleteArgs),
}

#[derive(Args, Debug)]
pub struct GetArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub path: String,
    #[arg(long = "ref")]
    pub rref: String,
}

#[derive(Args, Debug)]
pub struct WriteArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub path: String,
    #[arg(long)]
    pub branch: String,
    #[arg(long)]
    pub content: String,
    #[arg(long)]
    pub message: String,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub path: String,
    #[arg(long)]
    pub branch: String,
    #[arg(long)]
    pub message: String,
}

pub async fn run(ctx: Context, cmd: FileCmd) -> Result<()> {
    match cmd {
        FileCmd::Get(a) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(files::get(&a.project, &a.path, &a.rref))
                .await?;
            emit_object(&v)?;
        }
        FileCmd::Raw(a) => {
            let (_s, _h, bytes) = ctx
                .client
                .send_raw(files::raw(&a.project, &a.path, &a.rref))
                .await?;
            std::io::stdout().write_all(&bytes).ok();
        }
        FileCmd::Blame(a) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(files::blame(&a.project, &a.path, &a.rref))
                .await?;
            emit_object(&v)?;
        }
        FileCmd::Create(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("create file {}", a.path))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx
                .client
                .send_json(files::create(
                    &a.project, &a.path, &a.branch, &a.content, &a.message,
                ))
                .await?;
            emit_object(&v)?;
        }
        FileCmd::Update(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("update file {}", a.path))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx
                .client
                .send_json(files::update(
                    &a.project, &a.path, &a.branch, &a.content, &a.message,
                ))
                .await?;
            emit_object(&v)?;
        }
        FileCmd::Delete(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete file {}", a.path))? {
                anyhow::bail!("aborted");
            }
            let _ = ctx
                .client
                .send_raw(files::delete(&a.project, &a.path, &a.branch, &a.message))
                .await?;
        }
    }
    Ok(())
}
