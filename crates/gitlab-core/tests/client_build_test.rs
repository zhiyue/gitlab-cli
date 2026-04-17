use gitlab_core::client::{Client, ClientOptions};

#[test]
fn client_builds_with_defaults() {
    let c = Client::new(ClientOptions {
        host: "https://gitlab.example.com".into(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    assert_eq!(c.base_url().as_str(), "https://gitlab.example.com/api/v4/");
}

#[test]
fn host_without_scheme_is_rejected() {
    let err = Client::new(ClientOptions {
        host: "gitlab.example.com".into(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap_err();
    assert_eq!(err.code(), gitlab_core::error::ErrorCode::InvalidArgs);
}

#[test]
fn host_keeps_trailing_slash_consistent() {
    let c = Client::new(ClientOptions {
        host: "https://gitlab.example.com/".into(),
        token: "glpat-x".into(),
        ..ClientOptions::default()
    })
    .unwrap();
    assert_eq!(c.base_url().as_str(), "https://gitlab.example.com/api/v4/");
}
