## Your task: `objective-messages`, two objective messages, stale objectives, the banner box

The user's playtest (results 1013-1758): "I get two objective completion messages" (their screenshot on 07000007 at
`--bond 91.91,43.85,-32.12`, the library table: the banner shows OBJECTIVE COMPLETE twice, stacked, while picking up
armour). Also: headless `--shot` renders on 07000007 and 07000008 at the level start show the banner
"NEW OBJECTIVE / Breach the castle walls", 07000006's objective, before 07's own "Rendezvous with undercover agents..."
(`nf-view <iso> 07000007 --frame 200 --shot a.png`). And "the mission label is overlapping the grey box that's overlaid
there" (the banner text versus its box; check both one- and two-line banners and long objective texts at 1280x720 and
4:3).

Find the game's rules: when an objective-complete message is queued (`Mission_*` / the HUD message queue: one per
objective, de-duplicated?), which objectives a level announces at its start (only the current level's, after the warm
carry from 05/06), and the banner box size/text placement (the HUD element's own rect). Fix all three. Tests: completing
one objective queues exactly one message; loading 07000007 cold or warm announces only 07's objective; the banner text
rect lies inside its box for the longest objective string in every level; sister assertions.

Files: `crates/nf-game/src/mission.rs` (or `mission/`), `crates/nf-view/src/hud.rs`.

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

**Branch:** `s/objective-messages`.
