use assert_cmd::Command;
use predicates::str::contains;
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
async fn note_on_issue_and_mr() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/issues/3/notes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":7,"body":"hi"}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/issues/3/notes/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":7,"body":"hi"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/merge_requests/5/notes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":88,"body":"lgtm"})))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/projects/1/merge_requests/5/notes/88"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":88,"body":"edited"})))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/v4/projects/1/merge_requests/5/notes/88"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let base = server.uri();
    env_cmd(&base)
        .args([
            "note",
            "list",
            "--project",
            "1",
            "--on",
            "issue",
            "--target",
            "3",
        ])
        .assert()
        .success()
        .stdout(contains("\"id\":7"));
    env_cmd(&base)
        .args([
            "note",
            "get",
            "--project",
            "1",
            "--on",
            "issue",
            "--target",
            "3",
            "--id",
            "7",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "note",
            "create",
            "--project",
            "1",
            "--on",
            "mr",
            "--target",
            "5",
            "--body",
            "lgtm",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "note",
            "update",
            "--project",
            "1",
            "--on",
            "mr",
            "--target",
            "5",
            "--id",
            "88",
            "--body",
            "edited",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "note",
            "delete",
            "--project",
            "1",
            "--on",
            "mr",
            "--target",
            "5",
            "--id",
            "88",
        ])
        .assert()
        .success();
}
