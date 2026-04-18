use gitlab_core::auth::MaskedToken;
use gitlab_core::client::{Client, ClientOptions};
use gitlab_core::request::RequestSpec;
use gitlab_core::retry::RetryPolicy;
use reqwest::Method;

#[test]
fn masked_token_hides_middle() {
    assert_eq!(MaskedToken("glpat-ABCDEFGHIJKL").to_string(), "glpa****IJKL");
    assert_eq!(MaskedToken("xx").to_string(), "****");
}

#[tokio::test]
async fn error_never_contains_literal_token() {
    let client = Client::new(ClientOptions {
        host: "http://127.0.0.1:1".into(),
        token: "glpat-SHOULDNEVERAPPEAR".into(),
        retry: RetryPolicy { max_attempts: 0, max_attempts_429: 0, ..RetryPolicy::default() },
        ..ClientOptions::default()
    })
    .unwrap();
    let err = client
        .send_json::<serde_json::Value>(RequestSpec::new(Method::GET, "version"))
        .await
        .unwrap_err();
    let msg = err.to_string();
    let dbg = format!("{err:?}");
    let payload = serde_json::to_string(&err.to_payload()).unwrap();
    assert!(!msg.contains("glpat-SHOULDNEVERAPPEAR"), "display leaked token: {msg}");
    assert!(!dbg.contains("glpat-SHOULDNEVERAPPEAR"), "debug leaked token: {dbg}");
    assert!(!payload.contains("glpat-SHOULDNEVERAPPEAR"), "payload leaked token: {payload}");
}
