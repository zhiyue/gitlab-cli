use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::resources::projects;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Subcommand, Debug)]
pub enum ProjectCmd {
    /// List accessible projects (auto-paginates)
    List(ListArgs),
    /// Get a single project by id or full path
    Get { id: String },
    /// Create a project
    Create(CreateArgs),
    /// Update a project
    Update(UpdateArgs),
    /// Delete a project
    Delete { id: String },
    /// Fork a project
    Fork { id: String },
    /// Archive a project
    Archive { id: String },
    /// Unarchive a project
    Unarchive { id: String },
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)] pub visibility: Option<String>,
    #[arg(long)] pub search: Option<String>,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)] pub name: String,
    #[arg(long)] pub path: Option<String>,
    #[arg(long)] pub visibility: Option<String>,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    pub id: String,
    /// JSON body; prefix @file to load from disk
    #[arg(long)]
    pub data: String,
}

pub async fn run(ctx: Context, cmd: ProjectCmd) -> Result<()> {
    match cmd {
        ProjectCmd::List(a) => {
            let req = projects::list_spec(a.visibility.as_deref(), a.search.as_deref());
            let stream = projects::stream(&ctx.client, req);
            let fmt = ctx.output;
            let limit = ctx.limit;
            emit_stream::<serde_json::Value, _>(stream, fmt, limit).await?;
        }
        ProjectCmd::Get { id } => {
            let v: serde_json::Value = ctx.client.send_json(projects::get_spec(&id)).await?;
            emit_object(&v)?;
        }
        ProjectCmd::Create(a) => {
            let spec = projects::create_spec(&a.name, a.path.as_deref(), a.visibility.as_deref());
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(), path: spec.path.clone(),
                    query: spec.query.clone(), body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, "create project")? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        ProjectCmd::Update(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            let spec = projects::update_spec(&a.id, &body);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(), path: spec.path.clone(),
                    query: spec.query.clone(), body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, &format!("update project {}", a.id))? { anyhow::bail!("aborted"); }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        ProjectCmd::Delete { id } => {
            let spec = projects::delete_spec(&id);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(), path: spec.path.clone(),
                    query: spec.query.clone(), body: None,
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, &format!("delete project {id}"))? { anyhow::bail!("aborted"); }
            let _ = ctx.client.send_raw(spec).await?;
        }
        ProjectCmd::Fork { id } => {
            let v: serde_json::Value = ctx.client.send_json(projects::fork_spec(&id)).await?;
            emit_object(&v)?;
        }
        ProjectCmd::Archive { id } => {
            let v: serde_json::Value = ctx.client.send_json(projects::archive_spec(&id)).await?;
            emit_object(&v)?;
        }
        ProjectCmd::Unarchive { id } => {
            let v: serde_json::Value = ctx.client.send_json(projects::unarchive_spec(&id)).await?;
            emit_object(&v)?;
        }
    }
    Ok(())
}
