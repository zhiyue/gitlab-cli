use assert_cmd::Command;

fn live_enabled() -> Option<(String, String, String)> {
    let host = std::env::var("GITLAB_TEST_HOST").ok()?;
    let token = std::env::var("GITLAB_TEST_TOKEN").ok()?;
    let project = std::env::var("GITLAB_TEST_PROJECT").ok()?;
    Some((host, token, project))
}

#[test]
#[ignore]
fn live_version() {
    let Some((host, token, _)) = live_enabled() else { return; };
    Command::cargo_bin("gitlab").unwrap()
        .env("GITLAB_HOST", &host).env("GITLAB_TOKEN", &token)
        .arg("version").assert().success();
}

#[test]
#[ignore]
fn live_me() {
    let Some((host, token, _)) = live_enabled() else { return; };
    Command::cargo_bin("gitlab").unwrap()
        .env("GITLAB_HOST", &host).env("GITLAB_TOKEN", &token)
        .arg("me").assert().success();
}

#[test]
#[ignore]
fn live_project_get() {
    let Some((host, token, project)) = live_enabled() else { return; };
    Command::cargo_bin("gitlab").unwrap()
        .env("GITLAB_HOST", &host).env("GITLAB_TOKEN", &token)
        .args(["project","get",&project]).assert().success();
}

#[test]
#[ignore]
fn live_mr_list() {
    let Some((host, token, project)) = live_enabled() else { return; };
    Command::cargo_bin("gitlab").unwrap()
        .env("GITLAB_HOST", &host).env("GITLAB_TOKEN", &token)
        .args(["mr","list","--project",&project,"--limit","5"]).assert().success();
}
