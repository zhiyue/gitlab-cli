# gitlab-cli agent UX improvements (v0.2 mini-plan)

> **For agentic workers:** Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Add three agent-facing improvements: lazy 3-tier `manifest` command, `from-url` URL parser shortcut, conditional `error.hint` field; plus README quirks section.

**Architecture:** All additions are pure CLI-layer (no `gitlab-core` changes). Manifest data lives in a static TOML file embedded via `include_str!`. Error hints live in a small lookup function in `errout.rs`.

**Tech Stack:** Existing — clap derive, serde_json, toml.

---

## Task 1: `manifest` command (3-tier lazy)

**Files:**
- Create: `crates/gitlab-cli/src/cmd/manifest.rs`
- Create: `crates/gitlab-cli/manifest_data.toml`
- Modify: `crates/gitlab-cli/src/cmd/mod.rs` (add `pub mod manifest;`)
- Modify: `crates/gitlab-cli/src/main.rs` (add `Manifest(ManifestArgs)` variant)
- Create: `crates/gitlab-cli/tests/manifest_test.rs`

- [ ] **Step 1: Write the failing test**

`crates/gitlab-cli/tests/manifest_test.rs`:

```rust
use assert_cmd::Command;
use predicates::str::contains;
use serde_json::Value;

fn cmd() -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", "https://example.com").env("GITLAB_TOKEN", "glpat-x");
    c
}

#[test]
fn manifest_index_lists_commands_and_quirks() {
    let out = cmd().arg("manifest").output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["version"].is_string());
    assert!(v["instance"].as_str().unwrap().contains("14.0.5"));
    assert!(v["exit_codes"]["5"].as_str().unwrap().contains("not_found"));
    let cmds = v["commands"].as_array().unwrap();
    assert!(cmds.len() >= 17, "expected ≥17 commands, got {}", cmds.len());
    assert!(cmds.iter().any(|c| c["name"] == "mr"));
    assert!(cmds.iter().any(|c| c["name"] == "api"));
    assert!(v["agent_hints"].as_array().unwrap().len() >= 3);
}

#[test]
fn manifest_command_detail_includes_verbs() {
    let out = cmd().args(["manifest", "mr"]).output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["name"], "mr");
    let verbs = v["verbs"].as_array().unwrap();
    assert!(verbs.iter().any(|x| x["verb"] == "changes"));
    assert!(verbs.iter().any(|x| x["verb"] == "commits"));
    // commits verb has a known quirk
    let commits = verbs.iter().find(|x| x["verb"] == "commits").unwrap();
    assert!(commits.get("quirk").is_some(), "mr commits should document parent_ids quirk");
}

#[test]
fn manifest_verb_detail_returns_single_verb() {
    let out = cmd().args(["manifest", "mr", "changes"]).output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["command"], "mr");
    assert_eq!(v["verb"], "changes");
    assert!(v["example"].is_string());
}

#[test]
fn manifest_unknown_command_404() {
    let out = cmd().args(["manifest", "nonexistent"]).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown command"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test manifest_test`
Expected: FAIL — `manifest` not wired.

- [ ] **Step 3: Implement**

Create `crates/gitlab-cli/manifest_data.toml`:

