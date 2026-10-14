## Your task: `06-alarm`, 07000006: the searchlight alarm never sounds

The user, playtest 1013-1825 (06, searchlights): the guards get alerted and start shooting when Bond walks into a
beam, "but there's no alarm that goes off". 045l-fix-searchlight-alarm booted the light script; the alert works, the
ALARM (the siren sound and whatever else the game does: the alarm music, flashing lights, more guards) does not. Find
the game's alarm path from the searchlight spot (`Searchlight_*`, the alarm channel, `Alarm_*`/the siren emitter, the
music event 2 'combat/alarm' the wallhug freeze reads) and port what's missing. Tests (NF_ISO): Bond stepping into a
06 beam starts the alarm sound (the sound id plays) and whatever channel/state the game sets, within the game's tick
count; sister: outside the beam nothing starts. NF_SILENT stays respected.

Files: `crates/nf-game/src/{searchlight,drone,sound}.rs` as needed.

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

**Branch:** `s/06-alarm`.
