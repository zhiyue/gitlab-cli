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

## License

MIT.
