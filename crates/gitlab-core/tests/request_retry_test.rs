use gitlab_core::client::{Client, ClientOptions};
use gitlab_core::request::RequestSpec;
use reqwest::Method;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn retries_on_500_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"version":"14.0.5-ee"})),
        )
        .mount(&server)
        .await;
    let client = Client::new(ClientOptions {
        host: server.uri(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    let body: serde_json::Value = client
        .send_json(RequestSpec::new(Method::GET, "version"))
        .await
        .unwrap();
    assert_eq!(body["version"], "14.0.5-ee");
}

#[tokio::test]
async fn non_retryable_4xx_fails_immediately() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/nope"))
        .respond_with(ResponseTemplate::new(404).set_body_string("404 Not Found"))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::new(ClientOptions {
        host: server.uri(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    let err = client
        .send_json::<serde_json::Value>(RequestSpec::new(Method::GET, "nope"))
        .await
        .unwrap_err();
    assert_eq!(err.code(), gitlab_core::error::ErrorCode::NotFound);
}

#[tokio::test]
async fn honors_retry_after_on_429() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"version":"14.0.5-ee"})),
        )
        .mount(&server)
        .await;
    let client = Client::new(ClientOptions {
        host: server.uri(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    let start = std::time::Instant::now();
    let _: serde_json::Value = client
        .send_json(RequestSpec::new(Method::GET, "version"))
        .await
        .unwrap();
    assert!(start.elapsed().as_millis() >= 900);
}
