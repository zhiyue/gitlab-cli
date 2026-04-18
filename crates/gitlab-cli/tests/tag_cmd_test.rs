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
async fn tag_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/repository/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"name":"v1"}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/repository/tags/v1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name":"v1"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/repository/tags"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"name":"v2"})))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/v4/projects/1/repository/tags/v1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/protected_tags"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"name":"v1"})))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/v4/projects/1/protected_tags/v1"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let base = server.uri();
    env_cmd(&base)
        .args(["tag", "list", "--project", "1"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["tag", "get", "--project", "1", "--name", "v1"])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "tag",
            "create",
            "--project",
            "1",
            "--name",
            "v2",
            "--ref",
            "main",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args(["tag", "delete", "--project", "1", "--name", "v1"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["tag", "protect", "--project", "1", "--name", "v1"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["tag", "unprotect", "--project", "1", "--name", "v1"])
        .assert()
        .success();
}
