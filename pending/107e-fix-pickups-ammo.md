## Your task: `pickups-ammo`, weapon and ammo pickups: caps, dropped guns, sounds

Check the pickup layer against the game on every level: the per-ammo-type caps, picking up a weapon you already own
(ammo only), dropped guard guns (their ammo count, despawn, the pickup radius), ammo boxes, the pickup sound and the
on-screen pickup message, the auto-switch rule (`Player_IsBetterWeapon` 0x1a6eb0, already live). Tests (NF_ISO): pick up
a dropped guard gun twice (second adds ammo up to the cap), an ammo box, a new weapon; the numbers and message equal the
game's; sister assertions.

Files: `crates/nf-game/src/{pickup,drone}.rs`, the message in `crates/nf-view/src/hud.rs` (small).

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

**Branch:** `s/pickups-ammo`.
