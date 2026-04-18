# Releasing gitlab-cli

End-to-end release SOP for `gitlab-cli`. Last validated against the v0.2.1 release on 2026-04-18.

## TL;DR — happy path

```bash
# 1. Ensure clean working tree on main
git checkout main && git pull origin main && git status

# 2. Bump workspace version + CHANGELOG entry
vim Cargo.toml                 # workspace.package.version = "X.Y.Z"
vim CHANGELOG.md               # Add ## [X.Y.Z] — YYYY-MM-DD with notes

# 3. Build to update Cargo.lock + sanity check
cargo build --release -p gitlab-cli
./target/release/gitlab --version    # Verify 'gitlab X.Y.Z (...)'

# 4. Run the pre-flight gauntlet locally (same as CI)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo deny check

# 5. Commit, tag, push
git add Cargo.toml Cargo.lock CHANGELOG.md
git -c commit.gpgsign=false commit -m "chore(release): bump 0.x.y → X.Y.Z"
git -c commit.gpgsign=false tag -a vX.Y.Z -m "vX.Y.Z — <one-line summary>"
git push origin main
git push origin vX.Y.Z

# 6. Watch the pipeline (~3 minutes)
gh run watch $(gh run list --workflow=release.yml --limit=1 --json databaseId --jq '.[0].databaseId') -R zhiyue/gitlab-cli --exit-status

# 7. Verify
brew update && brew upgrade zhiyue/tap/gitlab-cli
gitlab --version    # Should print X.Y.Z
```

