## Your task: `fix-hit-locations2`, hit locations: the branch fails its own tests on master

Base: `s/fix-hit-locations` — merge origin/master FIRST. HELD: on current master this branch alone fails two of its own
tests (`fix_hit_locations_live::p2k_head_shot_deals_the_games_damage_and_plays_the_shot_death` "an aimed head kill plays
the 0x42 shot-death, not 0x40 (state 0x40)"; `party_guest_head_shot_deals_two_torso_without_the_silent_kill` panics at
:300 "07000007 has present mode-0xD guests"), and combined with fix-flesh-impacts it breaks
`fix_flesh_impacts_live::a_p2k_body_shot_spawns_the_blood_puff_at_the_hit_point` ("scale_rate = (6-3)/60, got 6").
Master since your base has fix-guard-weapons3, fix-melee, fix-pickups-ammo (bullet.rs damage/difficulty), the
satchel/tripbomb, Bond damage and the 06 driver. Find what moved (the death-state choice by hit zone vs master's
damage path, the 07 guest modes after master's AI changes, the effect your branch spawns over the flesh puff) and fix it
from the game's code. Keep fix_flesh_impacts_live green with your branch. Run the full `tools/test/suite.py`.

Files: `crates/nf-game/src/drone.rs`.

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

**Branch:** `s/fix-hit-locations2`.

---
claimed-by: sandbox1 at 2026-10-15T02:14Z
