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
async fn branch_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/repository/branches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"name":"main"}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/repository/branches/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"name":"main"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/repository/branches"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"name":"topic"})))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/v4/projects/1/repository/branches/topic"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/protected_branches"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"name":"main"})))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/v4/projects/1/protected_branches/main"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let base = server.uri();
    env_cmd(&base)
        .args(["branch", "list", "--project", "1"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["branch", "get", "--project", "1", "--name", "main"])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "branch",
            "create",
            "--project",
            "1",
            "--name",
            "topic",
            "--ref",
            "main",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args(["branch", "delete", "--project", "1", "--name", "topic"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["branch", "protect", "--project", "1", "--name", "main"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["branch", "unprotect", "--project", "1", "--name", "main"])
        .assert()
        .success();
}
