## Your task: `weapon-fx-levels`, weapons: sounds and particles on every level

Base: origin/master. If `s/weapon-matrix2` is not on master yet, merge it into your branch first (it carries the
upgraded runner `nf-extract weapon-matrix` and docs/reports/weapon-matrix2.md, which this task builds on).

The user: weapons must work "no matter what level, with sounds and proper animations and particles". 100a made the
fp models load on every level (the give path's asset merge); do the same audit for the EFFECTS. For each of the 27
levels x each weapon the level can hold (the carried set, every pickup, the give list): the fire sound, the reload /
draw / mode-switch sounds, the muzzle flash, the shell eject, the tracer, the impact puff/spark/decal per surface, the
projectile's model and trail (rockets, grenades, darts, crossbow bolts), the explosion. Record which resolve (asset
found and played/spawned) vs missing, as a table in the report (numbers and ids, no game bytes). Where an effect is
missing because its bank/texture lives in another level's archive, load it the way the game does (find how the game
makes them available: the global/shared banks, the weapon data's sound and effect ids) or through the same merge 100a
uses. Fix every missing cell or give the game evidence for why the game also lacks it. Remember the watch gadgets'
laser and stunner have their own beam/impact effects (rapid impact puffs, not bullets).

Tests (NF_ISO): the table's totals (no missing cells left without a reason), plus 3 named spot checks on 3 different
levels (a rifle, a launcher, a gadget) that the sound id plays and the particle emitter spawns on fire and on impact;
sister assertions. NF_SILENT must stay respected (no audio device opened in tests).
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report; a test on the drawn set alone is not enough (several fixes passed their tests while the rendered
frame still showed the bug).

Files: `crates/nf-view/src/{effects,scene}.rs` or wherever the weapon effects resolve, `crates/nf-audio` only if a
bank-loading hook is needed, a new `crates/nf-view/tests/weapon_fx_levels_live.rs`.

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

**Branch:** `s/weapon-fx-levels`.

---
claimed-by: sandbox6 at 2026-10-14T13:24Z

---
## Report (done, 2026-10-14T16:37Z)

