#[allow(unused_imports)]
use gitlab_core::retry::{RetryPlan, RetryPolicy};
use std::time::Duration;

#[test]
#[allow(clippy::uninlined_format_args, clippy::cast_possible_truncation)]
fn default_policy_returns_4_network_backoffs() {
    let p = RetryPolicy::default();
    let plan = p.plan_for_network();
    assert_eq!(plan.attempts.len(), 4);
    let expected = [500, 1000, 2000, 4000];
    for (got, want) in plan.attempts.iter().zip(expected.iter()) {
        let tol = *want * 20 / 100;
        assert!(
            got.as_millis() as u64 >= want - tol && got.as_millis() as u64 <= want + tol,
            "backoff {:?} not within 20% of {}ms",
            got,
            want
        );
    }
}

#[test]
fn retry_after_header_beats_backoff() {
    let p = RetryPolicy::default();
    let next = p.next_delay_for_429(Some("7"), 0);
    assert_eq!(next, Some(Duration::from_secs(7)));
}

#[test]
fn no_retries_when_disabled() {
    let p = RetryPolicy {
        max_attempts: 0,
        ..RetryPolicy::default()
    };
    let plan = p.plan_for_network();
    assert!(plan.attempts.is_empty());
}

#[test]
fn retry_after_invalid_falls_back_to_backoff() {
    let p = RetryPolicy::default();
    let next = p.next_delay_for_429(Some("invalid"), 0);
    assert!(next.is_some());
    assert!(next.unwrap() >= Duration::from_millis(400));
}
