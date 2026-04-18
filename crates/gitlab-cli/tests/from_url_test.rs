use assert_cmd::Command;
use serde_json::Value;

fn cmd() -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", "https://example.com")
        .env("GITLAB_TOKEN", "glpat-x");
    c
}

#[test]
fn from_url_mr() {
    let out = cmd()
        .args([
            "from-url",
            "https://gitlab.deepwisdomai.com/group/sub/proj/-/merge_requests/123",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "mr");
    assert_eq!(v["project"], "group/sub/proj");
    assert_eq!(v["mr"], 123);
    assert!(v["host"]
        .as_str()
        .unwrap()
        .contains("gitlab.deepwisdomai.com"));
}

#[test]
fn from_url_issue() {
    let out = cmd()
        .args(["from-url", "https://gitlab.example.com/g/p/-/issues/45"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "issue");
    assert_eq!(v["issue"], 45);
}

#[test]
fn from_url_blob() {
    let out = cmd()
        .args([
            "from-url",
            "https://gitlab.example.com/g/p/-/blob/main/src/lib.rs",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "file");
    assert_eq!(v["project"], "g/p");
    assert_eq!(v["ref"], "main");
    assert_eq!(v["path"], "src/lib.rs");
}

#[test]
fn from_url_blob_with_sha() {
    let out = cmd()
        .args([
            "from-url",
            "https://gitlab.example.com/g/p/-/blob/abc123def/path/to/file.py",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ref"], "abc123def");
    assert_eq!(v["path"], "path/to/file.py");
}

#[test]
fn from_url_commit() {
    let out = cmd()
        .args(["from-url", "https://gitlab.example.com/g/p/-/commit/abc123"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "commit");
    assert_eq!(v["sha"], "abc123");
}

#[test]
fn from_url_pipeline() {
    let out = cmd()
        .args([
            "from-url",
            "https://gitlab.example.com/g/p/-/pipelines/9999",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["kind"], "pipeline");
    assert_eq!(v["pipeline"], 9999);
}

#[test]
fn from_url_project_root() {
    let out = cmd()
        .args(["from-url", "https://gitlab.example.com/g/p"])
        .output()
        .unwrap();
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
    let out = cmd()
        .args([
            "from-url",
            "https://gitlab.example.com/g/p/-/merge_requests/5",
        ])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["suggested"].as_str().unwrap().contains("gitlab mr"));
}
