use assert_cmd::Command;
use predicates::str::contains;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn env_cmd(host: &str) -> Command {
    let mut c = Command::cargo_bin("gitlab").unwrap();
    c.env("GITLAB_HOST", host).env("GITLAB_TOKEN", "glpat-x").env("GITLAB_ASSUME_YES", "1");
    c
}

#[tokio::test]
async fn job_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":77}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/pipelines/5/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":78}]))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/jobs/77"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":77,"status":"success"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/jobs/77/play"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":77,"status":"pending"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/jobs/77/retry"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":78}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/jobs/77/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":77,"status":"canceled"}))).mount(&server).await;
    Mock::given(method("POST")).and(path("/api/v4/projects/1/jobs/77/erase"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":77}))).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/jobs/77/trace"))
        .respond_with(ResponseTemplate::new(200).set_body_string("step1\nstep2\n").insert_header("Content-Type", "text/plain")).mount(&server).await;
    Mock::given(method("GET")).and(path("/api/v4/projects/1/jobs/77/artifacts"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0x50,0x4B,0x03,0x04]).insert_header("Content-Type", "application/zip")).mount(&server).await;

    let base = server.uri();
    env_cmd(&base).args(["job","list","--project","1"]).assert().success();
    env_cmd(&base).args(["job","list","--project","1","--pipeline","5"]).assert().success().stdout(contains("78"));
    env_cmd(&base).args(["job","get","--project","1","--id","77"]).assert().success();
    env_cmd(&base).args(["job","play","--project","1","--id","77"]).assert().success();
    env_cmd(&base).args(["job","retry","--project","1","--id","77"]).assert().success();
    env_cmd(&base).args(["job","cancel","--project","1","--id","77"]).assert().success();
    env_cmd(&base).args(["job","erase","--project","1","--id","77"]).assert().success();
    let out = env_cmd(&base).args(["job","trace","--project","1","--id","77"]).output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("step1"));
    let out = env_cmd(&base).args(["job","artifacts","--project","1","--id","77"]).output().unwrap();
    assert!(out.status.success());
    assert_eq!(&out.stdout[..4], &[0x50,0x4B,0x03,0x04]);
}
