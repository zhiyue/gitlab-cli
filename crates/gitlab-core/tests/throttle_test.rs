use gitlab_core::throttle::Throttle;
use std::time::Instant;

#[tokio::test]
async fn disabled_throttle_never_sleeps() {
    let t = Throttle::disabled();
    let start = Instant::now();
    for _ in 0..20 {
        t.acquire().await;
    }
    assert!(start.elapsed().as_millis() < 50);
}

#[tokio::test]
async fn enabled_throttle_spaces_requests() {
    let t = Throttle::per_second(5);
    let start = Instant::now();
    for _ in 0..10 {
        t.acquire().await;
    }
    let elapsed = start.elapsed().as_millis();
    assert!(elapsed >= 800, "10 reqs @ 5 rps should take ≥ 0.8s, got {elapsed}ms");
}
