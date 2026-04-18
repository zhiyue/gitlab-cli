use anyhow::{anyhow, Result};
use clap::Args;
use serde_json::{json, Value};
use url::Url;

use crate::output::emit_object;

#[derive(Args, Debug)]
pub struct FromUrlArgs {
    /// GitLab web URL (project, MR, issue, commit, blob, branch, pipeline, etc.)
    pub url: String,
}

pub fn run(args: &FromUrlArgs) -> Result<()> {
    let url = Url::parse(&args.url).map_err(|e| anyhow!("invalid URL: {e}"))?;
    let host = format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""));
    let path = url.path().trim_start_matches('/').trim_end_matches('/');
    let parsed = parse_path(&host, path)?;
    emit_object(&parsed)?;
    Ok(())
}

/// Returns a JSON object describing the URL.
#[allow(clippy::too_many_lines)]
fn parse_path(host: &str, path: &str) -> Result<Value> {
    // GitLab URL shape: <namespace…>/<project>/-/<kind>/<rest>
    // If "-" separator missing, the whole path is the project.
    let (project, rest) = match path.split_once("/-/") {
        Some((p, r)) => (p, Some(r)),
        None => (path, None),
    };
    if project.is_empty() {
        return Err(anyhow!("URL has no project segment"));
    }
    let mut out = json!({
        "host": host,
        "project": project,
    });
    let Some(rest) = rest else {
        out["kind"] = json!("project");
        out["suggested"] = json!(format!("gitlab project get {project}"));
        return Ok(out);
    };
    let mut parts = rest.splitn(3, '/');
    let kind = parts.next().unwrap_or("");
    match kind {
        "merge_requests" => {
            let iid: u64 = parts
                .next()
                .ok_or_else(|| anyhow!("missing MR iid"))?
                .parse()
                .map_err(|_| anyhow!("MR iid not a number"))?;
            out["kind"] = json!("mr");
            out["mr"] = json!(iid);
            out["suggested"] = json!(format!("gitlab mr get --project {project} --mr {iid}"));
        }
        "issues" => {
            let iid: u64 = parts
                .next()
                .ok_or_else(|| anyhow!("missing issue iid"))?
                .parse()
                .map_err(|_| anyhow!("issue iid not a number"))?;
            out["kind"] = json!("issue");
            out["issue"] = json!(iid);
            out["suggested"] = json!(format!(
                "gitlab issue get --project {project} --issue {iid}"
            ));
        }
        "commit" => {
            let sha = parts.next().ok_or_else(|| anyhow!("missing commit sha"))?;
            out["kind"] = json!("commit");
            out["sha"] = json!(sha);
            out["suggested"] = json!(format!("gitlab commit get --project {project} --sha {sha}"));
        }
        "blob" | "raw" => {
            let rref = parts.next().ok_or_else(|| anyhow!("missing ref"))?;
            let file_path = parts.next().ok_or_else(|| anyhow!("missing file path"))?;
            out["kind"] = json!("file");
            out["ref"] = json!(rref);
            out["path"] = json!(file_path);
            out["suggested"] = json!(format!(
                "gitlab file raw --project {project} --path {file_path} --ref {rref}"
            ));
        }
        "tree" => {
            let rref = parts.next().unwrap_or("HEAD");
            out["kind"] = json!("tree");
            out["ref"] = json!(rref);
            out["suggested"] = json!(format!("gitlab repo tree --project {project} --ref {rref}"));
        }
        "tags" => {
            let name = parts.next().ok_or_else(|| anyhow!("missing tag name"))?;
            out["kind"] = json!("tag");
            out["tag"] = json!(name);
            out["suggested"] = json!(format!("gitlab tag get --project {project} --name {name}"));
        }
        "pipelines" => {
            let id: u64 = parts
                .next()
                .ok_or_else(|| anyhow!("missing pipeline id"))?
                .parse()
                .map_err(|_| anyhow!("pipeline id not a number"))?;
            out["kind"] = json!("pipeline");
            out["pipeline"] = json!(id);
            out["suggested"] = json!(format!("gitlab pipeline get --project {project} --id {id}"));
        }
        "jobs" => {
            let id: u64 = parts
                .next()
                .ok_or_else(|| anyhow!("missing job id"))?
                .parse()
                .map_err(|_| anyhow!("job id not a number"))?;
            out["kind"] = json!("job");
            out["job"] = json!(id);
            out["suggested"] = json!(format!("gitlab job get --project {project} --id {id}"));
        }
        other => {
            out["kind"] = json!("unknown");
            out["raw_kind"] = json!(other);
            out["suggested"] = json!(format!("gitlab project get {project}"));
        }
    }
    Ok(out)
}
