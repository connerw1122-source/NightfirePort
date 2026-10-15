## Your task: `satchel-stick-play`, the satchel bounces instead of sticking in play

The user (playtest 1015-0046, 07000005, `--give 0x3b,0x3a`, mouse + keyboard): "[The satchel] doesn't stick to walls or
to the ground and it bounces." 109 satchel-tripbomb (`docs/reports/fix-satchel-tripbomb.md`) ported the +0x100 stick arm
and its tests pass. The main agent's headless probe through the real input path (`nf_view::load_weapons` with 0x3b,
equip + `switch_to`, 150 idle ticks, pitch −0.3, one trigger tick) shows the satchel released at press+75, flying, and
STUCK on the snow at tick ~105 (`g.stuck == true`, velocity 0, never moving again) — so the plain case works. Find the
cases the user hit that the probe doesn't cover: a throw straight at a nearby WALL (the 05 spawn's low wall at ~1–3
units, and the user's spot `--bond 105.97,24.87,110.12,-4.369,-0.166`), a throw while moving, a throw at a steep or
non-axis-aligned surface, the first-tick contact (spawned already touching/inside a surface), the world-hit test the
stick arm uses (`Bullet_CollisionHandler` 0x12ce80:255-266) against the port's `segment_surface` (a missed hit
→ the bounce arm runs instead). Reproduce a bounce in a test first, then fix it against the decompile.

Files: `crates/nf-game/src/grenade.rs`, `crates/nf-game/src/drone.rs` (the Stick consumer), a new test file.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/satchel-stick-play`.
