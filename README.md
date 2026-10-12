# NightfirePort task queue

This branch (`queue`) holds the task queue for the sandbox workers. It's separate from the code (`master`) on purpose.

| Folder | Contents |
|---|---|
| `pending/` | Tasks waiting. Workers take them in **filename order**, so prefix a priority: `10-fix-truck.md` runs before `50-fix-hud.md`. |
| `claimed/` | Tasks a worker is on now. The file ends with `claimed-by: <worker> at <time>`. |
| `done/` | Finished tasks, with the worker's report appended. |
| `blocked/` | Tasks the worker could not finish, with the reason appended. |

## For the coordinator

- **Adding a task:** commit a file to `pending/` holding only the task body. Use the same text as a prompt's "Your task" section and anything after it: what to do, files, branch name `s/<id>`.
  - **No credentials, tokens or private URLs:** this branch is public. Workers already have the setup and rules from their starting prompt.
- **Reading results:** `done/<id>.md` and `blocked/<id>.md` end with the report.
- **Requeueing a stale claim** (a sandbox died): `git mv claimed/<id>.md pending/<id>.md`, then remove the trailing `claimed-by` block, commit and push.

## For workers

- Claim the next task with `bash claim.sh <worker-name>`. It prints `TASK <id> <file>` or `EMPTY`.
- Finish with `bash finish.sh <id> done|blocked <report.md>`.
- Claims and finishes are atomic. If two workers race, the push rejects one, and it retries.
