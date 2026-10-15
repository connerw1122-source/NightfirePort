# fix-scores-handoff — the NewScoresRef PTP hand-off (G-045 remainder)

Branch `s/fix-scores-handoff`, 2026-10-15. Task 111e.

## What's done

**The research (task step 1).** `NewScoresRef` is a symtab name (0x30cc6c, 4 bytes, sbss), not a
decompile identifier. Its complete reference census:

- ACTION.ELF: exactly two references — `bootup` 0x1c7e78:86 (`NewScoresRef = &DAT_0031ca38`, the
  writer of the POINTER only) and `PlrStat_GetScore` 0x1984b0:45-64 (the reader) [M the
  whole-decompile grep]. A whole-ELF constant scan finds no other 0x31ca38/0x31c090 byte pattern
  in ACTION.ELF.
- DRIVING.ELF / SLUS_205.79: hardcode NONE of the PTP addresses (binary constant scan) — the
  driving side receives the block pointer at boot (research-driving's open "loader dispatch
  rule"). An (X, X+0x9a8) constant-pair scan over DRIVING.ELF produced only instruction-stream
  noise. **The DRIVING-side result-block writer remains unfound** (it may not exist; see Live
  checks).
- So nobody fills the 9×3 s32 rows at PTP+0x9a8 (0x31ca38) in the shipped game — [L ee_start_05]
  the pointer is 0x31ca38 and the rows read zero. The 110d zeroed-buffer model for the 0x09
  reader stands.
- The row format [M 0x1984b0:50-64]: `[value, count, target]` per row — NOT the task guess's
  "(stat id, value, medal state)". The copy maps row[0]→entry+8 (the stat; row 7's value ×100 —
  the driving par seconds become 1/100 s), row[1]→entry+0 (the count/par), row[2]→entry+4 (the
  target). The medal is never carried per row: P_NFRESULTS 0x208e78:96-146 compares the entry's
  last (+0x34) against the thresholds.
- The results path: `P_NFRESULTS_Handler` 0x208e78:77 `PlrStat_GetScore(0)` (commit mode,
  [M 0x1984b0:301-315]) with `Mission_Status() == 6` [M 0x182c60:85-122]; the same scoring at
  the DRIVING→ACTION hand-off (`Boot_LoadPTPData` 0x1c8560: load PTPDATA 0x31c090/0xa50, set the
  handed-off map hash, `PlrStat_GetScore(0)`, then the front end 0x07000048). The PTP stats copy
  beside it: `PlrStats_GetDrivingData`/`SetDrivingData` 0x199ac8/0x199b18 — per ScoringTable
  entry the rows pointer (+0x18) and the row-0 value (the moments row) at PTP 0x31c1e4/0x31c214.

**The port (task step 2).** `Progress::finish_level(base_map, difficulty)` in
`crates/nf-game/src/progress.rs`: at a level end the Sim produces the rows the game would hand
to the results screen — the game's own formula at the results-path status 6 [M 0x1984b0:316-333]
— as a plain `FinishedLevel` (no UI): the 9 scored rows (count/target/value/ratio/raw), base,
dif, bonus, last (base+bonus), the entry's untouched best, the would-be medal (the P_NFRESULTS
chain), and the `PlrMissionStats` snapshot the rows were produced from. Mirrored on
`Progress::finished`. READ-mode (commit = false) by design: a mid-mission part exit (07000005's
truck ride into 07000006 is a trigger LOAD_LEVEL, not a mission end) must not stamp best/last —
the results page's own commit (already ported in `menu.rs results_enter` as the game's
`GetScore(0)`) stays the only committer. The production hook is one additive block at the end of
`Drones::tick` (drone.rs): when the level-exit latch stands, produce for `mission.base_map` at
`Drones::difficulty`; re-running while the latch stands is inert.

**The tests (task step 3).**

