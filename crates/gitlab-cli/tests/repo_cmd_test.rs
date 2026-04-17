use assert_cmd::Command;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn repo_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/tree"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"path":"src"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/archive.tar.gz"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0x1f,0x8b]).insert_header("Content-Type", "application/gzip")).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/compare")).and(query_param("from", "a")).and(query_param("to", "b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"commits":[]}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/contributors"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"name":"alice"}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/repository/merge_base"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"deadbeef"}))).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["repo","tree","--project","1"]).assert().success();
    let out = env_cmd(&base).args(["repo","archive","--project","1","--format","tar.gz"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(&out.stdout[..2], &[0x1f,0x8b]);
    env_cmd(&base).args(["repo","compare","--project","1","--from","a","--to","b"]).assert().success();
    env_cmd(&base).args(["repo","contributors","--project","1"]).assert().success();
    env_cmd(&base).args(["repo","merge-base","--project","1","--ref","a","--ref","b"]).assert().success();
}
