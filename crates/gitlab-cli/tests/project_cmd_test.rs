use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn project_list_auto_paginates() {
    let server = MockServer::start().await;
    let base = server.uri();
    Mock::given(method("GET")).and(path("/api/v4/projects")).and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!([{"id":1}, {"id":2}]))
            .insert_header(
                "Link",
                format!("<{base}/api/v4/projects?page=2&per_page=100>; rel=\"next\"")
            ))
        .mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects")).and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":3}])))
        .mount(&server).await;

    env_cmd(&base).args(["project", "list"])
        .assert().success()
        .stdout(contains("\"id\":1")).stdout(contains("\"id\":3"));
}

#[tokio::test]
async fn project_get_by_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/group%2Fproj"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":9, "path_with_namespace":"group/proj"})))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["project", "get", "group/proj"])
        .assert().success().stdout(contains("group/proj"));
}

#[tokio::test]
async fn project_create() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/api/v4/projects"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":77,"name":"new"})))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["project", "create", "--name", "new"])
        .assert().success().stdout(contains("77"));
}

#[tokio::test]
async fn project_delete() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/5"))
        .respond_with(ResponseTemplate::new(202).set_body_string(""))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["project", "delete", "5"])
        .assert().success();
}

#[tokio::test]
async fn project_archive_and_unarchive() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/api/v4/projects/5/archive"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":5,"archived":true})))
        .mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/5/unarchive"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":5,"archived":false})))
        .mount(&server).await;
    env_cmd(&server.uri()).args(["project", "archive", "5"]).assert().success().stdout(contains("archived\":true"));
    env_cmd(&server.uri()).args(["project", "unarchive", "5"]).assert().success().stdout(contains("archived\":false"));
}
