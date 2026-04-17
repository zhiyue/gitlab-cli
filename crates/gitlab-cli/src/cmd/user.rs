use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::users;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};

#[derive(Subcommand, Debug)]
pub enum UserCmd {
    List(ListArgs),
    Get(IdArgs),
    Me,
    Keys(IdArgs),
    Emails(IdArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)] pub search: Option<String>,
}

#[derive(Args, Debug)]
pub struct IdArgs {
    #[arg(long)] pub id: u64,
}

pub async fn run(ctx: Context, cmd: UserCmd) -> Result<()> {
    match cmd {
        UserCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, users::list(a.search.as_deref()));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        UserCmd::Get(a) => {
            let v: serde_json::Value = ctx.client.send_json(users::get(a.id)).await?;
            emit_object(&v)?;
        }
        UserCmd::Me => {
            let v: serde_json::Value = ctx.client.send_json(users::me()).await?;
            emit_object(&v)?;
        }
        UserCmd::Keys(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, users::keys(a.id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        UserCmd::Emails(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, users::emails(a.id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
    }
    Ok(())
}
