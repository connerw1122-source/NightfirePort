## Your task: `trigger-unknowns`, settle U-032 (triggers) from the decompile

`docs/UNKNOWNS.md` U-032 (read the row): in one live snapshot spawner 268 had started while channels 0x22–0x25 and 0x44
were all 0 — 0x22 fits a pulse, but 0x44 never latching although TouchOnce 306 shares the same patch is unexplained.
Read `Trigger_Update` 0x1b1a78, the spawner's start condition and every writer/clearer of those channels in the decompile;
answer the unknown with citations, port any behaviour the port gets wrong, and add NF_ISO tests reproducing the snapshot's
state through the real trigger path. Update U-032 (answered / still open with what's left). Non-visual.

Files: `crates/nf-game/src/triggers.rs`, `docs/UNKNOWNS.md`, a new test file.

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

**Branch:** `s/fix-trigger-unknowns`.

---
claimed-by: sandbox7 at 2026-10-15T09:38Z
