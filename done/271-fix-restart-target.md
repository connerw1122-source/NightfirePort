## Your task: `restart-target`, where Restart and a mission fail send Bond

The user, failing on 07000007: "rather than go to the start of the level it goes to the very beginning map, 05". The port
restarts at the mission's base map (`MissionData`: 05–08 are one mission, Restart → `ResetMap_LevelToLoad(BaseMap, 0, 1)`).
The user is sure the real game restarts at the start of the CURRENT level. One of us is wrong.

1. Trace both paths in the decompile: the fail screen's Restart (`P_ENDMISSION_Handler` 0x216618, `C_GCPAUSE_Handler`
   0x204408), `Mission_Update` 0x182c60's fail branch, and what `ResetMap_LevelToLoad`'s arguments do with the RamSave
   (`Player_RamLoad`/the carry-over between levels). Is there a per-level checkpoint (a RamSave taken on level entry)
   that a "restart" reloads instead of BaseMap?
2. Write the answer with [M] evidence in `docs/systems/mission.md`, and make the port do what the game does, with a test
   on 07000007: fail → restart → the level and Bond's state the game would give.
3. Request a PCSX2 live check from the main agent that settles it (fail on 07, pick Restart, which map loads).

Files: `crates/nf-game/src/mission.rs`, `crates/nf-view/src/menu.rs` (the results/pause actions), `docs/systems/mission.md`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing** (start it early, in the background, so it finishes inside your session):
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count. If your change adds `Rand_*` draws, seed-pinned
  tests may move: re-pick them by the WORKLOG 0072 procedure (`NF_TEST_SEED` search against the same live expectations).
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/restart-target`.

---
claimed-by: sandbox4 at 2026-10-13T16:24Z

---
## Report (done, 2026-10-13T17:23Z)

Closed by the driver: the report is on `s/fix-restart-target` (docs/reports/fix-restart-target.md); the worker didn't run finish.sh.