- `crates/nf-view/tests/fix_scores_handoff_live.rs` (NF_ISO): finishes 07000005 through the REAL
  exit path — the truck ride (the gate_guard_truck_live helpers: cold start → porch → ch-0x5C
  plate → hand-off cut-scene → crouched ride → park → EndLevel chain → ch 0x1D → level_exit
  0x07000006) — then asserts the hand-off equals the formula recomputed from the snapshot tally:
  rows 1/2/3/4/5/8 are the task's sister's ZERO rows (the vanilla ride is a no-kill, no-shot
  run), row 6 health / row 7 elapsed score from the snapshot, base = Σ raw, bonus = base × dif,
  last = base + bonus, the medal chain, and the entry's last/best untouched by the read-mode
  production. FAIL-FIRST recorded: with the hook disabled the test dies at the missing rows
  (`finished` is None — the port's NULL-path abort, exactly the task's symptom); restored → ok
  (16.5 s; the run's hand-off: base 100000 = the moments row 50000 + the time row 50000,
  last 100000, medal 1, difficulty 1).
- Unit sisters in `progress.rs`: `finish_level_handoff_no_kills_zero_rows` (a run with no kills
  gives the zero rows; the read-mode no-commit; the idempotent re-run; the unknown-hash None)
  and `finish_level_handoff_kill_run` (the generic row's `target · min(count, value) / count`
  on a kill tally; the health fptoui; the difficulty-3 ×2.0 multiplier).

## How it was verified

Foreground suite chunks on the FINAL tree (merged with origin/master 0bcd8077 → 39924dab; this
sandbox reaps background jobs — the nohup suite.py runner died with an empty suite.out, so the
chunks ran in the foreground, per the earlier sessions' pattern):

- nf-game lib: **177 passed / 0 failed** (the touched crate, includes the 2 new unit tests).
- nf-formats: 77 passed; the 3 `disc_golden` fails are exactly the allowed slim-ISO set.
- nf-audio: green. nf-view lib: 8/8.
- Integration: progress_remainder_live 4/4 + progress_remainder_failfirst 3/3 (including the
  0x09 zeroed-`NewScoresRef` scoring tests), level_reload_live + fix_progress_live 9/9 (the
  level-exit consumption paths with the hook live), levels_sweep_live 3/3 (every level loads and
  runs 600 ticks with the hook live, 185 s), fix_scores_handoff_live 1/1.

## What's left

- The DRIVING-side row writer: still unfound (probably nothing to find — the shipped rows read
  zero). If the main agent's live check below shows a driving section writing non-zero rows at
  PTP+0x9a8, a follow-up task can thread the real values into `Progress::new_scores`; the
  landing spot is ready.
- The results screen itself is 180-fix-menus' (the data this branch produces is its input);
  `menu.rs results_enter` already re-runs the game's scoring on entry.

## Shared files touched (minimal, per the rules)

- `crates/nf-game/src/drone.rs`: ONE additive block at the tick's end (the level-exit hook,
  lines marked `fix-scores-handoff`). No reformatting.
- `docs/WORKLOG.md`: appended entries only (the merge with origin/master kept both sides).

## Live checks for the main agent

PCSX2 (the game, during a driving section — e.g. the 07000004 overflow-tunnel drive):

1. `gdbwatch 0x31ca38 0x6c write --seconds 30` — the 9×3 s32 NewScoresRef rows. Expected on
   retail: ALL ZERO (no ACTION/DRIVING writer exists [M]; the bootup pointer store writes the
   pointer, not the rows). Any non-zero write identifies the DRIVING-side producer this task
   could not find — its address is the follow-up's evidence.
2. `gdbwatch 0x30cc6c 4 read --seconds 30` — the pointer's readers. Expected: only
   `PlrStat_GetScore` 0x1984b0 (at the results/PTP-load moments).

nf-view (render spots; the hand-off is data-only — nothing draws):

3. `--bond` at the 07000005 truck ride's end (the gate_guard_truck spots still apply): the run
   should end in the exit to 07000006 exactly as before the change (the hook is inert off the
   exit latch; levels_sweep_live re-verified every level's 600 ticks).
