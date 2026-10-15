## Your task: `hit-locations`, guard hit locations: head, body, limbs

Check the game's hit-location damage on guards: the hitbox set per skeleton, the per-zone damage multiplier (head
shots), armour/helmets, the reaction anim per zone (head snap, leg stagger, arm drop of the gun), the death anim choice
by zone and direction, and blood/impact effects per zone (`NDrone2_DealWithObjHit` 0x13fff0 case 5,
`DroneFunc_SendHurtMessage`, the hitbox data). Port what differs. Tests (NF_ISO): a P2K head shot vs body shot on a
05 guard deals the game's damage and plays the game's reaction/death anim ids (numbers); sister assertions.

Files: `crates/nf-game/src/drone.rs`, `crates/nf-game/src/damage.rs` if it exists.

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

**Branch:** `s/hit-locations`.

---
claimed-by: sandbox5 at 2026-10-14T21:42Z

---
## Report (done, 2026-10-15T01:17Z)

