use anyhow::{anyhow, Result};
use clap::Args;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::output::emit_object;

const MANIFEST_DATA: &str = include_str!("../../manifest_data.toml");

#[derive(Args, Debug)]
pub struct ManifestArgs {
    /// Top-level command name to drill into (e.g., 'mr', 'file').
    pub command: Option<String>,
    /// Verb under the command (e.g., 'changes', 'raw').
    pub verb: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ManifestData {
    instance_target: String,
    agent_hints: Vec<String>,
    #[serde(rename = "command")]
    commands: Vec<CommandEntry>,
    #[serde(rename = "verb", default)]
    verbs: Vec<VerbEntry>,
    #[serde(rename = "quirk", default)]
    quirks: Vec<QuirkEntry>,
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
struct CommandEntry {
    name: String,
    purpose: String,
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
struct VerbEntry {
    command: String,
    verb: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    example: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    quirk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    returns: Option<String>,
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
struct QuirkEntry {
    area: String,
    issue: String,
    workaround: String,
}

fn load_data() -> Result<ManifestData> {
    toml::from_str(MANIFEST_DATA).map_err(|e| anyhow!("manifest data parse: {e}"))
}

fn exit_codes() -> Value {
    json!({
        "0": "success",
        "1": "unknown error",
        "2": "invalid_args (clap parse failure)",
        "3": "unauthorized (HTTP 401)",
        "4": "forbidden (HTTP 403)",
        "5": "not_found (HTTP 404)",
        "6": "conflict (HTTP 409 / 422 / bad_request)",
        "7": "rate_limited (HTTP 429 after retries)",
        "8": "server_error (HTTP 5xx after retries)",
        "9": "network or timeout",
        "10": "dry_run completed (no HTTP request issued)"
    })
}

pub fn run(args: ManifestArgs) -> Result<()> {
    let data = load_data()?;
    match (args.command, args.verb) {
        (None, _) => {
            let v = json!({
                "version": env!("CARGO_PKG_VERSION"),
                "instance": data.instance_target,
                "exit_codes": exit_codes(),
                "commands": data.commands,
                "agent_hints": data.agent_hints,
                "global_quirks_count": data.quirks.len(),
                "next": "Run 'gitlab manifest <command>' for verbs, examples, and quirks."
            });
            emit_object(&v)?;
        }
        (Some(cmd), None) => {
            let entry = data.commands.iter().find(|c| c.name == cmd)
                .ok_or_else(|| anyhow!("unknown command: {cmd}. Run 'gitlab manifest' for the list."))?;
            let verbs: Vec<&VerbEntry> = data.verbs.iter().filter(|v| v.command == cmd).collect();
            let quirks: Vec<&QuirkEntry> = data.quirks.iter()
                .filter(|q| q.area.starts_with(&cmd) || q.area.contains(&format!(" {cmd}")) || q.area.contains(&format!("{cmd} ")))
                .collect();
            let v = json!({
                "name": entry.name,
                "purpose": entry.purpose,
                "verbs": verbs,
                "quirks": quirks,
                "next": format!("Run 'gitlab manifest {cmd} <verb>' for a single verb, or 'gitlab {cmd} --help' for clap-rendered args."),
            });
            emit_object(&v)?;
        }
        (Some(cmd), Some(verb)) => {
            let entry = data.verbs.iter().find(|v| v.command == cmd && v.verb == verb)
                .cloned();
            let v = match entry {
                Some(e) => json!({
                    "command": cmd,
                    "verb": verb,
                    "example": e.example,
                    "returns": e.returns,
                    "quirk": e.quirk,
                    "next": format!("For full args, run: gitlab {cmd} {verb} --help"),
                }),
                None => {
                    // Verb might exist in clap but have no enrichment data — return a minimal entry pointing to --help.
                    if data.commands.iter().any(|c| c.name == cmd) {
                        json!({
                            "command": cmd,
                            "verb": verb,
                            "note": "No enrichment data; verb may exist — check 'gitlab <cmd> <verb> --help' for clap-rendered args.",
                        })
                    } else {
                        return Err(anyhow!("unknown command: {cmd}"));
                    }
                }
            };
            emit_object(&v)?;
        }
    }
    Ok(())
}
