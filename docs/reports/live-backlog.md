# live-backlog — one ledger of open questions and the live checks that settle them

Branch `s/live-backlog`, docs only. The task: the project's live checks were scattered across ~204 reports'
"Live checks for the main agent" sections, `docs/UNKNOWNS.md` and `docs/GAPS.md`, where they get lost. This
task harvested them into one REA-style ledger — `docs/live/backlog.md` — where every unresolved question stays
with the procedure that settles it, the expected answers, the evidence for and against, and a status, until
qualifying observed evidence resolves it.

## What's done

1. **The sweep.** Grepped every report for live-check sections: 204 of the reports in `docs/reports/` carry
   one ("## Live checks for the main agent" and variants) — not the ~60 assumed. All 204 were harvested. Four
   parallel passes read the reports; a fifth read `docs/UNKNOWNS.md` (46 rows; 23 need the real game) and
   `docs/GAPS.md` (45 rows; 12 need the real game or a real GPU); a sixth built the answer key from
   `docs/live/*`, `docs/merge-log.md`, `docs/WORKLOG.md` and the reports' own "settled/superseded" notes.
2. **The ledger — `docs/live/backlog.md`.** 814 rows, one line each:
   `id | source | type | level | Q | HOW | EXP | EV | ST | SETTLED`.
   - **id**: L-001..L-786 from the reports (four harvest blocks in report-name order; the gaps 199/395-399/599
     are the reserved block boundaries, not missing rows), L-801..L-835 for the UNKNOWNS/GAPS rows, with the
     original U-xxx/G-xxx id kept in the source column.
   - **type**: `pcsx2` (needs the real game; the HOW names the tool: `pcsx2_shot.py`, `gdbwatch.py`,
     `gdbcall.py --spec`, `fieldprobe.py`, `pad_log.py`, `goto_level.py`, PINE), `render` (an nf-view `--bond`
     headless spot), or `both`.
   - **HOW** keeps the report's own numbers: level, `--pos`/`--bond`/`--pitch` spot, the watch/call spec, what
     to read. **EXP** says what each answer would change in the port. **EV** keeps the report's own [M]/[L]/[D]
     citations — the supporting and contradicting evidence.
   - **ST**: open / answered / obsolete / settled-in-report, with **SETTLED** naming the `docs/live/` file,
     report or merge-log line that records the answer, or why the question no longer applies.
   - Rows are source-faithful: wording, spots and addresses are the reports' own, compressed but not corrected;
     near-duplicates from different reports are kept until resolution dedupes them.