```toml
# Static enrichment data for `gitlab manifest`.
# Cross-cuts: clap reflection gives us command/verb names; this file gives
# example invocations, return shapes, and known 14.0.5 quirks.

instance_target = "GitLab 14.0.5-ee"

agent_hints = [
  "Set GITLAB_ASSUME_YES=1 (or assume_yes=true in config.toml [host]) to skip the 'type yes' prompt for write commands.",
  "For details on a command, run: gitlab manifest <command> [<verb>]",
  "Errors are emitted to stderr as JSON; check error.code (string vocabulary) and error.hint (when present).",
  "Output is the unmodified GitLab 14.0.5 REST response — fields differ from 17.x.",
  "Default --output is 'json' (single object or array); use 'ndjson' for line-delimited streaming.",
]

# One entry per top-level command. Keep purpose < 80 chars.
[[command]]
name = "version"
purpose = "GitLab instance version + revision"

[[command]]
name = "me"
purpose = "Current authenticated user (GET /user)"

[[command]]
name = "config"
purpose = "Local config: set-token | list | path"

[[command]]
name = "api"
purpose = "Raw HTTP escape hatch — METHOD + path + optional --data/--query"

[[command]]
name = "manifest"
purpose = "Self-describing schema for agents (lazy 3-tier)"

[[command]]
name = "project"
purpose = "Projects (list/get/create/update/delete/fork/archive/unarchive)"

[[command]]
name = "group"
purpose = "Groups (list/get/members/projects/subgroups/create/update/delete)"

[[command]]
name = "mr"
purpose = "Merge requests (list/get/create/update/close/reopen/merge/rebase/approve/unapprove/changes/commits/pipelines)"

[[command]]
name = "issue"
purpose = "Issues (list/get/create/update/close/reopen/move/stats/link/unlink)"

[[command]]
name = "pipeline"
purpose = "Pipelines (list/get/create/retry/cancel/delete/variables)"

[[command]]
name = "job"
purpose = "Jobs (list/get/play/retry/cancel/erase/trace/artifacts) — trace/artifacts return raw bytes"

[[command]]
name = "commit"
purpose = "Commits (list/get/create/diff/comments/statuses/cherry-pick/revert/refs)"

[[command]]
name = "branch"
purpose = "Branches (list/get/create/delete/protect/unprotect)"

[[command]]
name = "tag"
purpose = "Tags (list/get/create/delete/protect/unprotect)"

[[command]]
name = "file"
purpose = "Repository files (get/raw/blame/create/update/delete) — accepts --ref branch|tag|sha"

[[command]]
name = "repo"
purpose = "Repository ops (tree/archive/compare/contributors/merge-base)"

[[command]]
name = "user"
purpose = "Users (list/get/me/keys/emails)"

[[command]]
name = "label"
purpose = "Labels (list/get/create/update/delete/subscribe/unsubscribe)"

[[command]]
name = "note"
purpose = "Comments on issue/mr/commit/snippet (list/get/create/update/delete)"

[[command]]
name = "discussion"
purpose = "Discussion threads on issue/mr/commit (list/get/resolve/unresolve)"

[[command]]
name = "search"
purpose = "Global / group / project search (--scope blobs|issues|... --query)"

# Per-verb enrichment (only entries where example or quirk help).
# Format: [[verb]] command="x" verb="y" example="..." quirk="..." returns="..."

[[verb]]
command = "mr"
verb = "changes"
example = "gitlab mr changes --project group/proj --mr 123"
returns = "Single object with .changes[] containing {old_path, new_path, new_file, deleted_file, renamed_file, diff (unified)}"

[[verb]]
command = "mr"
verb = "commits"
example = "gitlab mr commits --project group/proj --mr 123"
quirk = "parent_ids field is always [] in GitLab 14.0.5. To get parent SHAs, call: gitlab commit get --project <p> --sha <sha>"

[[verb]]
command = "mr"
verb = "approve"
quirk = "EE-only endpoint. May 403 on CE instances or if approver self-approval is blocked."

[[verb]]
command = "file"
verb = "raw"
example = "gitlab file raw --project g/p --path src/foo.rs --ref main  # also accepts tag or commit SHA"
returns = "Raw bytes (no JSON wrapper) — pipe to file or stdout."

[[verb]]
command = "file"
verb = "get"
example = "gitlab file get --project g/p --path README.md --ref <sha>"
returns = "JSON: {file_path, ref, blob_id, commit_id, last_commit_id, size, encoding='base64', content (base64-encoded)}"

[[verb]]
command = "job"
verb = "trace"
example = "gitlab job trace --project g/p --id 12345"
returns = "Raw bytes (CI log, ANSI color codes preserved). Use plain bash redirect to capture."
quirk = "If job is still running, returns partial trace. Poll until status is success|failed|canceled."

[[verb]]
command = "job"
verb = "artifacts"
returns = "Raw bytes (zip archive). 404 → exit 5 if no artifacts."

[[verb]]
command = "api"
verb = "api"
example = "gitlab api GET /projects/123/pipeline_schedules\ngitlab api POST /projects/123/issues --data '{\"title\":\"hi\"}'"
quirk = "Use this for any 14.0.5 endpoint not wrapped by typed commands. Mutating verbs require --yes / GITLAB_ASSUME_YES."

[[verb]]
command = "search"
verb = "search"
example = "gitlab search --scope issues --query bug\ngitlab search --scope blobs --query 'fn main' --project g/p"
returns = "JSON array of hits; shape varies per scope."

# Cross-cutting known quirks (instance-level)
[[quirk]]
area = "mr diffs"
issue = "GET /merge_requests/:iid/diffs returns 404 in 14.0.5 (introduced in 15.7)"
workaround = "Use 'gitlab mr changes' which returns a single object with all file diffs."

[[quirk]]
area = "mr commits parent_ids"
issue = "MR commits endpoint always returns parent_ids=[] in 14.0.5"
workaround = "Use 'gitlab commit get --sha <sha>' to fetch full commit metadata including parent_ids."

[[quirk]]
area = "Project Access Tokens"
issue = "Available in 14.0.5 but CLI only supports user-scoped PATs."
workaround = "Use a user PAT, not a Project Access Token."

[[quirk]]
area = "Pagination"
issue = "Default per_page is 20 (max 100). list commands auto-paginate by default."
workaround = "Use --limit N to cap; use --no-paginate for first page only."

[[quirk]]
area = "Write confirmation"
issue = "Write commands refuse to run without TTY confirmation by default."
workaround = "Set GITLAB_ASSUME_YES=1 env var, or assume_yes=true under [host.\"...\"] in config.toml."
```

