#!/usr/bin/env bash
# Mark a claimed task finished (or blocked) and attach the report.
# Usage: bash <queue dir>/finish.sh <task-id> done|blocked <report.md>
set -u
id="${1:?task id}"; status="${2:?done or blocked}"; report="${3:?report file}"
case "$status" in done|blocked) ;; *) echo "status must be done or blocked"; exit 1;; esac
Q="$(cd "$(dirname "$0")" && pwd)"
cd "$Q"
for attempt in 1 2 3 4 5 6 7 8; do
  git fetch -q origin queue && git reset -q --hard origin/queue || { sleep 5; continue; }
  if [ ! -f "claimed/$id.md" ]; then echo "claimed/$id.md not found"; exit 1; fi
  git mv "claimed/$id.md" "$status/$id.md"
  printf '\n---\n## Report (%s, %s)\n\n' "$status" "$(date -u +%Y-%m-%dT%H:%MZ)" >> "$status/$id.md"
  cat "$report" >> "$status/$id.md"
  git add "$status/$id.md"
  git commit -qm "$status $id"
  if git push -q origin HEAD:queue 2>/dev/null; then echo "OK $status $id"; exit 0; fi
  sleep $((RANDOM % 5 + 1))
done
echo "FINISH-FAILED"; exit 1
