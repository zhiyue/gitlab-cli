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

echo "── 项目 metadata ──"
META=$($GL project get "$PROJ" 2>&1)
echo "$META" | python3 -c "
import json,sys
d=json.load(sys.stdin)
print(f'  id={d[\"id\"]} default_branch={d[\"default_branch\"]} visibility={d[\"visibility\"]} jobs_enabled={d[\"jobs_enabled\"]}')
print(f'  namespace={d[\"namespace\"][\"full_path\"]}')
print(f'  last_activity={d[\"last_activity_at\"]}')
"
PROJ_ID=$(echo "$META" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')

echo
echo "── 1. pipeline list (最近 10 条) ──"
PL=$($GL pipeline list --project "$PROJ" --limit 10 2>&1)
echo "$PL" | python3 -c "
import json,sys
ps=json.load(sys.stdin)
print(f'  total: {len(ps)}')
for p in ps:
    print(f'  - id={p[\"id\"]:>7} sha={p[\"sha\"][:8]} ref={p[\"ref\"]:<30} status={p[\"status\"]:<10} created={p[\"created_at\"]}')
"
run "pipeline list" 0 $GL pipeline list --project "$PROJ" --limit 5

echo
echo "── 2. 选一个最近完成的 pipeline ──"
PID=$(echo "$PL" | python3 -c "
import json,sys
ps=json.load(sys.stdin)
done=[p for p in ps if p['status'] in ('success','failed','canceled')]
print(done[0]['id'] if done else '')
")
if [ -z "$PID" ]; then
  echo "  no completed pipeline found in last 10 — picking the newest of any status"
  PID=$(echo "$PL" | python3 -c 'import json,sys; ps=json.load(sys.stdin); print(ps[0]["id"] if ps else "")')
fi
echo "  selected pipeline id=$PID"
echo
echo "── 3. pipeline get / variables ──"
run "pipeline get $PID"           0 $GL pipeline get --project "$PROJ" --id $PID
run "pipeline variables $PID"     0 $GL pipeline variables --project "$PROJ" --id $PID

echo
echo "── 4. job list (按 pipeline) ──"
run "job list --pipeline $PID"    0 $GL job list --project "$PROJ" --pipeline $PID
JOBS=$($GL job list --project "$PROJ" --pipeline $PID 2>/dev/null)
echo "$JOBS" | python3 -c "
import json,sys
js=json.load(sys.stdin)
print(f'  total jobs in pipeline: {len(js)}')
for j in js[:8]:
    print(f'  - job id={j[\"id\"]:>9} stage={j.get(\"stage\",\"?\"):<10} name={j[\"name\"]:<40} status={j[\"status\"]}')
"
JID=$(echo "$JOBS" | python3 -c "
import json,sys
js=json.load(sys.stdin)
done=[j for j in js if j['status'] in ('success','failed')]
print(done[0]['id'] if done else (js[0]['id'] if js else ''))
")
echo "  selected job id=$JID for trace test"

echo
echo "── 5. ⭐ job trace (CI 日志读取主测试) ──"
TRACE_OUT=$($GL job trace --project "$PROJ" --id $JID 2>&1)
TC=$?
if [ "$TC" = "0" ]; then
  PASS=$((PASS+1))
  BYTES=${#TRACE_OUT}
  LINES=$(echo "$TRACE_OUT" | wc -l | tr -d ' ')
  echo "  ✓ job trace exit=0 lines=$LINES bytes=$BYTES"
  echo
  echo "  ── 日志开头 (前 30 行) ──"
  echo "$TRACE_OUT" | head -30 | sed 's/^/  | /'
  echo "  ── ...日志结尾 (后 20 行) ──"
  echo "$TRACE_OUT" | tail -20 | sed 's/^/  | /'
else
  FAIL=$((FAIL+1)); FAILED+=("job trace exit=$TC")
  echo "  ✗ job trace exit=$TC: $TRACE_OUT" | head -c 300
fi

echo
echo "── 6. job get (单条 job 详情) ──"
run "job get $JID"   0 $GL job get --project "$PROJ" --id $JID

echo
echo "── 7. job artifacts (是否有 artifacts) ──"
$GL job artifacts --project "$PROJ" --id $JID > /tmp/cli_artifacts.bin 2>&1
AC=$?
if [ "$AC" = "0" ]; then
  SIZE=$(stat -f%z /tmp/cli_artifacts.bin 2>/dev/null || stat -c%s /tmp/cli_artifacts.bin)
  PASS=$((PASS+1))
  echo "  ✓ job artifacts exit=0 size=${SIZE} bytes"
  file /tmp/cli_artifacts.bin | head -1
elif [ "$AC" = "5" ]; then
  PASS=$((PASS+1))
  echo "  ✓ job artifacts exit=5 (404 = no artifacts on this job, expected)"
else
  FAIL=$((FAIL+1)); FAILED+=("job artifacts exit=$AC")
fi

echo
echo "── 8. 用 api 逃生门验证: GET /jobs/:id/trace 直接拉 ──"
RAW=$($GL api GET "/projects/$PROJ_ID/jobs/$JID/trace" 2>&1)
RC=$?
if [ "$RC" = "0" ] && [ ${#RAW} -gt 0 ]; then
  PASS=$((PASS+1))
  echo "  ✓ api GET .../trace exit=0 bytes=${#RAW}"
  if [ "${#RAW}" = "${#TRACE_OUT}" ]; then
    PASS=$((PASS+1)); echo "  ✓ api raw and 'job trace' subcommand return identical bytes"
  else
    echo "  ⚠ api raw=${#RAW}b vs 'job trace'=${#TRACE_OUT}b — 差异 (可能为 ANSI 颜色 / trailing 字节)"
  fi
fi

echo
echo "── 9. 找 failed job + 看错误日志 (如果有) ──"
FAIL_JID=$(echo "$JOBS" | python3 -c "
import json,sys
js=json.load(sys.stdin)
fj=[j for j in js if j['status']=='failed']
print(fj[0]['id'] if fj else '')
")
if [ -n "$FAIL_JID" ]; then
  echo "  found failed job id=$FAIL_JID"
  FAIL_TRACE=$($GL job trace --project "$PROJ" --id $FAIL_JID 2>&1)
  FC=$?
  if [ "$FC" = "0" ]; then
    PASS=$((PASS+1))
    echo "  ✓ failed-job trace exit=0 (${#FAIL_TRACE} bytes)"
    echo "  ── failure tail (后 15 行) ──"
    echo "$FAIL_TRACE" | tail -15 | sed 's/^/  | /'
  fi
else
  echo "  • this pipeline has no failed jobs to trace"
fi

echo
echo "── 10. 跨 pipeline 找不同 status 的 job 对照 ──"
echo "$PL" | python3 -c "
import json,sys
ps=json.load(sys.stdin)
from collections import Counter
c=Counter(p['status'] for p in ps)
print(f'  pipeline status mix in last 10: {dict(c)}')
"

echo
echo "════════════════════════════════════════"
echo "  PASS: $PASS  FAIL: $FAIL"
[ $FAIL -gt 0 ] && printf "  - %s\n" "${FAILED[@]}"
echo "════════════════════════════════════════"
exit $FAIL
