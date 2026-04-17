use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::groups;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Subcommand, Debug)]
pub enum GroupCmd {
    List(ListArgs),
    Get { id: String },
    Members { id: String },
    Projects { id: String },
    Subgroups { id: String },
    Create(CreateArgs),
    Update(UpdateArgs),
    Delete { id: String },
}

#[derive(Args, Debug)]
pub struct ListArgs { #[arg(long)] pub search: Option<String> }

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)] pub name: String,
    #[arg(long)] pub path: String,
    #[arg(long)] pub parent_id: Option<u64>,
}

#[derive(Args, Debug)]
pub struct UpdateArgs { pub id: String, #[arg(long)] pub data: String }

pub async fn run(ctx: Context, cmd: GroupCmd) -> Result<()> {
    match cmd {
        GroupCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, groups::list_spec(a.search.as_deref()));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        GroupCmd::Get { id } => {
            let v: serde_json::Value = ctx.client.send_json(groups::get_spec(&id)).await?;
            emit_object(&v)?;
        }
        GroupCmd::Members { id } => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, groups::members_spec(&id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        GroupCmd::Projects { id } => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, groups::projects_spec(&id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        GroupCmd::Subgroups { id } => {
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, groups::subgroups_spec(&id));
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        GroupCmd::Create(a) => {
            let spec = groups::create_spec(&a.name, &a.path, a.parent_id);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(), path: spec.path.clone(),
                    query: spec.query.clone(), body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, "create group")? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        GroupCmd::Update(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            let spec = groups::update_spec(&a.id, &body);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(), path: spec.path.clone(),
                    query: spec.query.clone(), body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, &format!("update group {}", a.id))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        GroupCmd::Delete { id } => {
            let spec = groups::delete_spec(&id);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(), path: spec.path.clone(),
                    query: spec.query.clone(), body: None,
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, &format!("delete group {id}"))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(spec).await?;
        }
    }
    Ok(())
}
