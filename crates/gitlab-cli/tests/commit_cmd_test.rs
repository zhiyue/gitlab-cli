use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn commit_all_verbs() {
    let server = MockServer::start().await;
    let base = server.uri();
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":"abc"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits/abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"abc"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/repository/commits"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"new"}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits/abc/diff"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits/abc/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits/abc/statuses"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([]))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/repository/commits/abc/cherry_pick"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"cp"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/repository/commits/abc/revert"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":"rv"}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/commits/abc/refs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([]))).mount(&server).await;

    env_cmd(&base).args(["commit","list","--project","1"]).assert().success();
    env_cmd(&base).args(["commit","get","--project","1","--sha","abc"]).assert().success();
    env_cmd(&base).args(["commit","create","--project","1","--data",r#"{"branch":"main","commit_message":"c","actions":[]}"#]).assert().success();
    env_cmd(&base).args(["commit","diff","--project","1","--sha","abc"]).assert().success();
    env_cmd(&base).args(["commit","comments","--project","1","--sha","abc"]).assert().success();
    env_cmd(&base).args(["commit","statuses","--project","1","--sha","abc"]).assert().success();
    env_cmd(&base).args(["commit","cherry-pick","--project","1","--sha","abc","--branch","hotfix"]).assert().success();
    env_cmd(&base).args(["commit","revert","--project","1","--sha","abc","--branch","main"]).assert().success();
    env_cmd(&base).args(["commit","refs","--project","1","--sha","abc"]).assert().success();
}
