## Your task: `weapon-matrix4b`, weapon matrix: the in-suite tests must fit the time limit

Base: `s/weapon-matrix4` — merge origin/master into it FIRST (and `s/weapon-matrix3` if 103m2 hasn't landed it; keep
matrix3's gadget-cell test). HELD at merge: the full suite passed everything except `weapon_matrix_live`, which TIMES OUT
at the suite's 1200 s per-binary limit (last started: matrix_0700000d). The per-level tests now sweep at the honest
budgets (~60-90 s a story level), so a test per level for the whole list no longer fits. Keep every check and the
harness fixes, but make the suite binary finish well under 1200 s: a small fixed sample in-suite (05, 0c, one late
level, the gadget cells, the upgraded-runner checks), the full 27-level sweep only through `nf-extract weapon-matrix`
or an `#[ignore]`d test with a reason. Report the binary's runtime and the full `tools/test/suite.py` total.
106b-weapon-matrix5 builds on this.

Files: `crates/nf-view/tests/weapon_matrix_live.rs`, `crates/nf-view/src/matrix.rs` only if needed.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/weapon-matrix4b`.

---
claimed-by: sandbox7 at 2026-10-14T20:34Z

---
## Report (done, 2026-10-14T21:27Z)

# weapon-matrix4b — the weapon matrix tests must fit the suite's time limit

Task: `weapon_matrix_live` TIMED OUT at the merge (the per-level tests sweep at the honest
weapon-matrix4 budgets, ~60–90 s a story level; 27 levels do not fit the suite's per-binary
limit). Keep every check and the harness fixes; make the suite binary finish well under the
limit: a small fixed sample in-suite, the full sweep through `nf-extract weapon-matrix` or an
`#[ignore]`d test with a reason.

## What was done

**The merge first** (the task's base order): `s/weapon-matrix4b` = `s/weapon-matrix4` (371daf6d)
+ `origin/master` (1c376216) + `s/weapon-matrix3` (6171f2a8). matrix3 was NOT landed by 103m2
(not an ancestor of master), so it merges here per the task:

- `crates/nf-view/src/matrix.rs` — three conflicts, all resolved to **matrix4's harness evolution**
  (the god-refill/measure-weapons-only runner with the script-derived switch/reload budgets, the
  twins' base-then-mode route, the disable-window ride-out) — matrix3's older cold-reload/cycle
  variants are superseded by matrix4's versions of the same repairs. From matrix3 the merge KEEPS
  the `no_target` cell field and its stunner setter
  (`c.no_target |= sim.stunner_target == Some(false)` — `Player_WeaponInitBullet` 0x1a4bf0:169-200,
  the no-target arm), which the gadget-cell test needs as its escape hatch.
- `crates/nf-view/tests/weapon_matrix_live.rs` — matrix3's `gadget_cells_run_the_cycle_on_07000005`
  came through the merge and stays (the task's "keep matrix3's gadget-cell test").

**The restructuring** (`weapon_matrix_live.rs`, the only production-adjacent file changed):

1. **The in-suite sample** (what the suite now measures): `matrix_07000005`, `matrix_0700000c`,
   `matrix_0700001b` (05 = the give/gadget harness level, 0c = mid-story, 1b = a late story level —
   the sample weapon-matrix3b measured at 270 s on the lighter harness), plus
   `test_weapon_row_runs_the_sequence`, `upgraded_runner_checks_the_new_behaviours_on_07000005` and
   `gadget_cells_run_the_cycle_on_07000005`. Every assertion on the sampled levels is untouched.
2. **The three 07000005 consumers share one row** via a `OnceLock<matrix::LevelRow>` (`row05`):
   a full 05 run costs ~90 s and the level test, the upgraded-runner checks and the gadget cells
   all consume the same measurement — computed once per process (the suite runs
   `--test-threads=1`, the row is read-only afterwards).
3. **The other 24 levels keep their tests, `#[ignore]`d with the reason**: the full 27-level sweep
   does not fit the per-binary cap. They run with
   `NF_MATRIX=all cargo test -p nf-view --test weapon_matrix_live -- --ignored --test-threads=1`
   (or an explicit `NF_MATRIX=07000001,...` list — the selector now defaults to the sample when
   `NF_MATRIX` is unset, `all` clears it), or through `nf-extract weapon-matrix <iso>`
   (nf-extract/src/main.rs:70, unchanged) which prints the same run as a table.
4. `matrix.rs` needed no further change: it compiles as resolved, and the sample's assertions all
   pass against it (below).

## Verification

- **First-hand reproduction of the timeout's cause**: one sweep level costs ~161 s on this
  harness (`NF_MATRIX=07000001 … --ignored matrix_07000001` — 1 passed, 161 s), so 27 levels
  ≈ 4300 s — the held merge's timeout (`last started: matrix_0700000d`, the 13th test) follows.
  weapon-matrix3b measured the same ~100 s/level first-hand on the lighter harness; no re-run of
  the guaranteed-4000 s default was spent on reproducing it.
- **The suite binary now runs in 341 s** (5m41, first-hand on this sandbox):
  `6 passed; 0 failed; 24 ignored` — the sample (05, 0c, 1b), the dev-menu row, the
  upgraded-runner checks and the gadget cells, every check green, no cell panics.
- **The sweep path works**: `NF_MATRIX=07000001 … --ignored matrix_07000001` → 1 passed (161 s);
  `nf-extract weapon-matrix <iso>` is the other documented sweep route.
- **The full `tools/test/suite.py` total** on the final tree (the runner's per-binary limit is
  600 s by default — the matrix binary at 341 s fits it and the task's 1200 s figure):
  **1096 passed, 6 failed, 63 ignored (32 min), ZERO timeouts** — the held `weapon_matrix_live`
  timeout is gone. The 6: the 3 allowed `disc_golden` (slim ISO) and the 3 pre-existing pylon
  failures (`probe_pylon_render`, `fix_0c_pylon_live` ×2 — failing on master before this branch;
  63 ignored = 35 before + the 24 `#[ignore]`d sweep levels + the rest of the pre-existing set).
  The master merge landed during the session (9c55eace, the fix-character-room-light tests/docs —
  no production code) and its test re-ran green on the final tree (3/3).
- The pre-existing `character_lighting_live` and the sampled matrix assertions all pass on the
  merged tree.

## What is left

- The 24 `#[ignore]`d sweep levels are unchanged, unmeasured here (they were last swept by
  weapon-matrix4's report on 18 levels, 0 panics, 24 red cells — docs/reports/weapon-matrix4.md).
  The next full sweep belongs to `nf-extract weapon-matrix` or `-- --ignored` (106b-weapon-matrix5
  builds on this branch).
- The red-cell classification of the sample (05/0c/1b) is unchanged from matrix4's report — this
  task changed no measurement logic, only which levels run in-suite.

## Live checks for the main agent

None new — this task is harness-only. The standing checks from weapon-matrix4's report (the red
cells' live confirmation) still apply; the sampled levels' tables in this run's output are the
same measurements matrix4's report classifies.

## Files

- `crates/nf-view/tests/weapon_matrix_live.rs` — the restructured suite (sample + ignored sweep +
  shared 05 row).
- `crates/nf-view/src/matrix.rs` — the merge resolution only (matrix4's harness + matrix3's
  `no_target`), no new logic.
- `docs/WORKLOG.md`, this report.