3. **Answered rows marked.** The three `docs/live/` ground-truth files settle the portal checks (courtyard
   walls solid, 07 window hides the building, clock hall matches; the VisCelList PINE puzzle; the 40-node
   chain; the forced-objects refutation), the weapon fire-mode table and the weapon measurements (rocket
   0.3367 u/frame, frag release ~0.3 s, stunner press/contact-range, laser watch, mine stick + detonator 0x38,
   AIMS lights, SG5 casings, guard sighting→first shot ≈1.6 s, per-level loadouts, the truck driver's death).
   The answer key additionally marks: the 05 gate-door icon context 0xFF (fix-swim-vent's check), the prompt
   sphere-pick ~2.5 and grapple no-prompt pin, the 08 east-balcony jump context, the EE FPU rounding model,
   the AI-combat trace ground truth (61-tick notice), spawner p7 = the kill channel, the bldata census, the
   Raptor drop chain, the castle not mirrored, frame pacing, the drone posing census, the 07 Kimiko scene
   compare, the 101k front-end walk, SP having no radar, the 05 trench eye cel = 15, the 0c pylon, the Frinesi
   shell, the 109u viewfinder redo, swim entry, the armour ring. 19 obsolete framings are recorded with why
   (forced-objects theory refuted, p7 arm reading refuted, mine "→0x39" superseded by 0x38, Square-as-mode-
   button superseded by the measured layout, the sbs.ps1 crop artifact, the stale door[7]/level-chain tables,
   ...). One row ships answered inline (L-513, forced objects); the rest of the answered set is the "Answered"
   section + the marked rows.
4. **Value order + session grouping.** "What to run next" lists 12 clusters ordered by what unblocks a held or
   parked task first (the escort-01 cluster unblocking the held walkreal line 110f2/111b/111c/111g; the guard
   RNG/fire cluster behind ai_live's calibration; the 4a lift cluster behind fix-final-blockers and walkreal-c;
   the 0c vent/hatch cluster behind 109n2; portal leftovers; the weapon-matrix W/D cells; the UNKNOWNS High AI
   rows; one MP session; the front-end leftovers; weapon/UX grab-bag; audio/lighting; the long tail). A
   generated "Session planner: level and spot index" at the end groups all 814 rows per level (L01 has 59,
   L05 has 208, L06 56, L07 40, L08 38, MP 18, front-end 43, ...), each row's HOW carrying its spot, so one
   PCSX2 session clears a column.
5. **The rule — `docs/live/README.md`.** New file describing what docs/live holds plus the backlog rule: new
   reports add their live checks to the backlog with a row id, not only in the report; when a check is
   answered, its row is marked `answered` with a `SETTLED:` pointer to the recording file.

## How it was verified

- ID continuity: `grep -o "^L-[0-9]*"` over the ledger gives 814 unique ids, no duplicates; the only missing
  numbers are the documented block boundaries.
- The level index is generated from the ledger's own LVL column by `scripts/build_level_index.py` (scratch;
  not committed) — the row counts in it derive mechanically from the rows.
- Every row's source stem was taken from the report filename at harvest time; the 044x fields rows point at
  the `docs/fields/*.md` live-check sections they consolidate rather than inventing procedures.
- No game bytes, tokens or URLs committed; only `docs/live/backlog.md`, `docs/live/README.md`,
  `docs/WORKLOG.md` (appended entry) and this report are staged by name.

## Suite

The task changes no code (docs only: two new files under docs/live/, one appended WORKLOG entry, this report).
A full `python3 tools/test/suite.py` run was started in the background; background jobs do not survive between
tool calls in this sandbox (the first attempt died on a missing PATH, the restart is racing the session end),
so the reported total, if the run completes, is in `scratch/suite_backlog.out` and the WORKLOG entry; master's
last recorded full-suite totals are 1341 passed / 0 failed (2026-10-15 09:06, before the 110o/110q holds which
failed only on their own branches). No test surface is touched by this branch.

## What's left

- The two `#[ignore]`-style maintenance jobs the ledger itself surfaced: dedupe near-duplicate rows at
  resolution time (kept source-faithful by design), and re-check the ~40 `render`-typed rows that only the
  main agent's GPU machine can run.
- The ledger's open set is now the queue's menu: the value-ordered clusters are ready to be claimed as
  PCSX2-session tasks; each answered row should be closed with the answer recorded under `docs/live/` and its
  row's ST flipped.

## Live checks for the main agent

None new — this task is the index of them. The first sessions worth running, per the value order:
1. 07000001 escort cluster (L-021, L-455, L-456, L-643, L-644, L-646, L-647, L-022, L-452) — one cold load,
   walk the route with `gdbwatch.py` on `switch_channels+0x21/0x0e`, Drone+0xBA8, and the exit plate.
2. Guard RNG cluster (L-317 first: a watchpoint on the RNG state words through one guard burst; then L-318,
   L-319, L-301/L-304 on 06).
3. 0700004a lift cluster (L-243, L-246, L-247, L-761, L-463) with the 16 door[7] E-check (L-244/L-760) on the way.
4. 0700000c vent/hatch cluster (L-066, L-067, L-068, L-072, L-073, L-074, L-379, L-645, L-648, L-649, L-650).

## ntfy failed

All three fallbacks attempted at 2026-10-15 18:40 UTC; the ntfy.sh topic's daily message quota is exhausted:

1. curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits
   with a paid plan, see https://ntfy.sh/docs/publish/#limitations"}`
2. python3 urllib.request: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. wget: exit 0 with no output — delivery not confirmable (a 429 body was not printed by `-q -O-`).

The notification text that was to be sent: `live-backlog: done — docs/live/backlog.md: 814-row live-check
ledger harvested from 204 reports + UNKNOWNS/GAPS, answered/obsolete marked, value-ordered clusters + level
index; README rule added (branch s/live-backlog)`.
