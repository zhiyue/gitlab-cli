use assert_cmd::Command;
use predicates::str::contains;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn version_prints_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"version":"14.0.5-ee","revision":"abc"})),
        )
        .mount(&server)
        .await;
    let host = server.uri();
    let assert = Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .arg("version")
        .assert()
        .success();
    assert.stdout(contains("14.0.5-ee"));
}

#[tokio::test]
async fn me_prints_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/user"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"id":1,"username":"alice"})),
        )
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .arg("me")
        .assert()
        .success()
        .stdout(contains("alice"));
}

#[tokio::test]
async fn unauthorized_exits_with_code_3() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/user"))
        .respond_with(ResponseTemplate::new(401).set_body_string("401 Unauthorized"))
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .arg("me")
        .assert()
        .code(3)
        .stderr(contains("unauthorized"));
}

#[tokio::test]
async fn server_error_exits_with_code_8_after_retries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let host = server.uri();
    Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .arg("--retries")
        .arg("1")
        .arg("version")
        .assert()
        .code(8);
}
