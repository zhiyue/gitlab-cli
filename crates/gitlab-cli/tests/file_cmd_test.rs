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
#[allow(clippy::too_many_lines)]
async fn file_all_verbs() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/repository/files/src%2Ffoo.rs"))
        .and(query_param("ref", "main"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"file_path":"src/foo.rs","content":"base64"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/1/repository/files/src%2Ffoo.rs/raw"))
        .and(query_param("ref", "main"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("fn main(){}\n")
                .insert_header("Content-Type", "text/plain"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/api/v4/projects/1/repository/files/src%2Ffoo.rs/blame",
        ))
        .and(query_param("ref", "main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v4/projects/1/repository/files/new.txt"))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"file_path":"new.txt"})))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/v4/projects/1/repository/files/new.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"file_path":"new.txt"})))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/api/v4/projects/1/repository/files/new.txt"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let base = server.uri();
    env_cmd(&base)
        .args([
            "file",
            "get",
            "--project",
            "1",
            "--path",
            "src/foo.rs",
            "--ref",
            "main",
        ])
        .assert()
        .success()
        .stdout(contains("file_path"));
    let out = env_cmd(&base)
        .args([
            "file",
            "raw",
            "--project",
            "1",
            "--path",
            "src/foo.rs",
            "--ref",
            "main",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("fn main"));
    env_cmd(&base)
        .args([
            "file",
            "blame",
            "--project",
            "1",
            "--path",
            "src/foo.rs",
            "--ref",
            "main",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "file",
            "create",
            "--project",
            "1",
            "--path",
            "new.txt",
            "--branch",
            "main",
            "--content",
            "hi",
            "--message",
            "c",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "file",
            "update",
            "--project",
            "1",
            "--path",
            "new.txt",
            "--branch",
            "main",
            "--content",
            "hi2",
            "--message",
            "u",
        ])
        .assert()
        .success();
    env_cmd(&base)
        .args([
            "file",
            "delete",
            "--project",
            "1",
            "--path",
            "new.txt",
            "--branch",
            "main",
            "--message",
            "d",
        ])
        .assert()
        .success();
}
