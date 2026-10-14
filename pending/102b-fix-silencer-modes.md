## Your task: `silencer-modes`, the silencer doesn't stay; infinite ammo and fire rate

The user's playtest (results 1013-1825, 07000006 and 07000005): "I tried adding a silencer. The animation played, but
then the silencer disappeared, but it still shot silenced bullets"; "I had equipped a silencer, swap out, swap back, it
didn't stay"; with the dev panel's infinite ammo "the reserves filled, but the magazine didn't stay full"; "semi-auto
doesn't shoot fast enough on the SSG" (the sniper rifle). Find in the game: the P2K silencer pair (0x02 'Semi' <-> 0x03
'Silenced', the twin-variant switch in `gun.rs`) and how the view model shows the silencer (a separate mesh or the 0x03
record's own model: the port shows the silencer only during the attach animation); what the game keeps when switching
away and back (which twin is remembered: `BLData+0x930/0x932`, the owned bytes); and the sniper's refire interval
(weapon_data `+0x??` refire, `Player_WeaponFiring` 0x1a5460). Fix all three; make dev infinite ammo keep the CLIP full
too (dev only). Tests: attach the silencer, 60 ticks later the view model is the silenced mesh; switch to another weapon
and back: still silenced; the sniper's shot interval equals the table's; sister assertions.

Files: `crates/nf-game/src/gun.rs`, `crates/nf-view/src/{scene,dev}.rs` as needed.

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

**Branch:** `s/silencer-modes`.
