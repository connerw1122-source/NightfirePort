## Your task: `wallhug-prompt`, the wall-shimmy prompt: range and third person; the claw's impact sound

User (05 trench ledge): "near the wall edging area I kept seeing a prompt and when I entered a prompt it was in first
person. should be 3rd person." and "That wall walking ledge area, its interaction prompt area is a bit too large. Like
I'm walking away from it and I still see it." Port the game's wall-hug (creep wall 0x3F) prompt volume (its exact
size/test) and the third-person camera during the shimmy (`Player_PositionCamera` for the creep substate). Also the
Q-Claw: "hook latched, pulled me in, but I also heard a bullet impact" - the claw's bolt must not play the bullet
impact sound on the hook.
This comes from the user's playtest 1014-1725 of the real-input build (`nf-view <iso> <level> --dev`, mouse +
keyboard). The user was very disappointed: several merged fixes passed their tests yet were wrong or broken in play.
Your test MUST drive the real input path (`Sim::step_bond` with the `Input` the window builds - lib.rs maps the mouse
and keys), not internal setters, and must fail first on master. The main agent will compare against PCSX2 after you
finish; put the exact steps to reproduce in the report.

Files: `crates/nf-game/src/player.rs`, `crates/nf-view/src/{scene,lib}.rs`.

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

**Branch:** `s/wallhug-prompt`.

---
claimed-by: sandbox1 at 2026-10-15T04:01Z

---
## Report (done, 2026-10-15T05:27Z)

Closed by the driver: the report is on `s/fix-wallhug-prompt` (docs/reports/fix-wallhug-prompt.md); the worker didn't run finish.sh.
