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
async fn search_three_scopes() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/search")).and(query_param("scope", "issues")).and(query_param("search", "bug"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":1,"title":"bug A"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/groups/atoms/search")).and(query_param("scope", "commits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":"abc"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/search")).and(query_param("scope", "blobs")).and(query_param("search", "fn"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"filename":"main.rs"}]))).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["search","--scope","issues","--query","bug"]).assert().success().stdout(contains("bug A"));
    env_cmd(&base).args(["search","--scope","commits","--query","x","--group","atoms"]).assert().success().stdout(contains("abc"));
    env_cmd(&base).args(["search","--scope","blobs","--query","fn","--project","1"]).assert().success().stdout(contains("main.rs"));
}
