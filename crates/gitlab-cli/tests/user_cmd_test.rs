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
async fn user_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/users"))
        .and(query_param("search", "alice"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!([{"id":1,"username":"alice"}])),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/users/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":1,"username":"alice"})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":1,"username":"alice"})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/users/1/keys"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":9,"title":"laptop"}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/users/1/emails"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{"id":1,"email":"a@x"}])))
        .mount(&server)
        .await;

    let base = server.uri();
    env_cmd(&base)
        .args(["user", "list", "--search", "alice"])
        .assert()
        .success()
        .stdout(contains("alice"));
    env_cmd(&base)
        .args(["user", "get", "--id", "1"])
        .assert()
        .success();
    env_cmd(&base)
        .args(["user", "me"])
        .assert()
        .success()
        .stdout(contains("alice"));
    env_cmd(&base)
        .args(["user", "keys", "--id", "1"])
        .assert()
        .success()
        .stdout(contains("laptop"));
    env_cmd(&base)
        .args(["user", "emails", "--id", "1"])
        .assert()
        .success()
        .stdout(contains("a@x"));
}
