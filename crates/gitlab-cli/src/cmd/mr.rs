use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::merge_requests as mr;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Subcommand, Debug)]
pub enum MrCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Update(UpdateArgs),
    Close(Target),
    Reopen(Target),
    Merge(MergeArgs),
    Rebase(Target),
    Approve(Target),
    Unapprove(Target),
    Changes(Target),
    Commits(Target),
    Pipelines(Target),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long, conflicts_with = "group")]
    pub project: Option<String>,
    #[arg(long)]
    pub group: Option<String>,
    #[arg(long)]
    pub state: Option<String>,
}

#[derive(Args, Debug)]
pub struct Target {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub mr: u64,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub source: String,
    #[arg(long)]
    pub target: String,
    #[arg(long)]
    pub title: String,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub mr: u64,
    #[arg(long)]
    pub data: String,
}

#[derive(Args, Debug)]
pub struct MergeArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub mr: u64,
    #[arg(long)]
    pub squash: bool,
}

#[allow(clippy::too_many_lines)]
pub async fn run(ctx: Context, cmd: MrCmd) -> Result<()> {
    match cmd {
        MrCmd::List(a) => {
            let req = match (a.project, a.group) {
                (Some(p), None) => mr::list_for_project(&p, a.state.as_deref()),
                (None, Some(g)) => mr::list_for_group(&g, a.state.as_deref()),
                _ => anyhow::bail!("pass either --project or --group"),
            };
            let stream = PagedStream::<serde_json::Value>::start(&ctx.client, req);
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        MrCmd::Get(t) => {
            let v: serde_json::Value = ctx.client.send_json(mr::get_spec(&t.project, t.mr)).await?;
            emit_object(&v)?;
        }
        MrCmd::Create(a) => {
            let spec = mr::create_spec(&a.project, &a.source, &a.target, &a.title);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(),
                    path: spec.path.clone(),
                    query: spec.query.clone(),
                    body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, "create MR")? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        MrCmd::Update(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            let spec = mr::update_spec(&a.project, a.mr, &body);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(),
                    path: spec.path.clone(),
                    query: spec.query.clone(),
                    body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, &format!("update MR {}", a.mr))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        MrCmd::Close(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(mr::close_spec(&t.project, t.mr))
                .await?;
            emit_object(&v)?;
        }
        MrCmd::Reopen(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(mr::reopen_spec(&t.project, t.mr))
                .await?;
            emit_object(&v)?;
        }
        MrCmd::Merge(a) => {
            let spec = mr::merge_spec(&a.project, a.mr, a.squash);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(),
                    path: spec.path.clone(),
                    query: spec.query.clone(),
                    body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, &format!("merge MR {}", a.mr))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        MrCmd::Rebase(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(mr::rebase_spec(&t.project, t.mr))
                .await?;
            emit_object(&v)?;
        }
        MrCmd::Approve(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(mr::approve_spec(&t.project, t.mr))
                .await?;
            emit_object(&v)?;
        }
        MrCmd::Unapprove(t) => {
            let _ = ctx
                .client
                .send_raw(mr::unapprove_spec(&t.project, t.mr))
                .await?;
        }
        MrCmd::Changes(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(mr::changes_spec(&t.project, t.mr))
                .await?;
            emit_object(&v)?;
        }
        MrCmd::Commits(t) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                mr::commits_page(&t.project, t.mr),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        MrCmd::Pipelines(t) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                mr::pipelines_page(&t.project, t.mr),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
    }
    Ok(())
}
