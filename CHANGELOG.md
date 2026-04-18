# Changelog

All notable changes to `gitlab-cli` documented here. Format follows [Keep a Changelog](https://keepachangelog.com).

## [0.2.1] — 2026-04-18

### Fixed

- `Cargo.toml` workspace version was still `0.1.0`, so `gitlab --version` printed `0.1.0` even on the v0.2.0 release tag. Bumped to track release tags going forward.
- `scripts/update-homebrew-formula.sh` constructed the wrong URL (`<stem>.tar.gz.sha256` instead of `<stem>.sha256`), causing the `bump-homebrew` job to 404 on every fetch. Fixed.
- `deny.toml` rejected `Unicode-3.0` (icu_* via url→idna) and `CDLA-Permissive-2.0` (webpki-roots). Both added to allow list.
- `release.yml` `workflow_dispatch` `tag` input now properly prefixed with `refs/tags/` for `taiki-e/*-action` compatibility.

### Removed

- `x86_64-apple-darwin` (Intel Mac) build dropped from release matrix — `macos-13` runner is being deprecated by GitHub Actions. Intel Mac users: use the curl|sh installer or `cargo install --git ...`.

## [0.2.0] — 2026-04-18

### Added

- `gitlab manifest` — lazy 3-tier self-describing schema for agents (~3 KB index, drill-down per command/verb).
- `gitlab from-url <url>` — parse any GitLab web URL (MR, issue, commit, blob, pipeline, tree, tag, job, project) into structured JSON with a `suggested` follow-up command.
- Conditional `error.hint` field on stderr error JSON — actionable fix suggestions for the 10 most common error patterns (unauthorized, forbidden, not_found, conflict, rate_limited, server_error, bad_request).
- `assume_yes = true` per-host config option (`~/.config/gitlab-cli/config.toml [host."…"]`) — equivalent to setting `GITLAB_ASSUME_YES=1` env var.
- `README.md` "Known GitLab 14.0.5 API quirks" section documenting `mr commits.parent_ids = []`, missing `mr diffs` / `raw_diffs` endpoints, etc.

### Changed

- Config file path moved from `~/Library/Application Support/gitlab-cli/` to `~/.config/gitlab-cli/` on macOS to match XDG (Linux already used XDG). Manual migration: `mkdir -p ~/.config/gitlab-cli && mv ~/Library/Application\ Support/gitlab-cli/config.toml ~/.config/gitlab-cli/`.
- Build script (`build.rs`) now also watches the resolved branch ref file (e.g., `.git/refs/heads/main`) so `--version` git sha refreshes on new commits, not only on branch switch.

### Removed

- `mr diffs` subcommand — the underlying `GET /merge_requests/:iid/diffs` endpoint was introduced in GitLab 15.7 and returns 404 on 14.0.5. Use `mr changes` for the same data.
- `directories` crate dependency (replaced by hand-rolled XDG resolution).

### Fixed

- `client::send_raw` retry loop: `attempt_idx_net` was a constant, causing infinite retries on 5xx/timeout/connect errors. Now properly increments per attempt and terminates after `RetryPolicy::max_attempts` tries.

## [0.1.0] — 2026-04-17

Initial release.

- 17 top-level commands wrapping GitLab 14.0.5 REST API: `project`, `group`, `mr`, `issue`, `pipeline`, `job`, `commit`, `branch`, `tag`, `file`, `repo`, `user`, `label`, `note`, `discussion`, `search`, plus utility (`version`, `me`, `config`) and the `api` raw escape hatch.
- JSON-first stdout (single object or array), structured stderr error JSON, bounded exit-code vocabulary (0/2/3/4/5/6/7/8/9/10).
- Auto-pagination across `Link` headers; `--limit N`, `--no-paginate`, `--output ndjson` controls.
- Retry policy (4 backoffs for 5xx/network, 5 for 429 honoring `Retry-After`) + client-side rate limiter (`--rps`).
- PAT auth via `--token` / `GITLAB_TOKEN` / `~/.config/gitlab-cli/config.toml`.
- Single static binary on macOS arm64+x86_64 and Linux musl arm64+x86_64.
- 81 unit + integration tests against wiremock; opt-in L3 smoke tests against real instance.
