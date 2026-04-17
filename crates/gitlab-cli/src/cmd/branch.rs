use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::branches;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum BranchCmd {
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
    #[arg(long)]
    pub search: Option<String>,
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

pub async fn run(ctx: Context, cmd: BranchCmd) -> Result<()> {
    match cmd {
        BranchCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                branches::list(&a.project, a.search.as_deref()),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        BranchCmd::Get(t) => {
            let v: serde_json::Value = ctx.client.send_json(branches::get(&t.project, &t.name)).await?;
            emit_object(&v)?;
        }
        BranchCmd::Create(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("create branch {}", a.name))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(branches::create(&a.project, &a.name, &a.rref)).await?;
            emit_object(&v)?;
        }
        BranchCmd::Delete(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete branch {}", t.name))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(branches::delete(&t.project, &t.name)).await?;
        }
        BranchCmd::Protect(t) => {
            let v: serde_json::Value = ctx.client.send_json(branches::protect(&t.project, &t.name)).await?;
            emit_object(&v)?;
        }
        BranchCmd::Unprotect(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("unprotect branch {}", t.name))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(branches::unprotect(&t.project, &t.name)).await?;
        }
    }
    Ok(())
}
