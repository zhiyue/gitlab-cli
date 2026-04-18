#!/usr/bin/env bash
set -u
GL=/Users/zhiyue/workspace/gitlab-cli/target/release/gitlab
PROJ="solutions_general/metagpt/metagptx-backend"
PASS=0; FAIL=0; FAILED=()

run() {
  local name="$1"; shift
  local expect="${1:-0}"; shift
  local out; out=$("$@" 2>&1); local code=$?
  if [ "$code" = "$expect" ]; then
    PASS=$((PASS+1)); printf "  ✓ %-55s exit=%d\n" "$name" "$code"
  else
    FAIL=$((FAIL+1)); FAILED+=("$name (exit=$code want=$expect)")
    printf "  ✗ %-55s exit=%d (want %s)\n" "$name" "$code" "$expect"
    echo "    $(echo "$out" | head -c 250)"
  fi
}

PROJ_ID=$($GL project get "$PROJ" 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')

echo "── 找最近一条 success pipeline (最完整的 job 集合) ──"
PID=$($GL pipeline list --project "$PROJ" --limit 30 2>/dev/null | python3 -c "
import json,sys
ps=json.load(sys.stdin)
done=[p for p in ps if p['status']=='success']
print(done[0]['id'] if done else (ps[0]['id'] if ps else ''))
")
echo "  pipeline id=$PID"

echo
echo "── pipeline 元信息 ──"
$GL pipeline get --project "$PROJ" --id $PID | python3 -c "
import json,sys
d=json.load(sys.stdin)
fields=['id','status','ref','sha','tag','user','created_at','updated_at','started_at','finished_at','duration','queued_duration','source','web_url']
for f in fields:
    v=d.get(f)
    if isinstance(v,dict): v=v.get('username') or v.get('name')
    print(f'  {f:<18} = {v}')
"

echo
echo "── pipeline variables (pipeline 触发时定义的变量) ──"
$GL pipeline variables --project "$PROJ" --id $PID 2>/dev/null | python3 -c "
import json,sys
vs=json.load(sys.stdin)
print(f'  count: {len(vs)}')
for v in vs[:10]:
    print(f'  - {v.get(\"key\")}={v.get(\"value\",\"\")[:60]}')"

echo
echo "── jobs in pipeline ──"
JOBS=$($GL job list --project "$PROJ" --pipeline $PID 2>/dev/null)
N_JOBS=$(echo "$JOBS" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)))')
echo "  count: $N_JOBS"
echo "$JOBS" | python3 -c "
import json,sys
js=json.load(sys.stdin)
for j in js:
    dur=j.get('duration') or 0
    print(f'  - id={j[\"id\"]:>9} stage={j.get(\"stage\",\"?\"):<24} name={j[\"name\"]:<30} status={j[\"status\"]:<10} dur={dur:>6.1f}s')
"

echo
echo "── ⭐ 拉每个 job 的 trace 头尾 (这是 pipeline 全部相关日志) ──"
echo "$JOBS" | python3 -c "
import json,sys
js=json.load(sys.stdin)
for j in js:
    if j['status'] in ('success','failed'):
        print(j['id'], j['name'])
" | while read JID JNAME; do
  echo
  echo "  ╭── job $JID ($JNAME) ──"
  TRACE=$($GL job trace --project "$PROJ" --id $JID 2>/dev/null)
  TC=$?
  if [ "$TC" = "0" ] && [ -n "$TRACE" ]; then
    PASS=$((PASS+1))
    BYTES=${#TRACE}
    LINES=$(echo "$TRACE" | wc -l | tr -d ' ')
    echo "  │  ✓ trace ok (lines=$LINES bytes=$BYTES)"
    echo "  │  ── head 5 lines ──"
    echo "$TRACE" | head -5 | sed 's/^/  │   /'
    if [ "$LINES" -gt 10 ]; then
      echo "  │  ── tail 5 lines ──"
      echo "$TRACE" | tail -5 | sed 's/^/  │   /'
    fi
  else
    FAIL=$((FAIL+1)); FAILED+=("trace job $JID")
    echo "  │  ✗ trace exit=$TC"
  fi
  echo "  ╰─"
done

echo
echo "── pipeline-level: bridges (downstream / parent-child pipelines) ──"
BRIDGES=$($GL api GET "/projects/$PROJ_ID/pipelines/$PID/bridges" 2>&1)
echo "$BRIDGES" | python3 -c "
import json,sys
try:
    bs=json.load(sys.stdin)
    if isinstance(bs, dict) and 'error' in bs:
        print('  (no access:', bs['error'].get('message','?')[:80])
    else:
        print(f'  count: {len(bs)}')
        for b in bs[:5]:
            print(f'  - bridge id={b.get(\"id\")} status={b.get(\"status\")} downstream={(b.get(\"downstream_pipeline\") or {}).get(\"id\")}')
except: print('  parse error')
"

echo
echo "── pipeline-level: test report ──"
TEST_REPORT=$($GL api GET "/projects/$PROJ_ID/pipelines/$PID/test_report" 2>&1)
echo "$TEST_REPORT" | python3 -c "
import json,sys
try:
    r=json.load(sys.stdin)
    if isinstance(r, dict) and 'error' in r:
        print('  (no test report:', r['error'].get('message','?')[:80])
    else:
        print(f'  total_time={r.get(\"total_time\")} count={r.get(\"total_count\")} success={r.get(\"success_count\")} failed={r.get(\"failed_count\")} skipped={r.get(\"skipped_count\")}')
        suites=r.get('test_suites') or []
        for s in suites[:5]:
            print(f'  - suite {s.get(\"name\")}: {s.get(\"total_count\")} tests, {s.get(\"failed_count\")} failed')
except: print('  parse error')
"

echo
echo "── pipeline-level: events (审计) ──"
EVENTS=$($GL api GET "/projects/$PROJ_ID/events" --query "action=created" --query "target_type=Pipeline" 2>&1)
echo "$EVENTS" | python3 -c "
import json,sys
try:
    es=json.load(sys.stdin)
    if isinstance(es, dict) and 'error' in es:
        print('  (no events:', es['error'].get('message','?')[:80])
    else:
        related=[e for e in es if str((e.get('target_id') or '')) == '$PID' or str(e.get('target_iid','')) == '$PID']
        print(f'  total project events: {len(es)}, related to pipeline $PID: {len(related)}')
except: print('  parse error')
"

echo
echo "── 用 ndjson 拉一长串 jobs (验证 pagination 在 job list 上 work) ──"
ALL_JOBS_COUNT=$($GL --output ndjson job list --project "$PROJ" --limit 50 2>/dev/null | wc -l | tr -d ' ')
PASS=$((PASS+1))
echo "  ✓ ndjson job list --limit 50 produced $ALL_JOBS_COUNT lines"

echo
echo "════════════════════════════════════════"
echo "  PASS: $PASS  FAIL: $FAIL"
[ $FAIL -gt 0 ] && printf "  - %s\n" "${FAILED[@]}"
echo "════════════════════════════════════════"
exit $FAIL
