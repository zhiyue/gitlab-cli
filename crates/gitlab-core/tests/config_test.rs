use gitlab_core::config::{Config, HostConfig};
use std::io::Write;

#[test]
fn parses_multi_host_toml() {
    let toml = r#"
default_host = "gitlab.example.com"

[host."gitlab.example.com"]
token = "glpat-AAA"
rps = 5
default_project = "g/p"

[host."gitlab.com"]
token = "glpat-BBB"
"#;
    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.default_host.as_deref(), Some("gitlab.example.com"));
    assert_eq!(cfg.host.len(), 2);
    let h = cfg.host.get("gitlab.example.com").unwrap();
    assert_eq!(h.token.as_deref(), Some("glpat-AAA"));
    assert_eq!(h.rps, Some(5));
    assert_eq!(h.default_project.as_deref(), Some("g/p"));
}

#[test]
fn load_missing_file_returns_empty_config() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("missing.toml");
    let cfg = Config::load_from(&path).unwrap();
    assert!(cfg.host.is_empty());
}

#[test]
fn load_reads_existing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "default_host = \"a\"").unwrap();
    writeln!(f, "[host.\"a\"]").unwrap();
    writeln!(f, "token = \"tok\"").unwrap();
    let cfg = Config::load_from(&path).unwrap();
    assert_eq!(cfg.default_host.as_deref(), Some("a"));
}

#[test]
fn host_for_resolves_to_specified_then_default() {
    let cfg = Config {
        default_host: Some("a".into()),
        host: std::collections::HashMap::from([
            (
                "a".into(),
                HostConfig {
                    token: Some("aaa".into()),
                    ..HostConfig::default()
                },
            ),
            (
                "b".into(),
                HostConfig {
                    token: Some("bbb".into()),
                    ..HostConfig::default()
                },
            ),
        ]),
    };
    assert_eq!(cfg.host_for(None).unwrap().token.as_deref(), Some("aaa"));
    assert_eq!(
        cfg.host_for(Some("b")).unwrap().token.as_deref(),
        Some("bbb")
    );
    assert!(cfg.host_for(Some("c")).is_none());
}
