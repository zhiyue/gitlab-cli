use anyhow::Result;
use clap::{Args, Subcommand};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::issues;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::{confirm_or_skip, dry_run_envelope, Intent};

#[derive(Subcommand, Debug)]
pub enum IssueCmd {
    List(ListArgs),
    Get(Target),
    Create(CreateArgs),
    Update(UpdateArgs),
    Close(Target),
    Reopen(Target),
    Move(MoveArgs),
    Stats,
    Link(LinkArgs),
    Unlink(UnlinkArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub state: Option<String>,
}

#[derive(Args, Debug)]
pub struct Target {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub issue: u64,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub labels: Option<String>,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub issue: u64,
    #[arg(long)]
    pub data: String,
}

#[derive(Args, Debug)]
pub struct MoveArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub issue: u64,
    #[arg(long)]
    pub to: String,
}

#[derive(Args, Debug)]
pub struct LinkArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub issue: u64,
    #[arg(long)]
    pub target_project: String,
    #[arg(long)]
    pub target_issue: u64,
}

#[derive(Args, Debug)]
pub struct UnlinkArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub issue: u64,
    #[arg(long)]
    pub link_id: u64,
}

pub async fn run(ctx: Context, cmd: IssueCmd) -> Result<()> {
    match cmd {
        IssueCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                issues::list_for_project(&a.project, a.state.as_deref()),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        IssueCmd::Get(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(issues::get_spec(&t.project, t.issue))
                .await?;
            emit_object(&v)?;
        }
        IssueCmd::Create(a) => {
            let spec = issues::create_spec(&a.project, &a.title, a.labels.as_deref());
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(),
                    path: spec.path.clone(),
                    query: spec.query.clone(),
                    body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, "create issue")? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        IssueCmd::Update(a) => {
            let body = crate::cmd::load_json(&a.data)?;
            let spec = issues::update_spec(&a.project, a.issue, &body);
            if ctx.dry_run {
                emit_object(&dry_run_envelope(&Intent {
                    method: spec.method.clone(),
                    path: spec.path.clone(),
                    query: spec.query.clone(),
                    body: spec.body.clone(),
                }))?;
                std::process::exit(10);
            }
            if !confirm_or_skip(ctx.assume_yes, &format!("update issue {}", a.issue))? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        IssueCmd::Close(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(issues::close_spec(&t.project, t.issue))
                .await?;
            emit_object(&v)?;
        }
        IssueCmd::Reopen(t) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(issues::reopen_spec(&t.project, t.issue))
                .await?;
            emit_object(&v)?;
        }
        IssueCmd::Move(a) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(issues::move_spec(&a.project, a.issue, &a.to))
                .await?;
            emit_object(&v)?;
        }
        IssueCmd::Stats => {
            let v: serde_json::Value = ctx.client.send_json(issues::stats_spec()).await?;
            emit_object(&v)?;
        }
        IssueCmd::Link(a) => {
            let spec = issues::link_spec(&a.project, a.issue, &a.target_project, a.target_issue);
            if !confirm_or_skip(ctx.assume_yes, "link issue")? {
                anyhow::bail!("aborted");
            }
            let v: serde_json::Value = ctx.client.send_json(spec).await?;
            emit_object(&v)?;
        }
        IssueCmd::Unlink(a) => {
            let spec = issues::unlink_spec(&a.project, a.issue, a.link_id);
            if !confirm_or_skip(ctx.assume_yes, "unlink issue")? {
                anyhow::bail!("aborted");
            }
            let _ = ctx.client.send_raw(spec).await?;
        }
    }
    Ok(())
}
