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
async fn group_list() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/groups"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!([{"id":1,"full_path":"atoms"}])),
        )
        .mount(&server)
        .await;
    env_cmd(&server.uri())
        .args(["group", "list"])
        .assert()
        .success()
        .stdout(contains("atoms"));
}

#[tokio::test]
async fn group_get_members_projects_subgroups() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/groups/atoms"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":1,"full_path":"atoms"})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/groups/atoms/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":42}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/groups/atoms/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":7}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/groups/atoms/subgroups"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":13}])))
        .mount(&server)
        .await;
    env_cmd(&server.uri())
        .args(["group", "get", "atoms"])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["group", "members", "atoms"])
        .assert()
        .success()
        .stdout(contains("42"));
    env_cmd(&server.uri())
        .args(["group", "projects", "atoms"])
        .assert()
        .success()
        .stdout(contains("7"));
    env_cmd(&server.uri())
        .args(["group", "subgroups", "atoms"])
        .assert()
        .success()
        .stdout(contains("13"));
}

#[tokio::test]
async fn group_create_update_delete() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v4/groups"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(json!({"id":5,"name":"n","path":"p"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/groups/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":5,"name":"nn"})))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/v4/groups/5"))
        .respond_with(ResponseTemplate::new(202).set_body_string(""))
        .mount(&server)
        .await;
    env_cmd(&server.uri())
        .args(["group", "create", "--name", "n", "--path", "p"])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["group", "update", "5", "--data", r#"{"name":"nn"}"#])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["group", "delete", "5"])
        .assert()
        .success();
}
