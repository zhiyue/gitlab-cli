use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::labels;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Subcommand, Debug)]
pub enum LabelCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Update(UpdateArgs),
    Delete(Target),
    Subscribe(Target),
    Unsubscribe(Target),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)] pub project: String,
}

#[derive(Args, Debug)]
pub struct Target {
    #[arg(long)] pub project: String,
    #[arg(long)] pub id: u64,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)] pub project: String,
    #[arg(long)] pub name: String,
    #[arg(long)] pub color: String,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    #[arg(long)] pub project: String,
    #[arg(long)] pub id: u64,
    #[arg(long)] pub data: String,
}

pub async fn run(ctx: Context, cmd: LabelCmd) -> Result<()> {
    match cmd {
        LabelCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, labels::list(&a.project));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        LabelCmd::Get(t) => {
            let v: serde_json::Value = ctx.client.send_json(labels::get(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
        LabelCmd::Create(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("create label {}", a.name))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(labels::create(&a.project, &a.name, &a.color)).await?;
            emit_object(&v)?;
        }
        LabelCmd::Update(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            if !confirm_or_skip(ctx.assume_yes, &format!("update label {}", a.id))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(labels::update(&a.project, a.id, body)).await?;
            emit_object(&v)?;
        }
        LabelCmd::Delete(t) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete label {}", t.id))? {
                anyhow::bail!("aborted");
            }
            let _ = ctx.client.send_raw(labels::delete(&t.project, t.id)).await?;
        }
        LabelCmd::Subscribe(t) => {
            let v: serde_json::Value = ctx.client.send_json(labels::subscribe(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
        LabelCmd::Unsubscribe(t) => {
            let v: serde_json::Value = ctx.client.send_json(labels::unsubscribe(&t.project, t.id)).await?;
            emit_object(&v)?;
        }
    }
    Ok(())
}
