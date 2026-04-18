use anyhow::{anyhow, Result};
use clap::Args;
use gitlab_core::page::PagedStream;
use gitlab_core::resources::search;

use crate::context::Context;
use crate::output::emit_stream;

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[arg(long)] pub scope: String,
    #[arg(long)] pub query: String,
    #[arg(long, conflicts_with = "group")] pub project: Option<String>,
    #[arg(long)] pub group: Option<String>,
}

pub async fn run(ctx: Context, a: SearchArgs) -> Result<()> {
    let req = match (a.project, a.group) {
        (Some(p), None) => search::project(&p, &a.scope, &a.query),
        (None, Some(g)) => search::group(&g, &a.scope, &a.query),
        (None, None) => search::global(&a.scope, &a.query),
        (Some(_), Some(_)) => return Err(anyhow!("--project and --group are mutually exclusive")),
    };
    let stream = PagedStream::<serde_json::Value>::start(&ctx.client, req);
    emit_stream(stream, ctx.output, ctx.limit).await?;
    Ok(())
}
