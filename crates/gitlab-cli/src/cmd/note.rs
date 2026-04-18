use anyhow::{anyhow, Result};
use clap::{Args, Subcommand, ValueEnum};
use gitlab_core::page::PagedStream;
use gitlab_core::resources::notes;

use crate::context::Context;
use crate::output::{emit_object, emit_stream};
use crate::safety::confirm_or_skip;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum OnKind {
    Issue,
    Mr,
    Commit,
    Snippet,
}

impl OnKind {
    fn to_core(self) -> notes::Kind {
        match self {
            OnKind::Issue => notes::Kind::Issue,
            OnKind::Mr => notes::Kind::Mr,
            OnKind::Commit => notes::Kind::Commit,
            OnKind::Snippet => notes::Kind::Snippet,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum NoteCmd {
    List(ListArgs),
    Get(GetArgs),
    Create(CreateArgs),
    Update(UpdateArgs),
    Delete(GetArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub on: OnKind,
    #[arg(long)]
    pub target: String,
}

#[derive(Args, Debug)]
pub struct GetArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub on: OnKind,
    #[arg(long)]
    pub target: String,
    #[arg(long)]
    pub id: u64,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub on: OnKind,
    #[arg(long)]
    pub target: String,
    #[arg(long)]
    pub body: String,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub on: OnKind,
    #[arg(long)]
    pub target: String,
    #[arg(long)]
    pub id: u64,
    #[arg(long)]
    pub body: String,
}

pub async fn run(ctx: Context, cmd: NoteCmd) -> Result<()> {
    match cmd {
        NoteCmd::List(a) => {
            let stream = PagedStream::<serde_json::Value>::start(
                &ctx.client,
                notes::list(&a.project, a.on.to_core(), &a.target),
            );
            emit_stream(stream, ctx.output, ctx.limit).await?;
        }
        NoteCmd::Get(a) => {
            let v: serde_json::Value = ctx
                .client
                .send_json(notes::get(&a.project, a.on.to_core(), &a.target, a.id))
                .await?;
            emit_object(&v)?;
        }
        NoteCmd::Create(a) => {
            if !confirm_or_skip(ctx.assume_yes, "create note")? {
                return Err(anyhow!("aborted"));
            }
            let v: serde_json::Value = ctx
                .client
                .send_json(notes::create(
                    &a.project,
                    a.on.to_core(),
                    &a.target,
                    &a.body,
                ))
                .await?;
            emit_object(&v)?;
        }
        NoteCmd::Update(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("update note {}", a.id))? {
                return Err(anyhow!("aborted"));
            }
            let v: serde_json::Value = ctx
                .client
                .send_json(notes::update(
                    &a.project,
                    a.on.to_core(),
                    &a.target,
                    a.id,
                    &a.body,
                ))
                .await?;
            emit_object(&v)?;
        }
        NoteCmd::Delete(a) => {
            if !confirm_or_skip(ctx.assume_yes, &format!("delete note {}", a.id))? {
                return Err(anyhow!("aborted"));
            }
            let _ = ctx
                .client
                .send_raw(notes::delete(&a.project, a.on.to_core(), &a.target, a.id))
                .await?;
        }
    }
    Ok(())
}
