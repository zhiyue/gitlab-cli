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
async fn label_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/labels"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":9,"name":"bug"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/labels/9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":9,"name":"bug"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/labels"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":10,"name":"feat"}))).mount(&server).await;
    Mock::given(method("PUT")).and(path("/api/v4/projects/1/labels/9"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":9,"name":"bug2"}))).mount(&server).await;
    Mock::given(method("DELETE")).and(path("/api/v4/projects/1/labels/9"))
        .respond_with(ResponseTemplate::new(204)).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/labels/9/subscribe"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":9}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/labels/9/unsubscribe"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":9}))).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["label","list","--project","1"]).assert().success();
    env_cmd(&base).args(["label","get","--project","1","--id","9"]).assert().success();
    env_cmd(&base).args(["label","create","--project","1","--name","feat","--color","#0F0"]).assert().success();
    env_cmd(&base).args(["label","update","--project","1","--id","9","--data",r#"{"name":"bug2"}"#]).assert().success();
    env_cmd(&base).args(["label","delete","--project","1","--id","9"]).assert().success();
    env_cmd(&base).args(["label","subscribe","--project","1","--id","9"]).assert().success();
    env_cmd(&base).args(["label","unsubscribe","--project","1","--id","9"]).assert().success();
}
