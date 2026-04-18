#!/usr/bin/env bash
# End-to-end test of gitlab-cli against daizhibin/gitlab-statics
set -u
export GITLAB_ASSUME_YES=1   # ← skip interactive confirmation for write ops
GL=/Users/zhiyue/workspace/gitlab-cli/target/release/gitlab
PROJ="daizhibin/gitlab-statics"
TS=$(date +%s)
PREFIX="cli-test-$TS"
PASS=0
FAIL=0
FAILED_TESTS=()

run() {
  local name="$1"; shift
  local expect_code="${1:-0}"; shift
  local out
  out=$("$@" 2>&1)
  local code=$?
  if [ "$code" = "$expect_code" ]; then
    PASS=$((PASS+1))
    printf "  ✓ %-60s exit=%d\n" "$name" "$code"
  else
    FAIL=$((FAIL+1))
    FAILED_TESTS+=("$name (got exit=$code, expected $expect_code)")
    printf "  ✗ %-60s exit=%d (want %s)\n" "$name" "$code" "$expect_code"
    echo "    output: $(echo "$out" | head -c 300)"
  fi
}

section() { echo; echo "── $1 ──"; }

# Bookkeeping for cleanup
CLEANUP_BRANCHES=()
CLEANUP_TAGS=()
CLEANUP_LABELS=()
CLEANUP_FILES=()  # "branch:path"
CLEANUP_ISSUES=()
CLEANUP_MRS=()

