use gitlab_core::error::{ErrorCode, GitlabError};

#[test]
fn http_404_classifies_as_not_found() {
    let err = GitlabError::from_status(404, "404 Project Not Found".into(), None);
    assert_eq!(err.code(), ErrorCode::NotFound);
    assert_eq!(err.exit_code(), 5);
    assert!(!err.retryable());
}

#[test]
fn http_429_classifies_as_rate_limited_retryable() {
    let err = GitlabError::from_status(429, "Too Many Requests".into(), None);
    assert_eq!(err.code(), ErrorCode::RateLimited);
    assert_eq!(err.exit_code(), 7);
    assert!(err.retryable());
}

#[test]
fn http_500_classifies_as_server_retryable() {
    let err = GitlabError::from_status(500, "oops".into(), None);
    assert_eq!(err.code(), ErrorCode::ServerError);
    assert!(err.retryable());
}

#[test]
fn network_error_is_retryable() {
    let err = GitlabError::network("connection reset".into());
    assert_eq!(err.code(), ErrorCode::Network);
    assert_eq!(err.exit_code(), 9);
    assert!(err.retryable());
}
