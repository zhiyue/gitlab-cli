use anyhow::Result;
use gitlab_core::request::RequestSpec;
use reqwest::Method;

use crate::context::Context;
use crate::output::emit_object;

pub async fn run(ctx: Context) -> Result<()> {
    let v: serde_json::Value = ctx.client.send_json(RequestSpec::new(Method::GET, "version")).await?;
    emit_object(&v)?;
    Ok(())
}
