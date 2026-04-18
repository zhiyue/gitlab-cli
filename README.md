# gitlab-cli

An agent-first Rust CLI for **GitLab EE 14.0.5** (not newer, not older).
Consumed by autonomous agents via `bash -c` + JSON.

## Quick start

```bash
gitlab config set-token --host gitlab.example.com --token glpat-XXXX
gitlab version
gitlab mr list --project atoms/api --state opened | jq '.[].iid'
```

## Commands

| Command | Verbs |
|---|---|
| `project` | list, get, create, update, delete, fork, archive, unarchive |
| `group` | list, get, create, update, delete, members, projects, subgroups |
| `mr` | list, get, create, update, close, reopen, merge, rebase, approve, unapprove, diffs, commits, changes, pipelines |
| `issue` | list, get, create, update, close, reopen, link, unlink, move, stats |
| `pipeline` | list, get, create, retry, cancel, delete, variables |
| `job` | list, get, play, retry, cancel, erase, trace, artifacts |
| `commit` | list, get, create, diff, comments, statuses, cherry-pick, revert, refs |
| `branch` | list, get, create, delete, protect, unprotect |
| `tag` | list, get, create, delete, protect, unprotect |
| `file` | get, create, update, delete, blame, raw |
| `repo` | tree, archive, compare, contributors, merge-base |
| `user` | list, get, me, keys, emails |
| `label` | list, get, create, update, delete, subscribe, unsubscribe |
| `note` | list, get, create, update, delete (issue/mr/commit/snippet) |
| `discussion` | list, get, resolve, unresolve (issue/mr/commit) |
| `search` | global/group/project scopes |
| `api` | `GET/POST/PUT/PATCH/DELETE <path>` escape hatch |

## Auth

Resolution order: `--token` > `GITLAB_TOKEN` > `~/.config/gitlab-cli/config.toml`.

## Output / errors / exit codes

- `stdout`: JSON (object for `get`/`create`/`update`/action-returning-body; array for `list`; NDJSON with `--output ndjson`).
- `stderr`: structured error JSON when a command fails.
- Exit codes: 0 success, 2 invalid args, 3 unauthorized, 4 forbidden, 5 not found, 6 conflict, 7 rate-limited, 8 server error, 9 network/timeout, 10 dry-run.

## Version caveat

This CLI is **frozen against GitLab 14.0.5-ee**. Fields and endpoints differ from 15.x+ — output is passed through unmodified.

## Known GitLab 14.0.5 API quirks

These are server-side behaviors of GitLab 14.0.5-ee that surprise agents. CLI does not paper over them — they're documented here so you know what to expect:

| Area | What happens | Workaround |
|---|---|---|
| `mr commits.parent_ids` | Always returns `[]` | Use `gitlab commit get --sha <id>` to fetch full commit including parent_ids |
| `mr diffs` endpoint | 404 (introduced in 15.7) | Use `gitlab mr changes` (single object with all file diffs) |
| `/raw_diffs` endpoint | 404 (introduced in 16.4) | Use `gitlab mr changes` and read each file's `.diff` field |
| Project Access Tokens | Available but CLI doesn't support | Use a user-scoped PAT |
| `users` endpoint extras | Some fields missing vs newer versions | Use `gitlab api GET /user/...` with explicit field selection if needed |
| Pagination caps | `per_page` max is 100 | CLI auto-paginates; use `--limit N` to cap |
| MR `approve` (EE) | 403 if license expired | Approval requires EE license + reviewer PAT |
| Write confirmation | TTY prompt blocks scripts | Set `GITLAB_ASSUME_YES=1` or `assume_yes=true` per host in config.toml |

Run `gitlab manifest` (and `gitlab manifest <command>`) for a JSON-formatted view of these quirks plus per-command examples — agents should consume that rather than this table.

## License

MIT.
