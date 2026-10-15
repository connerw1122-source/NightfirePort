## Your task: `dev-spawn-visible`, a guard spawned from the dev window must show up and fight

The playtest checks rely on the dev window's NPCs page to put a guard in front of the player (the stunner check, the
bullet-hit check). The main agent's dry run on master, `nf-view <iso> 07000005 --dev`, Bond at the spawn
(105, 25, 109), looking at the snow a few metres ahead:
- **"spawn at crosshair"** (`MenuAction::Spawn` → `Dev::spawn_guard`, `crates/nf-view/src/dev.rs`): the log says
  `[dev] guard #16: spawned (normal) at (101.2,24.3,106.4)`, but no guard is visible there or anywhere around Bond.
  (This row used to crash the game: the renderer's skin layout didn't cover the added Character; master now rebuilds
  the renderer when the character count changes — `App::apply_menu_action` in `lib.rs`.)
- **the skin rows** ("skin 05000030 (1/3)", `MenuAction::SpawnNpcSkin` → `panel::spawn_npc`): the pool count dropped to
  0/3, but no guard appeared "in front of Bond" either.

Find out why neither spawn is seen: is the drone placed under the floor or inside the wall (y 24.3 vs Bond's 25),
never `drawn`/`present` for the renderer, culled by the portal walk (no cel link — `build_LinkToRoom` 0x1c9680), or
missing from the frame's palette? Write a headless NF_ISO test through the real dev path
(`dev::apply_menu_action_to_scene` with both actions, then `Sim::step_bond` ticks) that asserts the spawned guard is
on the floor in front of Bond, in a visible cel, posed, and that he notices and shoots at Bond (a normal-mode guard);
render it with `--shot` (the `--frame` path with a scenario, or a test that writes the frame) and describe what shows.
Fix both rows. Use the game's own spawn path where one exists (`DynObj`/`Drone` init the level loader uses).

Files: `crates/nf-view/src/dev.rs`, `crates/nf-view/src/panel.rs`, a new test file.

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

**Branch:** `s/dev-spawn-visible`.

---
claimed-by: sandbox6 at 2026-10-15T06:11Z

---
## Report (done, 2026-10-15T07:46Z)

Closed by the driver: the report is on `s/fix-dev-spawn-visible` (docs/reports/fix-dev-spawn-visible.md); the worker didn't run finish.sh.
