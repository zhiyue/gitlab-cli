#[allow(unused_imports)]
use gitlab_cli_test_support::*;
// We use a small inline support module below.

#[test]
#[allow(clippy::assertions_on_constants)]
fn placeholder_until_command_exists() {
    assert!(true);
}

#[test]
fn tracing_filter_parses_levels() {
    let f = gitlab_cli::tracing_setup::filter_for(Some("debug"));
    assert_eq!(format!("{f}"), "debug");
    let f = gitlab_cli::tracing_setup::filter_for(Some("1"));
    assert_eq!(format!("{f}"), "info");
    let f = gitlab_cli::tracing_setup::filter_for(None);
    assert_eq!(format!("{f}"), "warn");
}

mod gitlab_cli_test_support {}
