#!/usr/bin/env bash
# Claim the next pending task from the queue branch.
# Usage (from your NightfirePort clone): bash <queue dir>/claim.sh <worker-name>
# Prints "TASK <id> <path to the task file>" or "EMPTY". The claim is atomic: if another worker
# pushed first, it retries with the next task.
set -u
W="${1:?worker name}"
Q="$(cd "$(dirname "$0")" && pwd)"
cd "$Q"
for attempt in 1 2 3 4 5 6 7 8; do
  git fetch -q origin queue && git reset -q --hard origin/queue || { sleep 5; continue; }
  f="$(ls pending/*.md 2>/dev/null | sort | head -1)"
  if [ -z "$f" ]; then echo "EMPTY"; exit 0; fi
  id="$(basename "$f" .md)"
  git mv "$f" "claimed/$id.md"
  printf '\n---\nclaimed-by: %s at %s\n' "$W" "$(date -u +%Y-%m-%dT%H:%MZ)" >> "claimed/$id.md"
  git add "claimed/$id.md"
  git commit -qm "claim $id ($W)"
  if git push -q origin HEAD:queue 2>/dev/null; then
    echo "TASK $id $Q/claimed/$id.md"; exit 0
  fi
  sleep $((RANDOM % 5 + 1))
done
echo "CLAIM-FAILED (could not push after 8 attempts)"; exit 1
