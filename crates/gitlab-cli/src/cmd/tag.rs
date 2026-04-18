use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::tags;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum TagCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Delete(Target),
    Protect(Target),
    Unprotect(Target),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub project: String,
}

#[derive(Args, Debug)]
pub struct Target {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub name: String,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub name: String,
    #[arg(long = "ref")]
    pub rref: String,
}

pub async fn run(ctx: Context, cmd: TagCmd) -> Result<()> {
    match cmd {
        TagCmd::List(a) => {
            let stream =
                PagedStream::<serde_json::Value>::start(&ctx.client, tags::list(&a.project));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        TagCmd::Get(t) => {
            let v: serde_json::Value = ctx.client.send_json(tags::get(&t.project, &t.name)).await?;
            emit_object(&v)?;
        }
        TagCmd::Create(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("create tag {}", a.name))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx
                .client
                .send_json(tags::create(&a.project, &a.name, &a.rref))
                .await?;
            emit_object(&v)?;
        }
        TagCmd::Delete(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete tag {}", t.name))? {
                anyhow::bail!("aborted");
            }
            let _ = ctx
                .client
                .send_raw(tags::delete(&t.project, &t.name))
                .await?;
        }
        TagCmd::Protect(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(tags::protect(&t.project, &t.name))
                .await?;
            emit_object(&v)?;
        }
        TagCmd::Unprotect(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("unprotect tag {}", t.name))? {
                anyhow::bail!("aborted");
            }
            let _ = ctx
                .client
                .send_raw(tags::unprotect(&t.project, &t.name))
                .await?;
        }
    }
    Ok(())
}
