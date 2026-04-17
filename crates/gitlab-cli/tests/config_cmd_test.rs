use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn config_path_prints_resolved_path() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_CONFIG", &cfg_path)
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(contains(cfg_path.to_string_lossy().as_ref()));
}

#[test]
fn config_set_token_writes_file() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_CONFIG", &cfg_path)
        .args(["config", "set-token", "--host", "gitlab.example.com", "--token", "glpat-AA"])
        .assert()
        .success();
    let text = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(text.contains("gitlab.example.com"));
    assert!(text.contains("glpat-AA"));
}

#[test]
fn config_list_masks_tokens() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");
    std::fs::write(&cfg_path, r#"
default_host = "gitlab.example.com"
[host."gitlab.example.com"]
token = "glpat-ABCDEFGHIJKL"
"#).unwrap();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_CONFIG", &cfg_path)
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(contains("glpa****IJKL"))
        .stdout(predicates::str::contains("glpat-ABCDEFGHIJKL").not());
}
