use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn prints_help() {
    let mut cmd = Command::cargo_bin("gitlab").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(contains("gitlab"));
}

#[test]
fn version_flag_works() {
    let mut cmd = Command::cargo_bin("gitlab").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn version_string_contains_target_triple() {
    let mut cmd = Command::cargo_bin("gitlab").unwrap();
    let out = cmd.arg("--version").output().unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        s.contains(env!("CARGO_PKG_VERSION")),
        "missing version: {s}"
    );
    assert!(s.contains("target="), "missing target in --version: {s}");
}
