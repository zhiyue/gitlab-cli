use gitlab_cli::safety::{confirm_or_skip, dry_run_envelope, Intent};
use reqwest::Method;

#[test]
fn dry_run_serializes_intent() {
    let v = dry_run_envelope(&Intent {
        method: Method::POST,
        path: "projects/1/merge_requests/5/merge".into(),
        query: vec![],
        body: Some(serde_json::json!({"squash": true})),
    });
    assert_eq!(v["dry_run"], true);
    assert_eq!(v["method"], "POST");
    assert_eq!(v["path"], "projects/1/merge_requests/5/merge");
    assert_eq!(v["body"]["squash"], true);
}

#[test]
fn confirm_or_skip_returns_true_when_assume_yes() {
    assert!(confirm_or_skip(true, "delete").unwrap());
}
