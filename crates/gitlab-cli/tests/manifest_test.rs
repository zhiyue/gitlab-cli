use assert_cmd::Command;
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
