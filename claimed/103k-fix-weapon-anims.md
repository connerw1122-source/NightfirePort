## Your task: `weapon-anims`, weapons: the view-model animations per weapon

Base: origin/master. If `s/weapon-matrix2` is not on master yet, merge it into your branch first (it carries the
upgraded runner `nf-extract weapon-matrix` and docs/reports/weapon-matrix2.md, which this task builds on).

Check every weapon's first-person animations against the game: draw, holster, idle (and idle fidgets), fire (per mode,
including burst and charge), dry fire, reload (magazine, per-shell start/loop/end, the launcher's), the mode switch
(silencer on/off, scope, grenade attachment), sprint/move bob, and zoom. For each: the anim id the game plays
(weapon data / view script, `Player_Weapon*`), its length and playback rate, the events on it (the shell eject, sound
cues, the muzzle flash frame) and the blend in/out. Table in the report: weapon x anim -> game id/len vs port id/len,
marking every mismatch. Fix the mismatches.

Tests (NF_ISO): for 6 weapons covering each reload kind (P2K, Frinesi shotgun, AT-420, MGL, sniper rifle, Samurai):
the anim sequence and tick lengths for draw -> fire -> reload -> mode switch -> holster equal the game's (numbers);
sister assertions.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report; a test on the drawn set alone is not enough (several fixes passed their tests while the rendered
frame still showed the bug).

Files: `crates/nf-game/src/gun.rs`, the view-script/anim code in `crates/nf-view/src/scene.rs` (small local edits),
a new `crates/nf-view/tests/weapon_anims_live.rs`.

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

**Branch:** `s/weapon-anims`.

---
claimed-by: sandbox3 at 2026-10-14T13:41Z