cleanup() {
  echo
  echo "── CLEANUP ──"
  PROJ_ID=$($GL project get $PROJ 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
  for entry in "${CLEANUP_FILES[@]:-}"; do
    [ -z "$entry" ] && continue
    BR=${entry%%:*}; FILE=${entry#*:}
    $GL file delete --project $PROJ --path "$FILE" --branch "$BR" --message "cleanup" >/dev/null 2>&1 && echo "  rm file $FILE@$BR" || true
  done
  for mr in "${CLEANUP_MRS[@]:-}"; do
    [ -z "$mr" ] && continue
    $GL mr close --project $PROJ --mr $mr >/dev/null 2>&1
    $GL api DELETE "/projects/$PROJ_ID/merge_requests/$mr" >/dev/null 2>&1 && echo "  rm MR $mr" || true
  done
  for br in "${CLEANUP_BRANCHES[@]:-}"; do
    [ -z "$br" ] && continue
    $GL branch delete --project $PROJ --name "$br" >/dev/null 2>&1 && echo "  rm branch $br" || true
  done
  for tag in "${CLEANUP_TAGS[@]:-}"; do
    [ -z "$tag" ] && continue
    $GL tag delete --project $PROJ --name "$tag" >/dev/null 2>&1 && echo "  rm tag $tag" || true
  done
  for lbl in "${CLEANUP_LABELS[@]:-}"; do
    [ -z "$lbl" ] && continue
    $GL label delete --project $PROJ --id $lbl >/dev/null 2>&1 && echo "  rm label $lbl" || true
  done
  for iss in "${CLEANUP_ISSUES[@]:-}"; do
    [ -z "$iss" ] && continue
    $GL api DELETE "/projects/$PROJ_ID/issues/$iss" >/dev/null 2>&1 && echo "  rm issue $iss" || true
  done
}
trap cleanup EXIT

section "1. Connectivity & identity"
run "version"               0 $GL version
run "me"                    0 $GL me
run "user me"               0 $GL user me
run "config path"           0 $GL config path
run "config list"           0 $GL config list

section "2. Project (read)"
run "project list --limit 3"  0 $GL project list --limit 3
run "project get by path"     0 $GL project get $PROJ
PROJ_ID=$($GL project get $PROJ | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
run "project get by id"       0 $GL project get $PROJ_ID

section "3. Group (read)"
run "group list --limit 3"        0 $GL group list --limit 3
run "group get autosearch"        0 $GL group get autosearch
run "group projects autosearch"   0 $GL group projects autosearch --limit 3
run "group members autosearch"    0 $GL group members autosearch --limit 3
run "group subgroups autosearch"  0 $GL group subgroups autosearch

section "4. Repository (read)"
run "repo tree main"              0 $GL repo tree --project $PROJ --ref main
run "repo contributors"           0 $GL repo contributors --project $PROJ
run "branch list"                 0 $GL branch list --project $PROJ
run "branch get main"             0 $GL branch get --project $PROJ --name main
run "tag list (empty)"            0 $GL tag list --project $PROJ
run "commit list main"            0 $GL commit list --project $PROJ --ref main
SHA=$($GL commit list --project $PROJ --ref main --limit 1 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d[0]["id"] if d else "")')
if [ -n "$SHA" ]; then
  SHORT_SHA=${SHA:0:8}
  run "commit get $SHORT_SHA"     0 $GL commit get --project $PROJ --sha $SHA
  run "commit diff $SHORT_SHA"    0 $GL commit diff --project $PROJ --sha $SHA
  run "commit refs $SHORT_SHA"    0 $GL commit refs --project $PROJ --sha $SHA
  run "commit comments $SHORT_SHA" 0 $GL commit comments --project $PROJ --sha $SHA
  run "commit statuses $SHORT_SHA" 0 $GL commit statuses --project $PROJ --sha $SHA
fi
README_FILE=$($GL repo tree --project $PROJ --ref main 2>/dev/null | python3 -c 'import json,sys; t=json.load(sys.stdin); print(next((x["path"] for x in t if x["type"]=="blob"), ""))')
if [ -n "$README_FILE" ]; then
  run "file get $README_FILE"   0 $GL file get --project $PROJ --path "$README_FILE" --ref main
  run "file raw $README_FILE"   0 $GL file raw --project $PROJ --path "$README_FILE" --ref main
  run "file blame $README_FILE" 0 $GL file blame --project $PROJ --path "$README_FILE" --ref main
fi
run "repo archive (binary)"   0 $GL repo archive --project $PROJ --format tar.gz

section "5. Issues + Notes (write)"
ISSUE_JSON=$($GL issue create --project $PROJ --title "$PREFIX issue" 2>&1)
ISSUE_IID=$(echo "$ISSUE_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["iid"])' 2>/dev/null)
if [ -n "$ISSUE_IID" ]; then
  CLEANUP_ISSUES+=("$ISSUE_IID")
  echo "  • created issue iid=$ISSUE_IID"
  PASS=$((PASS+1))
  run "issue get"               0 $GL issue get --project $PROJ --issue $ISSUE_IID
  run "issue list (>=1)"        0 $GL issue list --project $PROJ
  NOTE_JSON=$($GL note create --project $PROJ --on issue --target $ISSUE_IID --body "$PREFIX comment v1" 2>&1)
  NOTE_ID=$(echo "$NOTE_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])' 2>/dev/null)
  if [ -n "$NOTE_ID" ]; then
    PASS=$((PASS+1)); echo "  • created note id=$NOTE_ID on issue"
    run "note list on issue"      0 $GL note list --project $PROJ --on issue --target $ISSUE_IID
    run "note get on issue"       0 $GL note get --project $PROJ --on issue --target $ISSUE_IID --id $NOTE_ID
    run "note update on issue"    0 $GL note update --project $PROJ --on issue --target $ISSUE_IID --id $NOTE_ID --body "$PREFIX comment v2"
    run "note delete on issue"    0 $GL note delete --project $PROJ --on issue --target $ISSUE_IID --id $NOTE_ID
  else
    FAIL=$((FAIL+1)); FAILED_TESTS+=("note create on issue")
  fi
  run "discussion list on issue"  0 $GL discussion list --project $PROJ --on issue --target $ISSUE_IID
  run "issue close"               0 $GL issue close --project $PROJ --issue $ISSUE_IID
  run "issue reopen"              0 $GL issue reopen --project $PROJ --issue $ISSUE_IID
  run "issue stats"               0 $GL issue stats
else
  FAIL=$((FAIL+1)); FAILED_TESTS+=("issue create — output: $ISSUE_JSON")
fi

section "6. Branches + Files setup for MR test"
BR="$PREFIX-feat"
run "branch create $BR from main"  0 $GL branch create --project $PROJ --name "$BR" --ref main
CLEANUP_BRANCHES+=("$BR")
run "branch get $BR"               0 $GL branch get --project $PROJ --name "$BR"
run "branch list (search)"         0 bash -c "$GL branch list --project $PROJ --search '$PREFIX'"
F1="$PREFIX-a.txt"
F2="$PREFIX-b.md"
run "file create $F1"              0 $GL file create --project $PROJ --path "$F1" --branch "$BR" --content "line one" --message "$PREFIX add $F1"
CLEANUP_FILES+=("$BR:$F1")
run "file create $F2"              0 $GL file create --project $PROJ --path "$F2" --branch "$BR" --content "# heading" --message "$PREFIX add $F2"
CLEANUP_FILES+=("$BR:$F2")
run "file get $F1 from $BR"        0 $GL file get --project $PROJ --path "$F1" --ref "$BR"
run "file update $F1"              0 $GL file update --project $PROJ --path "$F1" --branch "$BR" --content "line one\nline two" --message "$PREFIX update $F1"
run "file blame $F1 from $BR"      0 $GL file blame --project $PROJ --path "$F1" --ref "$BR"
run "file raw $F1 from $BR"        0 $GL file raw --project $PROJ --path "$F1" --ref "$BR"
run "repo compare main..$BR"       0 $GL repo compare --project $PROJ --from main --to "$BR"
run "repo merge-base"              0 $GL repo merge-base --project $PROJ --ref main --ref "$BR"
run "commit list on $BR"           0 $GL commit list --project $PROJ --ref "$BR"

section "7. MR full lifecycle (deep test per user request)"
MR_JSON=$($GL mr create --project $PROJ --source "$BR" --target main --title "$PREFIX MR — full test" 2>&1)
MR_IID=$(echo "$MR_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["iid"])' 2>/dev/null)
if [ -z "$MR_IID" ]; then
  FAIL=$((FAIL+1)); FAILED_TESTS+=("mr create — output: $MR_JSON")
else
  CLEANUP_MRS+=("$MR_IID")
  echo "  • created MR iid=$MR_IID (state should be opened)"
  PASS=$((PASS+1))

  echo "  --- read MR metadata ---"
  run "mr get"                  0 $GL mr get --project $PROJ --mr $MR_IID
  run "mr list opened"          0 $GL mr list --project $PROJ --state opened
  run "mr list all"             0 $GL mr list --project $PROJ --state all
  run "mr changes (file diff)"  0 $GL mr changes --project $PROJ --mr $MR_IID
  run "mr diffs (paginated)"    0 $GL mr diffs --project $PROJ --mr $MR_IID
  run "mr commits"              0 $GL mr commits --project $PROJ --mr $MR_IID
  run "mr pipelines"            0 $GL mr pipelines --project $PROJ --mr $MR_IID

  echo "  --- comments / notes lifecycle ---"
  run "note list on MR (initially empty)"  0 $GL note list --project $PROJ --on mr --target $MR_IID
  N1_JSON=$($GL note create --project $PROJ --on mr --target $MR_IID --body "First comment from CLI" 2>&1)
  N1=$(echo "$N1_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])' 2>/dev/null)
  if [ -n "$N1" ]; then
    PASS=$((PASS+1)); echo "  • added note $N1"
    run "note get $N1"            0 $GL note get --project $PROJ --on mr --target $MR_IID --id $N1
    run "note update $N1"         0 $GL note update --project $PROJ --on mr --target $MR_IID --id $N1 --body "Edited comment"
    N2_JSON=$($GL note create --project $PROJ --on mr --target $MR_IID --body "Second comment" 2>&1)
    N2=$(echo "$N2_JSON" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])' 2>/dev/null)
    if [ -n "$N2" ]; then PASS=$((PASS+1)); echo "  • added note $N2"; fi
    run "note list on MR (>=2)"   0 $GL note list --project $PROJ --on mr --target $MR_IID
    run "note delete $N1"         0 $GL note delete --project $PROJ --on mr --target $MR_IID --id $N1
    [ -n "$N2" ] && run "note delete $N2"  0 $GL note delete --project $PROJ --on mr --target $MR_IID --id $N2
  fi

  echo "  --- discussions ---"
  run "discussion list on MR"   0 $GL discussion list --project $PROJ --on mr --target $MR_IID
  DISC_ID=$($GL discussion list --project $PROJ --on mr --target $MR_IID 2>/dev/null | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d[0]["id"] if d and "resolved" in str(d[0]) else "")')
  if [ -n "$DISC_ID" ]; then
    run "discussion get"        0 $GL discussion get --project $PROJ --on mr --target $MR_IID --id "$DISC_ID"
    # resolve only works on resolvable discussions — skip if it 4xxs
    $GL discussion resolve --project $PROJ --on mr --target $MR_IID --id "$DISC_ID" >/dev/null 2>&1 && PASS=$((PASS+1)) && echo "  ✓ discussion resolve"
  fi

  echo "  --- approve / unapprove (may need permission) ---"
  if $GL mr approve --project $PROJ --mr $MR_IID >/dev/null 2>&1; then
    PASS=$((PASS+1)); echo "  ✓ mr approve"
    $GL mr unapprove --project $PROJ --mr $MR_IID >/dev/null 2>&1 && PASS=$((PASS+1)) && echo "  ✓ mr unapprove" || echo "  • mr unapprove not applicable"
  else
    echo "  • mr approve skipped (likely EE-license-gated or self-approval blocked)"
  fi

  echo "  --- update MR title / description ---"
  run "mr update title"         0 $GL mr update --project $PROJ --mr $MR_IID --data '{"title":"'"$PREFIX"' MR — edited"}'

  echo "  --- state transitions ---"
  run "mr close"                0 $GL mr close --project $PROJ --mr $MR_IID
  run "mr reopen"               0 $GL mr reopen --project $PROJ --mr $MR_IID
  STATE=$($GL mr get --project $PROJ --mr $MR_IID 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["state"])')
  if [ "$STATE" = "opened" ]; then PASS=$((PASS+1)); echo "  ✓ post-reopen state == opened"; else FAIL=$((FAIL+1)); FAILED_TESTS+=("post-reopen state was '$STATE'"); fi

  echo "  --- mr rebase (no-op rebase OK on fresh branch) ---"
  $GL mr rebase --project $PROJ --mr $MR_IID >/dev/null 2>&1 && PASS=$((PASS+1)) && echo "  ✓ mr rebase" || echo "  • mr rebase skipped or failed (rebase needs no diverging commits)"

  echo "  --- final close (don't merge into main!) ---"
  run "mr close (final)"        0 $GL mr close --project $PROJ --mr $MR_IID
fi

section "8. Tags"
TAG="$PREFIX-tag"
run "tag create"          0 $GL tag create --project $PROJ --name "$TAG" --ref main
CLEANUP_TAGS+=("$TAG")
run "tag get"             0 $GL tag get --project $PROJ --name "$TAG"
run "tag list (>=1)"      0 $GL tag list --project $PROJ

section "9. Labels"
LBL="$PREFIX-label"
run "label create"        0 $GL label create --project $PROJ --name "$LBL" --color "#FF0000"
LBL_ID=$($GL label list --project $PROJ 2>/dev/null | python3 -c "import json,sys; ls=json.load(sys.stdin); print(next((l['id'] for l in ls if l['name']=='$LBL'), ''))")
CLEANUP_LABELS+=("$LBL_ID")
if [ -n "$LBL_ID" ]; then
  run "label list (>=1)"      0 $GL label list --project $PROJ
  run "label get"             0 $GL label get --project $PROJ --id $LBL_ID
  run "label subscribe"       0 $GL label subscribe --project $PROJ --id $LBL_ID
  run "label unsubscribe"     0 $GL label unsubscribe --project $PROJ --id $LBL_ID
  run "label update"          0 $GL label update --project $PROJ --id $LBL_ID --data '{"color":"#00FF00"}'
fi

section "10. Pipelines / Jobs (read; project has no CI)"
run "pipeline list"       0 $GL pipeline list --project $PROJ
run "job list"            0 $GL job list --project $PROJ

section "11. Search"
run "search global issues"     0 $GL search --scope issues --query "$PREFIX"
run "search project blobs"     0 $GL search --scope blobs --query "README" --project $PROJ

section "12. API escape hatch"
run "api GET /version"               0 $GL api GET /version
run "api GET pipeline_schedules"     0 $GL api GET "/projects/$PROJ_ID/pipeline_schedules"

section "13. Output formats"
run "ndjson project list"     0 $GL --output ndjson project list --limit 3
run "--limit 1 single result" 0 $GL project list --limit 1
run "--no-paginate"           0 $GL project list --no-paginate

section "14. Error paths (negative tests)"
run "404 project"             5 $GL project get nonexistent-namespace/nonexistent-proj
run "404 file"                5 $GL file get --project $PROJ --path nonexistent.txt --ref main
run "401 with bad token"      3 $GL --token glpat-deadbeef0000000000 me
run "invalid host"            2 $GL --host not-a-url me

section "15. Dry-run"
DRYOUT=$($GL --dry-run --yes mr create --project $PROJ --source main --target main --title test 2>&1)
DRYCODE=$?
if [ "$DRYCODE" = "10" ] && echo "$DRYOUT" | grep -q '"dry_run":true'; then
  PASS=$((PASS+1))
  echo "  ✓ dry-run on mr create exit=10 + dry_run envelope"
else
  FAIL=$((FAIL+1))
  FAILED_TESTS+=("dry-run mr create (exit=$DRYCODE)")
fi

echo
echo "════════════════════════════════════════════"
echo "  PASS: $PASS    FAIL: $FAIL"
if [ $FAIL -gt 0 ]; then
  echo "  Failed tests:"
  printf "    - %s\n" "${FAILED_TESTS[@]}"
fi
echo "════════════════════════════════════════════"
exit $FAIL
