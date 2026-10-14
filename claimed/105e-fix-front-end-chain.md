## Your task: `front-end-chain`, campaign: results screen to the next mission

docs/reports/campaign-flow.md blocker 7: after a mission ends the port lands on 07000048 and stops. Port the game's
front-end leg: the mission results/medal screen (`Mission_Update` 0x182c60 case 6 onward, the outro movie, the
results page), the unlock written to the progress layer (`nightfire_status` 0x202360, the codename profile from
103g-fix-save-profile), and the mission select then offering the next mission. Drive it in the campaign runner
from 01's end to 05's start. Tests (NF_ISO): completing 01-04 unlocks 05 in the profile and the mission page lists it;
the results page shows the game's fields (numbers); sister assertions.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report. Note: the main agent's PCSX2 captures have a CRT filter on; use them for presence and placement,
not exact colour or brightness.

Files: `crates/nf-view/src/menu.rs`, `crates/nf-game/src/{mission,progress,profile}.rs`.

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

**Branch:** `s/front-end-chain`.

---
claimed-by: sandbox6 at 2026-10-14T19:40Z
