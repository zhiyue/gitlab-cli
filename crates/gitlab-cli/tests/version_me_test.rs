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

#[tokio::test]
async fn unauthorized_error_includes_hint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/user"))
        .respond_with(ResponseTemplate::new(401).set_body_string("401 Unauthorized"))
        .mount(&server)
        .await;
    let host = server.uri();
    let out = Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-bad")
        .arg("me")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    let v: serde_json::Value = serde_json::from_str(&stderr).unwrap();
    assert!(
        v["error"]["hint"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("token"),
        "expected unauthorized hint mentioning 'token', got: {stderr}"
    );
}

#[tokio::test]
async fn not_found_error_includes_hint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/x"))
        .respond_with(
            ResponseTemplate::new(404).set_body_string("{\"message\":\"404 Project Not Found\"}"),
        )
        .mount(&server)
        .await;
    let host = server.uri();
    let out = Command::cargo_bin("gitlab")
        .unwrap()
        .env("GITLAB_HOST", &host)
        .env("GITLAB_TOKEN", "glpat-x")
        .args(["project", "get", "x"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(5));
    let v: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    let hint = v["error"]["hint"].as_str().unwrap_or("");
    assert!(!hint.is_empty(), "404 should produce a hint");
}
