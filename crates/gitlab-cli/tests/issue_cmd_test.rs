use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host)
        .env("GITLAB_TOKEN", "glpat-x")
        .env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn issue_full_lifecycle() {
    let server = MockServer::start().await;
    for (m, p, body) in [
        ("GET", "/api/v4/projects/1/issues", json!([{"iid":1}])),
        ("GET", "/api/v4/projects/1/issues/1", json!({"iid":1})),
        ("POST", "/api/v4/projects/1/issues", json!({"iid":9})),
        (
            "PUT",
            "/api/v4/projects/1/issues/1",
            json!({"iid":1,"state":"closed"}),
        ),
        ("POST", "/api/v4/projects/1/issues/1/move", json!({"iid":1})),
        ("GET", "/api/v4/issues_statistics", json!({"statistics":{}})),
        ("GET", "/api/v4/projects/1/issues/1/links", json!([])),
        (
            "POST",
            "/api/v4/projects/1/issues/1/links",
            json!({"source_issue":{},"target_issue":{}}),
        ),
    ] {
        let status = if m == "POST" { 201 } else { 200 };
        let b = body.clone();
        Mock::given(method(m))
            .and(path(p))
            .respond_with(ResponseTemplate::new(status).set_body_json(&b))
            .mount(&server)
            .await;
    }
    let mock_delete_link_path = "/api/v4/projects/1/issues/1/links/4";
    Mock::given(method("DELETE"))
        .and(path(mock_delete_link_path))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&server)
        .await;

    let base = server.uri();
    env_cmd(&base)
        .args(["issue", "list", "--project", "1"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["issue", "get", "--project", "1", "--issue", "1"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["issue", "create", "--project", "1", "--title", "t"])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "issue",
            "update",
            "--project",
            "1",
            "--issue",
            "1",
            "--data",
            r#"{"state_event":"close"}"#,
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args(["issue", "close", "--project", "1", "--issue", "1"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["issue", "reopen", "--project", "1", "--issue", "1"])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "issue",
            "move",
            "--project",
            "1",
            "--issue",
            "1",
            "--to",
            "2",
        ])
        .assert()
        .success();
    env_cmd(&base).args(["issue", "stats"]).assert().success();
    env_cmd(&base)
        .args([
            "issue",
            "link",
            "--project",
            "1",
            "--issue",
            "1",
            "--target-project",
            "2",
            "--target-issue",
            "7",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "issue",
            "unlink",
            "--project",
            "1",
            "--issue",
            "1",
            "--link-id",
            "4",
        ])
        .assert()
        .success();
}
