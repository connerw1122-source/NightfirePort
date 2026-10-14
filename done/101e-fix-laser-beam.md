## Your task: `laser-beam`, the Q-Laser and stunner fire bullets: no beam, the sound sticks

The user's playtest (results 1013-1758, 07000008 and 0700000c): holding fire with the laser watch "sounds like a rapid
fire of bullet impacts" and shows no laser beam; same for the stunner; "the sound keeps playing for both" after release;
on the 0c display vent "the laser isn't accurate". The hold-to-use work (fix-gadget-hold) made the laser burn on hold,
but each tick it apparently runs the bullet path (impact decal + impact sound) instead of the beam.

Find the game's laser/stunner fire: the beam draw (a line/sprite from the watch to the hit point), its loop sound
started on press and stopped on release (the sound handle), the burn on locks without bullet impacts, and the stunner's
own effect. Fix: no bullet impacts or impact sounds from these gadgets; the beam drawn each tick while held; the loop
sound stops on release, on weapon switch and on pause; the aim goes through the crosshair (accuracy). Tests: holding
fire for 60 ticks spawns 0 bullet decals and 0 impact sounds, keeps one loop voice, and the voice is gone 1 tick after
release; the beam's end point equals the crosshair ray's hit; sister assertions. Check the gadget ids too (the port's
`--give 0x4e,0x4a`).

Files: `crates/nf-game/src/{gadget,gun}.rs`, `crates/nf-view/src/scene.rs` (the beam draw).

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

**Branch:** `s/laser-beam`.

---
claimed-by: sandbox2 at 2026-10-14T01:48Z

---
## Report (done, 2026-10-14T03:05Z)

