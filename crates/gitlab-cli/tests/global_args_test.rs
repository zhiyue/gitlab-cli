use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn prints_help() {
    let mut cmd = Command::cargo_bin("gitlab").unwrap();
    cmd.arg("--help").assert().success().stdout(contains("gitlab"));
}

#[test]
fn version_flag_works() {
    let mut cmd = Command::cargo_bin("gitlab").unwrap();
    cmd.arg("--version").assert().success().stdout(contains(env!("CARGO_PKG_VERSION")));
}
