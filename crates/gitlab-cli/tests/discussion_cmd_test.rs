use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host)
        .env("GITLAB_TOKEN", "glpat-x")
        .env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn discussion_on_mr() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/merge_requests/5/discussions"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!([{"id":"abcd","resolved":false}])),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/merge_requests/5/discussions/abcd"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"abcd"})))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/projects/1/merge_requests/5/discussions/abcd"))
        .and(query_param("resolved", "true"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"id":"abcd","resolved":true})),
        )
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/projects/1/merge_requests/5/discussions/abcd"))
        .and(query_param("resolved", "false"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"id":"abcd","resolved":false})),
        )
        .mount(&server)
        .await;

    let base = server.uri();
    env_cmd(&base)
        .args([
            "discussion",
            "list",
            "--project",
            "1",
            "--on",
            "mr",
            "--target",
            "5",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "discussion",
            "get",
            "--project",
            "1",
            "--on",
            "mr",
            "--target",
            "5",
            "--id",
            "abcd",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "discussion",
            "resolve",
            "--project",
            "1",
            "--on",
            "mr",
            "--target",
            "5",
            "--id",
            "abcd",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "discussion",
            "unresolve",
            "--project",
            "1",
            "--on",
            "mr",
            "--target",
            "5",
            "--id",
            "abcd",
        ])
        .assert()
        .success();
}
