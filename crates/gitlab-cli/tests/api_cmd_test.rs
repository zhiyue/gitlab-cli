use assert_cmd::Command;
use predicates::str::contains;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn api_get_prints_json_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/pipeline_schedules"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([{"id":42}])))
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .args(["api", "GET", "/projects/1/pipeline_schedules"])
        .assert()
        .success()
        .stdout(contains("42"));
}

#[tokio::test]
async fn api_post_sends_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/labels"))
        .and(body_json(&serde_json::json!({"name":"bug","color":"#FF0000"})))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(serde_json::json!({"id":9,"name":"bug"})),
        )
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .env("GITLAB_ASSUME_YES", "1")
        .args([
            "api",
            "POST",
            "/projects/1/labels",
            "--data",
            "{\"name\":\"bug\",\"color\":\"#FF0000\"}",
        ])
        .assert()
        .success()
        .stdout(contains("bug"));
}

#[tokio::test]
async fn api_respects_dry_run() {
    let host = "http://127.0.0.1:1"; // unroutable; ensures no network
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", host)
        .env("GITLAB_TOKEN", "glpat-x")
        .args([
            "--dry-run",
            "api",
            "DELETE",
            "/projects/1/issues/5",
        ])
        .assert()
        .code(10)
        .stdout(contains("dry_run"))
        .stdout(contains("DELETE"));
}

#[tokio::test]
async fn api_query_flags_pass_through() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/issues"))
        .and(query_param("state", "opened"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .args(["api", "GET", "/issues", "--query", "state=opened"])
        .assert()
        .success();
}