If any step fails, see [Recovery](#recovery).

---

## Versioning policy

We follow [SemVer](https://semver.org). For this CLI:

- **MAJOR** (`1.0.0`): breaking change to CLI surface (renamed/removed command or flag, exit code remap, JSON shape break that agents could rely on).
- **MINOR** (`0.X.0`): new command or verb, new flag, new error.hint entry, new manifest field.
- **PATCH** (`0.X.Z`): bug fix, dependency bump, performance improvement, doc fix that ships in the binary (e.g., `manifest_data.toml`).

Pre-1.0 (`0.x.z`): we still try not to break things, but the second number may break occasionally. Document any breaks loudly in CHANGELOG.

---

## Pre-flight checklist

Before tagging, every item below must be true:

- [ ] Working tree clean (`git status`)
- [ ] You're on `main` and up-to-date with `origin/main`
- [ ] `Cargo.toml` `workspace.package.version` matches the new tag (without the `v` prefix)
- [ ] `CHANGELOG.md` has a section `## [X.Y.Z] — YYYY-MM-DD` with non-empty entries
- [ ] `cargo build --release -p gitlab-cli` succeeds locally
- [ ] `./target/release/gitlab --version` reports the new version
- [ ] `cargo fmt --all -- --check` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace --locked` all pass
- [ ] `cargo deny check` passes (license + advisory + bans)

Don't skip any of these. They're cheap; production debugging post-tag isn't.

---

## What the release pipeline does

When you `git push origin vX.Y.Z`, `.github/workflows/release.yml` runs:

```
git push origin vX.Y.Z
        │
        ▼
release.yml triggered (auto, ~3 min total on green path)
   ├── create-release          ~5 sec
   │     • taiki-e/create-gh-release-action
   │     • Creates GitHub Release with body from CHANGELOG.md section
   │
   ├── build (matrix × 6 platforms in parallel)   ~2-3 min
   │     • aarch64-apple-darwin (macos-14)
   │     • x86_64-unknown-linux-gnu (ubuntu-latest)
   │     • aarch64-unknown-linux-gnu (ubuntu-latest)
   │     • x86_64-unknown-linux-musl (ubuntu-latest)
   │     • aarch64-unknown-linux-musl (ubuntu-latest)
   │     • x86_64-pc-windows-msvc (windows-latest)
   │     • Each: cargo build --release → tar.gz/zip → sha256
   │     • Uploaded as Release attachments
   │     • Note: x86_64-apple-darwin (Intel Mac) is NOT built
   │       (macos-13 runner deprecated; formula has `depends_on arch: :arm64`)
   │
   └── bump-homebrew           ~10 sec  (needs: build)
         • Step A: scripts/update-homebrew-formula.sh vX.Y.Z
         •   Fetches sha256 for mac-arm + linux-arm + linux-x86 from Release
         •   Regenerates dist/homebrew/gitlab-cli.rb from template
         • Step B: commits dist/homebrew/gitlab-cli.rb back to main
         •   ("chore(homebrew): update formula for vX.Y.Z")
         • Step C: pushes Formula/gitlab-cli.rb to zhiyue/homebrew-tap
         •   (uses HOMEBREW_TAP_TOKEN secret + HOMEBREW_TAP_REPO variable)
```

Total elapsed: ~3-5 minutes on green path. After this completes, `brew install zhiyue/tap/gitlab-cli` immediately resolves to the new version.

---

## Verification

After the pipeline turns all-green, sanity check end-to-end:

```bash
# 1. Formula on tap should be updated
gh api repos/zhiyue/homebrew-tap/contents/Formula/gitlab-cli.rb \
  --jq '.content' | base64 -d | grep '^  version'
# Expected: version "X.Y.Z"

# 2. Main repo also got the auto-commit
git pull origin main
git log --oneline -3
# Expected: top commit "chore(homebrew): update formula for vX.Y.Z"

# 3. brew sees and installs the new version
brew update
brew upgrade zhiyue/tap/gitlab-cli
gitlab --version
# Expected: "gitlab X.Y.Z (target=..., git=<sha>)"

# 4. Real instance smoke
gitlab version
gitlab me
```

If any of these fail, see [Recovery](#recovery) below.

---

## Recovery

### Symptom: CI fails before tag is pushed

You haven't released anything yet. Just fix locally, commit, and start over from the [pre-flight checklist](#pre-flight-checklist). No tag means no public artifact.

### Symptom: tag pushed, but release.yml fails

Common causes (and what we hit during v0.2.0):

| Failure | Fix |
|---|---|
| `cargo deny check` rejects a new license | Add to `[licenses].allow` in `deny.toml`, commit, then either bump to v(X.Y.Z+1) or delete+repush the same tag (see below) |
| `cargo fmt --check` fails | `cargo fmt --all` locally, commit, bump or repush tag |
| One platform build fails (e.g., runner deprecated) | Drop it from the matrix in `release.yml` AND from `update-homebrew-formula.sh` AND from the formula template; commit; bump or repush |
| `bump-homebrew` Step A 404s on sha256 | Verify URL pattern in `scripts/update-homebrew-formula.sh` matches what `taiki-e/upload-rust-binary-action` actually emits (it's `<stem>.sha256`, not `<stem>.tar.gz.sha256`) |
| `bump-homebrew` Step C silently skipped | Check `HOMEBREW_TAP_TOKEN` secret + `HOMEBREW_TAP_REPO` variable are set on the repo. The `if:` condition in the workflow checks both; missing either causes a silent skip |

#### Re-running on the same tag (only OK pre-publication)

```bash
# 1. Fix in main
git push origin main

# 2. Force-update the tag to the new HEAD
git push origin :refs/tags/vX.Y.Z      # Delete remote tag
git tag -d vX.Y.Z                      # Delete local tag
git tag -a vX.Y.Z -m "..."             # Recreate at new HEAD
git push origin vX.Y.Z                 # Re-push, triggers release.yml again
```

⚠️ **Only do this if no one has installed the version yet.** If users have already done `brew install` of vX.Y.Z, they have the OLD binary cached. Better to bump to vX.Y.Z+1.

#### Bumping to next patch (recommended for any user-visible release)

Same flow as a fresh release, just with the next patch number. CHANGELOG should explain "0.X.Z fixes the bump-homebrew script that broke 0.X.(Z-1)" or similar.

### Symptom: release green but `brew install` doesn't see new version

```bash
brew untap zhiyue/tap
brew tap zhiyue/tap
brew install zhiyue/tap/gitlab-cli
```

If still wrong, manually inspect: `gh api repos/zhiyue/homebrew-tap/contents/Formula/gitlab-cli.rb --jq '.content' | base64 -d`. The version string in the formula is what brew uses.

### Symptom: pipeline succeeded but a binary is broken

Pull the binary from GitHub Releases and reproduce locally:

```bash
curl -sSL https://github.com/zhiyue/gitlab-cli/releases/download/vX.Y.Z/gitlab-cli-vX.Y.Z-aarch64-apple-darwin.tar.gz | tar xz
./gitlab --version
```

If broken, treat the release as toxic:

1. Mark the GitHub Release as "Pre-release" (so it's not the "latest")
2. Bump to vX.Y.Z+1 with the fix

We do not delete published releases — that breaks anyone with the binary cached.

---

## One-time setup (already done; for reference)

These are configured on the repo and don't need to be done again unless rotating credentials.

### GitHub Actions secrets / variables

```bash
# 1. Fine-grained PAT scoped to zhiyue/homebrew-tap, contents:write
#    Generate at https://github.com/settings/personal-access-tokens/new
echo 'github_pat_NEW_xxxx' | gh secret set HOMEBREW_TAP_TOKEN -R zhiyue/gitlab-cli

# 2. Tap repo (so the workflow knows where to push)
gh variable set HOMEBREW_TAP_REPO -R zhiyue/gitlab-cli -b "zhiyue/homebrew-tap"
```

### Rotating the tap PAT

Tokens expire (default 90 days for fine-grained PATs). Rotate before expiry:

1. Generate a new PAT with the same scope (`zhiyue/homebrew-tap` contents:write)
2. `echo 'github_pat_NEW' | gh secret set HOMEBREW_TAP_TOKEN -R zhiyue/gitlab-cli`
3. Revoke the old PAT at https://github.com/settings/personal-access-tokens

If the workflow's `Push to Homebrew tap (optional)` step starts silently skipping after a date, the PAT has expired.

### Initial `gh repo create` (already done)

```bash
gh repo create zhiyue/gitlab-cli \
  --public \
  --description "Agent-first CLI for legacy GitLab EE 14.0.5" \
  --source=. \
  --remote=origin \
  --push
```

---

## Things to avoid

- **Force-pushing tags after publication.** Anyone who installed the version still has the old binary; you'll create silent skew.
- **Deleting GitHub Releases.** Same reason. Mark as pre-release if needed; bump to fix.
- **Skipping CHANGELOG.** `taiki-e/create-gh-release-action` reads it for the Release body. No section = empty release notes.
- **Editing `dist/homebrew/gitlab-cli.rb` by hand.** It's regenerated every release. Edit `scripts/update-homebrew-formula.sh` if you need to change the template.
- **Bumping `Cargo.toml` version without testing locally.** `gitlab --version` reads `CARGO_PKG_VERSION` at compile time; if you tag without bumping, the binary will lie about its version (we hit this on v0.2.0 → fixed in v0.2.1).
- **Using `git commit -S` (sign) without verifying it works in CI.** Workflows can't sign; the bump-homebrew job uses `commit.gpgsign=false` explicitly.

---

## Reference: validated release log

| Tag | Date | Outcome |
|---|---|---|
| `v0.1.0` | 2026-04-17 | Local-only (never published; superseded by v0.2.0) |
| `v0.2.0` | 2026-04-18 | First public release; **bump-homebrew failed initially** due to wrong sha256 URL pattern; formula manually pushed to tap; binary mistakenly reported `0.1.0` because Cargo.toml version wasn't bumped |
| `v0.2.1` | 2026-04-18 | **First fully self-driving release.** Cargo.toml + CHANGELOG bumped, all 6 platform builds + bump-homebrew + tap push completed in ~3 min unattended. This is the SOP-validated path |

If a future release deviates from this SOP, document the deviation here so the next person knows.
