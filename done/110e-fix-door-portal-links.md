## Your task: `door-portal-links`, doors open and close their portals (block 0x21)

GAPS G-035 (read the row): doors are ported, but `Door_Link2Portal` (map block 0x21) isn't — a door's portal in the cel
graph never closes, so the portal walk treats every doorway as open (rooms behind closed doors are walked and drawn; the
game skips them while the door is shut). Decode block 0x21 (`docs/formats/` + the decompile of `Door_Link2Portal` and its
callers, the door state → portal flag), port it into the door/cel code, and make the portal walk honour it. NF_ISO tests:
on a level with a door between two rooms, the room behind the shut door is not in the visible-cel set from the other
side, and it is once the door opens (a sister assertion with the door open). Non-visual: the visible-cel set is the proof.

Files: `crates/nf-game/src/door.rs`, the cel/portal walk in `crates/nf-view/src/scene.rs` (additive), `crates/nf-formats`
(the block parse), a new test file.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/fix-door-portal-links`.

---
claimed-by: sandbox4 at 2026-10-15T09:32Z

---
## Report (done, 2026-10-15T11:11Z)

