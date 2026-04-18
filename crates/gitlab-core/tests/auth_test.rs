use gitlab_core::auth::{resolve_auth, AuthInputs};
use gitlab_core::config::{Config, HostConfig};

#[allow(clippy::field_reassign_with_default)]
fn cfg(host: &str, token: &str) -> Config {
    let mut c = Config::default();
    c.default_host = Some(host.into());
    c.host.insert(
        host.into(),
        HostConfig {
            token: Some(token.into()),
            ..HostConfig::default()
        },
    );
    c
}

#[test]
fn cli_flag_beats_env_and_config() {
    let resolved = resolve_auth(
        AuthInputs {
            flag_token: Some("flag-tok".into()),
            flag_host: None,
            env_token: Some("env-tok".into()),
            env_host: None,
        },
        &cfg("h", "cfg-tok"),
    )
    .unwrap();
    assert_eq!(resolved.token, "flag-tok");
    assert_eq!(resolved.host, "h");
}

#[test]
fn env_beats_config_when_no_flag() {
    let resolved = resolve_auth(
        AuthInputs {
            flag_token: None,
            flag_host: None,
            env_token: Some("env-tok".into()),
            env_host: None,
        },
        &cfg("h", "cfg-tok"),
    )
    .unwrap();
    assert_eq!(resolved.token, "env-tok");
}

#[test]
fn config_is_last_fallback() {
    let resolved = resolve_auth(AuthInputs::default(), &cfg("h", "cfg-tok")).unwrap();
    assert_eq!(resolved.token, "cfg-tok");
}

#[test]
fn missing_token_yields_unauthorized() {
    let err = resolve_auth(AuthInputs::default(), &Config::default()).unwrap_err();
    assert_eq!(err.code(), gitlab_core::error::ErrorCode::Unauthorized);
}

#[test]
fn host_precedence_flag_env_config() {
    let c = cfg("cfg-host", "tok");
    assert_eq!(
        resolve_auth(
            AuthInputs {
                flag_host: Some("flag-host".into()),
                ..AuthInputs::default()
            },
            &c
        )
        .unwrap()
        .host,
        "flag-host"
    );
    assert_eq!(
        resolve_auth(
            AuthInputs {
                env_host: Some("env-host".into()),
                ..AuthInputs::default()
            },
            &c
        )
        .unwrap()
        .host,
        "env-host"
    );
    assert_eq!(
        resolve_auth(AuthInputs::default(), &c).unwrap().host,
        "cfg-host"
    );
}
