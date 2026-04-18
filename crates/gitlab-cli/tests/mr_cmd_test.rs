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
async fn mr_list_by_group_and_project() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/groups/atoms/merge_requests"))
        .and(query_param("state", "opened"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"iid":1}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/merge_requests"))
        .and(query_param("state", "opened"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"iid":2}])))
        .mount(&server)
        .await;
    env_cmd(&server.uri())
        .args(["mr", "list", "--group", "atoms", "--state", "opened"])
        .assert()
        .success()
        .stdout(contains("1"));
    env_cmd(&server.uri())
        .args(["mr", "list", "--project", "1", "--state", "opened"])
        .assert()
        .success()
        .stdout(contains("2"));
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn mr_crud_and_actions() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/merge_requests/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"iid":5,"state":"opened"})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/merge_requests"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"iid":9})))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/projects/1/merge_requests/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"iid":5,"title":"t"})))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/projects/1/merge_requests/5/merge"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"iid":5,"state":"merged"})))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/projects/1/merge_requests/5/rebase"))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({"rebase_in_progress":true})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/merge_requests/5/approve"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id":5,"approved_by":[]})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/merge_requests/5/unapprove"))
        .respond_with(ResponseTemplate::new(201).set_body_string(""))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/merge_requests/5/changes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"changes":[]})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/merge_requests/5/diffs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/merge_requests/5/commits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":"abc"}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/merge_requests/5/pipelines"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":100}])))
        .mount(&server)
        .await;

    env_cmd(&server.uri())
        .args(["mr", "get", "--project", "1", "--mr", "5"])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args([
            "mr",
            "create",
            "--project",
            "1",
            "--source",
            "topic",
            "--target",
            "main",
            "--title",
            "T",
        ])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args([
            "mr",
            "update",
            "--project",
            "1",
            "--mr",
            "5",
            "--data",
            r#"{"title":"t"}"#,
        ])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["mr", "merge", "--project", "1", "--mr", "5"])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["mr", "rebase", "--project", "1", "--mr", "5"])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["mr", "approve", "--project", "1", "--mr", "5"])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["mr", "unapprove", "--project", "1", "--mr", "5"])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["mr", "changes", "--project", "1", "--mr", "5"])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["mr", "diffs", "--project", "1", "--mr", "5"])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["mr", "commits", "--project", "1", "--mr", "5"])
        .assert()
        .success();
    env_cmd(&server.uri())
        .args(["mr", "pipelines", "--project", "1", "--mr", "5"])
        .assert()
        .success();
}

#[tokio::test]
async fn mr_close_and_reopen() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/projects/1/merge_requests/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"iid":5,"state":"closed"})))
        .mount(&server)
        .await;
    env_cmd(&server.uri())
        .args(["mr", "close", "--project", "1", "--mr", "5"])
        .assert()
        .success()
        .stdout(contains("closed"));
    env_cmd(&server.uri())
        .args(["mr", "reopen", "--project", "1", "--mr", "5"])
        .assert()
        .success();
}