Create `crates/gitlab-cli/src/cmd/manifest.rs`:

```rust
use anyhow::{anyhow, Result};
use clap::Args;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::context::Context;
use crate::output::emit_object;

const MANIFEST_DATA: &str = include_str!("../../manifest_data.toml");

#[derive(Args, Debug)]
pub struct ManifestArgs {
    /// Top-level command name to drill into (e.g., 'mr', 'file').
    pub command: Option<String>,
    /// Verb under the command (e.g., 'changes', 'raw').
    pub verb: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ManifestData {
    instance_target: String,
    agent_hints: Vec<String>,
    #[serde(rename = "command")]
    commands: Vec<CommandEntry>,
    #[serde(rename = "verb", default)]
    verbs: Vec<VerbEntry>,
    #[serde(rename = "quirk", default)]
    quirks: Vec<QuirkEntry>,
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
struct CommandEntry {
    name: String,
    purpose: String,
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
struct VerbEntry {
    command: String,
    verb: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    example: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    quirk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    returns: Option<String>,
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
struct QuirkEntry {
    area: String,
    issue: String,
    workaround: String,
}

fn load_data() -> Result<ManifestData> {
    toml::from_str(MANIFEST_DATA).map_err(|e| anyhow!("manifest data parse: {e}"))
}

fn exit_codes() -> Value {
    json!({
        "0": "success",
        "1": "unknown error",
        "2": "invalid_args (clap parse failure)",
        "3": "unauthorized (HTTP 401)",
        "4": "forbidden (HTTP 403)",
        "5": "not_found (HTTP 404)",
        "6": "conflict (HTTP 409 / 422 / bad_request)",
        "7": "rate_limited (HTTP 429 after retries)",
        "8": "server_error (HTTP 5xx after retries)",
        "9": "network or timeout",
        "10": "dry_run completed (no HTTP request issued)"
    })
}

pub fn run(_ctx: Option<&Context>, args: ManifestArgs) -> Result<()> {
    let data = load_data()?;
    match (args.command, args.verb) {
        (None, _) => {
            let v = json!({
                "version": env!("CARGO_PKG_VERSION"),
                "instance": data.instance_target,
                "exit_codes": exit_codes(),
                "commands": data.commands,
                "agent_hints": data.agent_hints,
                "global_quirks_count": data.quirks.len(),
                "next": "Run 'gitlab manifest <command>' for verbs, examples, and quirks."
            });
            emit_object(&v)?;
        }
        (Some(cmd), None) => {
            let entry = data.commands.iter().find(|c| c.name == cmd)
                .ok_or_else(|| anyhow!("unknown command: {cmd}. Run 'gitlab manifest' for the list."))?;
            let verbs: Vec<&VerbEntry> = data.verbs.iter().filter(|v| v.command == cmd).collect();
            let quirks: Vec<&QuirkEntry> = data.quirks.iter()
                .filter(|q| q.area.starts_with(&cmd) || q.area.contains(&format!(" {cmd}")) || q.area.contains(&format!("{cmd} ")))
                .collect();
            let v = json!({
                "name": entry.name,
                "purpose": entry.purpose,
                "verbs": verbs,
                "quirks": quirks,
                "next": format!("Run 'gitlab manifest {cmd} <verb>' for a single verb, or 'gitlab {cmd} --help' for clap-rendered args."),
            });
            emit_object(&v)?;
        }
        (Some(cmd), Some(verb)) => {
            let entry = data.verbs.iter().find(|v| v.command == cmd && v.verb == verb)
                .cloned();
            let v = match entry {
                Some(e) => json!({
                    "command": cmd,
                    "verb": verb,
                    "example": e.example,
                    "returns": e.returns,
                    "quirk": e.quirk,
                    "next": format!("For full args, run: gitlab {cmd} {verb} --help"),
                }),
                None => {
                    // Verb might exist in clap but have no enrichment data — return a minimal entry pointing to --help.
                    if data.commands.iter().any(|c| c.name == cmd) {
                        json!({
                            "command": cmd,
                            "verb": verb,
                            "note": "No enrichment data; verb may exist — check 'gitlab <cmd> <verb> --help' for clap-rendered args.",
                        })
                    } else {
                        return Err(anyhow!("unknown command: {cmd}"));
                    }
                }
            };
            emit_object(&v)?;
        }
    }
    Ok(())
}
```

