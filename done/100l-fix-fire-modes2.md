## Your task: `fire-modes2`, land the fire modes on master's explosion path (no double blasts)

100f-fix-weapon-fire-modes (`origin/s/fix-weapon-fire-modes`) built the guided-missile camera (`guided_cam`,
`steer_missile`, the substate-10 camera ride), `docs/systems/weapons.md` and the mode tests. It can't merge as it is:
100c-fix-weapon-effects landed on master first with its OWN impact explode, so the branch now duplicates it twice:
1. the bullet world-hit explode: master queues `pending_blasts` and drains them through `spawn_blast` AFTER the
   per-tick `explosions.clear()` (an inline blast was wiped before the view saw it: weapon-effects found this by probe);
   the branch adds `blast_at` called inline at the same contact -> a rocket would explode TWICE (and the inline one is
   wiped anyway);
2. the grenade contact explode: master's `GrenadeEvent::Bounce` arm reads `+0x70 & 4` and blasts at the contact; the
   branch adds a grenade `explode_on_impact` flag for the same thing -> the MGL 'Grenade' twin would blast twice.
Take over your branch: merge `origin/master`, DELETE `blast_at` and `explode_on_impact` and use master's
`pending_blasts`/`spawn_blast` and Bounce-arm path for every impact blast (fold any game detail yours had that master
lacks into master's path, with its citation). Keep everything else: the guided camera and steering, the HUD overlay
(the red ring/brackets/scanlines and the end static per docs/live/2026-10-13-weapon-modes-pcsx2.md), the mode table,
the B-key mapping to the game's mode action, weapons.md. Tests: one rocket hit -> exactly ONE explosion entry and ONE
damage application; one MGL 'Grenade' contact -> exactly one; plus your mode/guided tests. Full suite on the final tree
(merged with master), 0 failures.

Files: `crates/nf-game/src/{drone,gun,grenade}.rs`, `crates/nf-view/src/{lib,scene,hud}.rs`, tests.

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

**Branch:** `s/fire-modes2`.

---
claimed-by: sandbox5 at 2026-10-14T06:24Z

---
## Report (done, 2026-10-14T07:28Z)

Closed by the driver: the report is on `s/fix-fire-modes2` (docs/reports/fix-fire-modes2.md); the worker didn't run finish.sh.
