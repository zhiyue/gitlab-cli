use gitlab_cli::context::{Context, CliInputs};
use gitlab_cli::globals::{GlobalArgs, OutputFormat};

fn defaults() -> GlobalArgs {
    GlobalArgs {
        host: None,
        token: None,
        output: OutputFormat::Json,
        limit: None,
        no_paginate: false,
        timeout: None,
        retries: None,
        no_retry: false,
        rps: None,
        dry_run: false,
        assume_yes: false,
        verbose: None,
        tls_skip_verify: false,
        config: None,
    }
}

#[test]
fn context_requires_token_somewhere() {
    let globals = defaults();
    let err = Context::build(CliInputs { globals, config_text: String::new() }).unwrap_err();
    assert!(err.to_string().contains("no host") || err.to_string().contains("no PAT"));
}

#[test]
fn context_builds_with_flag_inputs() {
    let mut g = defaults();
    g.host = Some("https://example.com".into());
    g.token = Some("glpat-z".into());
    let ctx = Context::build(CliInputs { globals: g, config_text: String::new() }).unwrap();
    assert_eq!(ctx.host, "https://example.com");
    assert_eq!(ctx.client.base_url().as_str(), "https://example.com/api/v4/");
}