Wire into `cmd/mod.rs`:

```rust
pub mod manifest;
```

In `main.rs`, add to `Command` enum:

```rust
Manifest(gitlab_cli::cmd::manifest::ManifestArgs),
```

In dispatch (the `match cli.command` arm — note: manifest does NOT need a Context since it doesn't hit GitLab):

```rust
Command::Manifest(args) => {
    return match gitlab_cli::cmd::manifest::run(None, args) {
        Ok(()) => std::process::ExitCode::from(0),
        Err(e) => {
            eprintln!("{{\"error\":{{\"code\":\"invalid_args\",\"message\":\"{}\",\"retryable\":false}}}}", e);
            std::process::ExitCode::from(2)
        }
    };
}
```

Place this BEFORE the Context::build call so manifest works without a configured token.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test manifest_test`
Expected: 4 PASS.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git -c commit.gpgsign=false commit -q -m "feat(cli): manifest command — lazy 3-tier self-describing schema for agents"
```

---

## Task 2: `from-url` URL parser shortcut

**Files:**
- Create: `crates/gitlab-cli/src/cmd/from_url.rs`
- Modify: `crates/gitlab-cli/src/cmd/mod.rs`
- Modify: `crates/gitlab-cli/src/main.rs`
- Create: `crates/gitlab-cli/tests/from_url_test.rs`

- [ ] **Step 1: Write the failing test**

```rust
use assert_cmd::Command;
use serde_json::Value;

fn cmd() -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", "https://example.com").env("GITLAB_TOKEN", "glpat-x");
    c
}

#[test]
fn from_url_mr() {
    let out = cmd().args(["from-url", "https://gitlab.deepwisdomai.com/group/sub/proj/-/merge_requests/123"]).output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "mr");
    assert_eq!(v["project"], "group/sub/proj");
    assert_eq!(v["mr"], 123);
    assert!(v["host"].as_str().unwrap().contains("gitlab.deepwisdomai.com"));
}

#[test]
fn from_url_issue() {
    let out = cmd().args(["from-url", "https://gitlab.example.com/g/p/-/issues/45"]).output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "issue");
    assert_eq!(v["issue"], 45);
}

#[test]
fn from_url_blob() {
    let out = cmd().args(["from-url", "https://gitlab.example.com/g/p/-/blob/main/src/lib.rs"]).output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "file");
    assert_eq!(v["project"], "g/p");
    assert_eq!(v["ref"], "main");
    assert_eq!(v["path"], "src/lib.rs");
}

#[test]
fn from_url_blob_with_sha() {
    let out = cmd().args(["from-url", "https://gitlab.example.com/g/p/-/blob/abc123def/path/to/file.py"]).output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ref"], "abc123def");
    assert_eq!(v["path"], "path/to/file.py");
}

#[test]
fn from_url_commit() {
    let out = cmd().args(["from-url", "https://gitlab.example.com/g/p/-/commit/abc123"]).output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "commit");
    assert_eq!(v["sha"], "abc123");
}

#[test]
fn from_url_pipeline() {
    let out = cmd().args(["from-url", "https://gitlab.example.com/g/p/-/pipelines/9999"]).output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "pipeline");
    assert_eq!(v["pipeline"], 9999);
}

#[test]
fn from_url_project_root() {
    let out = cmd().args(["from-url", "https://gitlab.example.com/g/p"]).output().unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "project");
    assert_eq!(v["project"], "g/p");
}

#[test]
fn from_url_invalid_returns_2() {
    let out = cmd().args(["from-url", "not-a-url"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn from_url_includes_suggested_command() {
    let out = cmd().args(["from-url", "https://gitlab.example.com/g/p/-/merge_requests/5"]).output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["suggested"].as_str().unwrap().contains("gitlab mr"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test from_url_test`
Expected: FAIL.

- [ ] **Step 3: Implement**

Create `crates/gitlab-cli/src/cmd/from_url.rs`:

```rust
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

pub fn run(args: FromUrlArgs) -> Result<()> {
    let url = Url::parse(&args.url).map_err(|e| anyhow!("invalid URL: {e}"))?;
    let host = format!("{}://{}", url.scheme(), url.host_str().unwrap_or(""));
    let path = url.path().trim_start_matches('/').trim_end_matches('/');
    let parsed = parse_path(host.as_str(), path)?;
    emit_object(&parsed)?;
    Ok(())
}

/// Returns a JSON object describing the URL.
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
    let rest = match rest {
        None => {
            out["kind"] = json!("project");
            out["suggested"] = json!(format!("gitlab project get {}", project));
            return Ok(out);
        }
        Some(r) => r,
    };
    let mut parts = rest.splitn(3, '/');
    let kind = parts.next().unwrap_or("");
    match kind {
        "merge_requests" => {
            let iid: u64 = parts.next().ok_or_else(|| anyhow!("missing MR iid"))?
                .parse().map_err(|_| anyhow!("MR iid not a number"))?;
            out["kind"] = json!("mr");
            out["mr"] = json!(iid);
            out["suggested"] = json!(format!("gitlab mr get --project {project} --mr {iid}"));
        }
        "issues" => {
            let iid: u64 = parts.next().ok_or_else(|| anyhow!("missing issue iid"))?
                .parse().map_err(|_| anyhow!("issue iid not a number"))?;
            out["kind"] = json!("issue");
            out["issue"] = json!(iid);
            out["suggested"] = json!(format!("gitlab issue get --project {project} --issue {iid}"));
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
            out["suggested"] = json!(format!(
                "gitlab repo tree --project {project} --ref {rref}"
            ));
        }
        "tags" => {
            let name = parts.next().ok_or_else(|| anyhow!("missing tag name"))?;
            out["kind"] = json!("tag");
            out["tag"] = json!(name);
            out["suggested"] = json!(format!("gitlab tag get --project {project} --name {name}"));
        }
        "pipelines" => {
            let id: u64 = parts.next().ok_or_else(|| anyhow!("missing pipeline id"))?
                .parse().map_err(|_| anyhow!("pipeline id not a number"))?;
            out["kind"] = json!("pipeline");
            out["pipeline"] = json!(id);
            out["suggested"] = json!(format!("gitlab pipeline get --project {project} --id {id}"));
        }
        "jobs" => {
            let id: u64 = parts.next().ok_or_else(|| anyhow!("missing job id"))?
                .parse().map_err(|_| anyhow!("job id not a number"))?;
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
```

Wire into `cmd/mod.rs`:

```rust
pub mod from_url;
```

In `main.rs` `Command` enum:

```rust
#[command(name = "from-url")]
FromUrl(gitlab_cli::cmd::from_url::FromUrlArgs),
```

In dispatch (also outside async runtime, like `manifest`):

```rust
Command::FromUrl(args) => {
    return match gitlab_cli::cmd::from_url::run(args) {
        Ok(()) => std::process::ExitCode::from(0),
        Err(e) => {
            eprintln!("{{\"error\":{{\"code\":\"invalid_args\",\"message\":\"{}\",\"retryable\":false}}}}", e);
            std::process::ExitCode::from(2)
        }
    };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test from_url_test`
Expected: 9 PASS.

- [ ] **Step 5: Commit**

```bash
git -c commit.gpgsign=false commit -q -m "feat(cli): from-url command — parse GitLab web URLs into project/iid/sha/path"
```

---

## Task 3: Conditional `error.hint` field + README known quirks section

**Files:**
- Modify: `crates/gitlab-cli/src/errout.rs` (add hint lookup + emit)
- Modify: `crates/gitlab-core/src/error.rs` (add `hint: Option<String>` to `ErrorPayload`)
- Modify: `README.md` (add "Known 14.0.5 API quirks" section)
- Modify: `crates/gitlab-cli/tests/version_me_test.rs` (extend existing 401 test to assert hint)

- [ ] **Step 1: Write the failing test**

Add to `crates/gitlab-cli/tests/version_me_test.rs`:

```rust
#[tokio::test]
async fn unauthorized_error_includes_hint() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/user"))
        .respond_with(ResponseTemplate::new(401).set_body_string("401 Unauthorized"))
        .mount(&server).await;
    let host = server.uri();
    let out = assert_cmd::Command::cargo_bin("gitlab").unwrap()
        .env("GITLAB_HOST", &host).env("GITLAB_TOKEN", "glpat-bad")
        .arg("me")
        .output().unwrap();
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert!(v["error"]["hint"].as_str().unwrap_or("").to_lowercase().contains("token"),
        "expected unauthorized hint mentioning 'token', got: {stderr}");
}

#[tokio::test]
async fn not_found_error_includes_hint() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/x"))
        .respond_with(ResponseTemplate::new(404).set_body_string("{\"message\":\"404 Project Not Found\"}"))
        .mount(&server).await;
    let host = server.uri();
    let out = assert_cmd::Command::cargo_bin("gitlab").unwrap()
        .env("GITLAB_HOST", &host).env("GITLAB_TOKEN", "glpat-x")
        .args(["project", "get", "x"])
        .output().unwrap();
    assert_eq!(out.status.code(), Some(5));
    let v: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    let hint = v["error"]["hint"].as_str().unwrap_or("");
    assert!(!hint.is_empty(), "404 should produce a hint");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gitlab-cli --test version_me_test unauthorized_error_includes_hint not_found_error_includes_hint`
Expected: FAIL — `hint` field not present.

- [ ] **Step 3: Implement**

Modify `crates/gitlab-core/src/error.rs` `ErrorPayload`:

```rust
#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: ErrorCode,
    pub status: Option<u16>,
    pub message: String,
    pub request_id: Option<String>,
    pub retryable: bool,
    pub details: serde_json::Value,
    /// Human-readable suggestion for fixing this specific error class. Set by the CLI layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}
```

Update both construction sites in `error.rs::to_payload()` to set `hint: None` (CLI layer fills it in).

Modify `crates/gitlab-cli/src/errout.rs`:

```rust
use gitlab_core::error::{ErrorCode, GitlabError};
use std::io::{self, Write};

pub fn report_error(err: &GitlabError) -> i32 {
    let mut payload = err.to_payload();
    payload.hint = lookup_hint(err);
    let body = serde_json::json!({ "error": payload });
    let stderr = io::stderr();
    let mut lock = stderr.lock();
    let _ = writeln!(
        lock,
        "{}",
        serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string())
    );
    err.exit_code()
}

fn lookup_hint(err: &GitlabError) -> Option<String> {
    let (code, msg) = match err {
        GitlabError::Http { code, message, .. } => (*code, message.as_str()),
        _ => return None,
    };
    let m = msg.to_lowercase();
    let hint = match code {
        ErrorCode::Unauthorized =>
            "Token is missing, expired, or revoked. Verify with: gitlab config list  (token shown masked). \
             Regenerate at: <host>/-/profile/personal_access_tokens",
        ErrorCode::Forbidden if m.contains("approve") =>
            "MR approval requires GitLab EE license and may be blocked for self-approval. \
             Try a different reviewer's PAT.",
        ErrorCode::Forbidden =>
            "Token lacks required scope, or you're not a member with sufficient role. \
             Check token scopes at: <host>/-/profile/personal_access_tokens",
        ErrorCode::NotFound if m.contains("file") =>
            "File not found at this ref. If the file was deleted, try the parent commit: \
             gitlab commit get --project <p> --sha <last-known-good-sha>  (then read .parent_ids[0])",
        ErrorCode::NotFound if m.contains("ref") || m.contains("commit") =>
            "Ref/commit not found. Check spelling, or that you have access to that branch.",
        ErrorCode::NotFound if m.contains("project") =>
            "Project not found or PAT lacks access. Verify path-with-namespace is correct (case-sensitive).",
        ErrorCode::NotFound =>
            "Resource not found. Verify ids/paths and that your PAT has access.",
        ErrorCode::Conflict if m.contains("already") =>
            "Resource already exists or in conflicting state. Re-fetch current state before retrying.",
        ErrorCode::Conflict =>
            "Validation failed. Check 'details' field for which fields are invalid.",
        ErrorCode::RateLimited =>
            "GitLab rate limit hit. CLI already retried with backoff. Reduce parallelism or set --rps 5.",
        ErrorCode::ServerError =>
            "GitLab returned 5xx. CLI retried automatically. If persistent, check instance status.",
        ErrorCode::BadRequest if m.contains("not allowed") =>
            "Operation not allowed in current state (e.g., merging closed MR, deleting protected branch).",
        ErrorCode::BadRequest =>
            "Bad request shape. Check 'details' for field-level validation errors.",
        _ => return None,
    };
    Some(hint.to_string())
}
```

Modify `README.md` — add a new section before `## License`:

```markdown
## Known GitLab 14.0.5 API quirks

These are server-side behaviors of GitLab 14.0.5-ee that surprise agents. CLI does not paper over them — they're documented here so you know what to expect:

| Area | What happens | Workaround |
|---|---|---|
| `mr commits.parent_ids` | Always returns `[]` | Use `gitlab commit get --sha <id>` to fetch full commit including parent_ids |
| `mr diffs` endpoint | 404 (introduced in 15.7) | Use `gitlab mr changes` (single object with all file diffs) |
| `/raw_diffs` endpoint | 404 (introduced in 16.4) | Use `gitlab mr changes` and read each file's `.diff` field |
| Project Access Tokens | Available but CLI doesn't support | Use a user-scoped PAT |
| `users` endpoint extras | Some fields missing vs newer versions | Use `gitlab api GET /user/...` with explicit field selection if needed |
| Pagination caps | `per_page` max is 100 | CLI auto-paginates; use `--limit N` to cap |
| MR `approve` (EE) | 403 if license expired | Approval requires EE license + reviewer PAT |
| Write confirmation | TTY prompt blocks scripts | Set `GITLAB_ASSUME_YES=1` or `assume_yes=true` per host in config.toml |

Run `gitlab manifest` (and `gitlab manifest <command>`) for a JSON-formatted view of these quirks plus per-command examples — agents should consume that rather than this table.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p gitlab-cli --test version_me_test`
Expected: previous 4 tests + 2 new pass (6 total).

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git -c commit.gpgsign=false commit -q -m "feat(cli): conditional error.hint field + README quirks section

- ErrorPayload gains optional 'hint' field (skipped when None, no JSON bloat)
- errout.rs::lookup_hint() maps (ErrorCode, message) to fix suggestions
  for the 10 most common error patterns
- README documents 8 known 14.0.5 server-side quirks"
```

---

## Acceptance criteria

- [ ] `gitlab manifest` runs without auth (works before `config set-token`)
- [ ] `gitlab manifest mr` lists 13+ verbs with the `commits.parent_ids` quirk explicit
- [ ] `gitlab from-url <mr-url>` returns kind=mr + project + iid + suggested
- [ ] 401/404 errors include a `hint` field with actionable text
- [ ] `cargo test --workspace` ≥ 95 tests pass (existing 81 + ~13 new)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
