use assert_cmd::Command;

#[test]
fn config_list_never_prints_raw_token() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("config.toml");
    std::fs::write(
        &cfg,
        r#"
default_host = "gitlab.example.com"
[host."gitlab.example.com"]
token = "glpat-DONOTLEAK1234"
"#,
    )
    .unwrap();
    let output = Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_CONFIG", &cfg)
        .args(["config", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stdout.contains("glpat-DONOTLEAK1234"));
    assert!(!stderr.contains("glpat-DONOTLEAK1234"));
}
