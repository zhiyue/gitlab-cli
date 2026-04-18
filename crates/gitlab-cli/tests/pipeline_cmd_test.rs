use assert_cmd::Command;
use predicates::str::contains;
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
async fn pipeline_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/pipelines"))
        .and(query_param("status", "running"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!([{"id":10,"status":"running"}])),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/pipelines/10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":10})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/pipeline"))
        .and(query_param("ref", "main"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":11})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/pipelines/10/retry"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":10,"status":"pending"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/pipelines/10/cancel"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"id":10,"status":"canceled"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/v4/projects/1/pipelines/10"))
        .respond_with(ResponseTemplate::new(204).set_body_string(""))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/pipelines/10/variables"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"key":"K","value":"V"}])))
        .mount(&server)
        .await;

    let base = server.uri();
    env_cmd(&base)
        .args(["pipeline", "list", "--project", "1", "--status", "running"])
        .assert()
        .success()
        .stdout(contains("running"));
    env_cmd(&base)
        .args(["pipeline", "get", "--project", "1", "--id", "10"])
        .assert()
        .success()
        .stdout(contains("\"id\":10"));
    env_cmd(&base)
        .args(["pipeline", "create", "--project", "1", "--ref", "main"])
        .assert()
        .success()
        .stdout(contains("\"id\":11"));
    env_cmd(&base)
        .args(["pipeline", "retry", "--project", "1", "--id", "10"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["pipeline", "cancel", "--project", "1", "--id", "10"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["pipeline", "delete", "--project", "1", "--id", "10"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["pipeline", "variables", "--project", "1", "--id", "10"])
        .assert()
        .success()
        .stdout(contains("\"key\":\"K\""));
}
