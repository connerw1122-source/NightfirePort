## Your task: `grenade-throw-anim`, the grenade leaves the hand while the pin is still being pulled

The user has now reported this FOUR times on 07000015 (`nf-view <iso> 07000015 --dev --give 0x34,0x36`, throw a
frag): "the grenade still gets released while I'm still pulling the pin". 109e (`fix-grenade-release`, merged; read
`docs/reports/fix-grenade-release.md`) audited the SPAWN timing and changed nothing. The spawn comes at press + 25
ticks (script frame 26 = weapon_data +0x44), which matches the game's code. So the bug is in what the player SEES,
not when the round spawns.

The real game, measured in PCSX2 (`docs/live/2026-10-14-weapon-measurements-pcsx2.md`, "The frag grenade 0x34",
frames 70 ms apart from the press):
- 01–02: the off hand pulls the pin;
- 03: the grenade raised;
- 04–06: the throw, the hands leave the screen;
- 07: the grenade visible in flight;
- the grenade leaves the hand about 0.3 s after the press.

The port's spawn is at about 0.42 s, yet the player still sees the pin pull when the grenade appears, so the
view-model throw animation runs too slow, or plays the wrong sequence, or starts late against the fire script. Check:
- which view sequence the fire script 0x0600049d drives (`view_frame`, seq 0x04000414, 70 frames) and at what rate the
  view model advances per tick in the LIVE window (the windowed path: `anim_time`, `ticks_for_frame`, the view-model
  palette, `view_time`), against the game's `AnimScriptTick` and the sequence's own frame rate (a 30 fps sequence
  played at 1 frame per tick would be 2× slow, or the reverse);
- the press-to-first-frame delay (the draw, the idle, the queue);
- the frame the hand opens in the sequence against the spawn tick.

Dump the view model's frame number per tick from press to spawn through the real input path, and compare it with the
PCSX2 frames above. Fix the mismatch at its cause, citing the decompile. A test (NF_ISO) pins the view-model frame at
the spawn tick to the hand-open frame.

Files: `crates/nf-view/src/scene.rs` (the view-model frame), `crates/nf-game/src/gun.rs`, a new test file.

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

**Branch:** `s/grenade-throw-anim`.
