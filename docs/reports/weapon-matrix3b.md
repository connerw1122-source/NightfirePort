# weapon-matrix3b — report

Branch `s/weapon-matrix3b` = `s/weapon-matrix3` (6171f2a8) + origin/master (b9ed8b8a) merged — the
held merge's only blocker was `weapon_matrix_live` exceeding the suite's per-binary wall clock.
All facts tagged [M]/[D]/[L]/[S].

## The symptom, reproduced first-hand

- The held-merge evidence: the full suite passed everything but `weapon_matrix_live`, which TIMED
  OUT at the runner's per-binary limit with `matrix_07000014` last started (the 15th of 27 level
  tests) — the gadget cells and the longer per-cell settles of weapon-matrix3 pushed the 27-level
  default past the cap. My previous session's suite log showed the same shape at a 600 s cap
  (last started: `matrix_07000015`).
- First-hand on this sandbox (pre-fix binary): `NF_MATRIX=07000011,07000012,07000013,07000014,
  07000015,07000016` — **6 mid-story levels did not finish in 590 s** (>98 s/level; the
  extrapolated 27-level default is ~45 min). The timeout is real and reproduced.

## The fix (keeps every check)

`crates/nf-view/tests/weapon_matrix_live.rs` only — `crates/nf-view/src/matrix.rs` needed no
change (the per-cell settles stay exactly as weapon-matrix3 measured them; no measured behaviour
altered):

1. **The suite's sample** (`levels()`): unset `NF_MATRIX` now selects **07000005** (the give/gadget
   harness level), **0700000c** (mid-story) and **0700001b** (the last story level) — the three
   levels the driver named, plus the full per-level check (every cell of the level still runs and
   the no-panic assertion + the mark table stay). The other 24 level tests return before loading.
2. **The full sweep stays reachable in-place**: `NF_MATRIX=all` re-selects every level, an
   explicit `NF_MATRIX=07000001,...` list selects any subset (the pre-3b mechanism, unchanged);
   the canonical full sweep is `nf-extract weapon-matrix <iso>` (crates/nf-extract/src/main.rs:70
   — unchanged, builds green).
3. **One 07000005 run per process** instead of three: the level test, `upgraded_runner_checks…`
   and `gadget_cells_run_the_cycle_on_07000005` each ran a full `matrix::run_level("07000005")`;
   they now share one row through a `OnceLock` (`row05()` — read-only consumers, and the suite
   runs the binary with `--test-threads=1`). The matrix line + mark table still print (once).
4. The gadget-cell test is kept whole (the driver's explicit ask): the 0x4A/0x4E/0x50/0x54 cycle,
   fire-event, sound and away/back assertions and the 0x4C dim-`n` sister all unchanged.

## Verification

- **The binary's runtime on the final tree**: `cargo test … --test weapon_matrix_live
  -- --test-threads=1` → **30/30 ok in 270.32 s** (the cargo wall 4m30 s) — well under both the
  suite's 600 s default and the 1200 s limit the held merge quoted (before: >1200 s with
  `matrix_07000014` last started; 6 levels > 590 s first-hand).
- **The selector still works**: `NF_MATRIX=07000003` → exactly one level runs (119.51 s: the
  level's cells + the shared 05 row for the two always-on check tests).
- **The always-on checks pass**: `test_weapon_row_runs_the_sequence` (the dev row: draw, 3 shot
  ticks, reload), `upgraded_runner_checks_the_new_behaviours_on_07000005` (the real give path, the
  MGL contact blast, the guided camera, the mode twin), `gadget_cells_run_the_cycle_on_07000005`
  (the four carried gadgets' cycle/fire/sound/away-back + the 0x4C dim sister).
- **The full suite** (`tools/test/suite.py`, the final tree, the background run): the total line
  and the FAILED/TIMEOUT lines are recorded at the bottom of this report.

## Shared-file notes

- **None touched by this task.** The diff on `drone.rs`/`gas.rs`/`hud.rs`/`lib.rs`/`scene.rs` in
  this branch comes from the origin/master merge (other workers' landed work), not from
  weapon-matrix3b. `matrix.rs` was not needed. `docs/WORKLOG.md`: the merge conflict resolved
  append-only (both sides' entries kept) + this task's entry.
- 105a-weapon-matrix4 had NOT landed on master when this branch was cut (it lives on
  `origin/s/weapon-matrix4`); nothing to fold in per the task's conditional.

## What's left

- Nothing blocking. The 24 un-sampled levels' no-panic assertion now runs only under
  `NF_MATRIX=all`/the explicit list or via `nf-extract weapon-matrix` — that is the driver's
  intended split (the sweep belongs to the extractor).
- If the runner's cap is ever raised past ~45 min, `NF_MATRIX=all cargo test --test
  weapon_matrix_live -- --test-threads=1` runs the full 27-level set in-process again.

## Live checks for the main agent

- No visual or sim behaviour changed — nothing to render.
- On a machine with the time for it: `nf-extract weapon-matrix <iso>` should print the full
  27-level table (the weapon-matrix3 report's sweep table shape: B/D/F/M/R/W marks) — spot-check
  that 07000014/15/16 rows still resolve their cells (they are the levels the suite no longer
  exercises).
- `NF_MATRIX=all cargo test -p nf-view --test weapon_matrix_live -- --test-threads=1` (~45 min)
  must pass 30/30 on a machine with no wall-clock pressure — that is the full-check escape hatch.

## The full-suite total (the final tree, tools/test/suite.py)

**1082 passed, 7 failed, 35 ignored — 38 min, 264 binaries, zero TIMEOUT/CRASH lines.**
`weapon_matrix_live` ran INSIDE the suite at **270 s / 30 tests 0 failed** (the held merge's
blocker — it previously TIMED OUT with `matrix_07000014` last started — is gone).

The 7 failed, attributed:
- 3 × `disc_golden` (`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_manifest`,
  `directory_tree_matches_the_golden_manifest`) — the slim ISO's allowed trio (the driver's rule).
- `fix_0c_pylon_live` × 2 + `probe_pylon_render` — `wgpu adapter: NotFound`: this sandbox has no
  GPU adapter; the same panic hits any render test here (documented sandbox artifact from earlier
  sessions).
- `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` — a guard-drone nav
  stall ("the drone stopped short of the goal … after +1303 ticks") that is **pre-existing on
  origin/master, not this branch**: this branch's crate sources are byte-identical to master's —
  the merge-base diff (`8295049..origin/s/weapon-matrix3`) touches only `matrix.rs` +
  `probe_gadget.rs` + `weapon_matrix_live.rs`, and my merge's only conflict was
  `docs/WORKLOG.md` (no source file conflicted, so master's sim code carried over verbatim). A
  live check for the main agent: run that walk on a master tree — if it stalls there too, the
  regression came in with a master-side merge (fix-06-driver's guard-shot changes are the recent
  neighbour); if it walks there, the sandbox's slower run is the variable to chase.
