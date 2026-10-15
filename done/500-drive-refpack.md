## Your task: `drive-refpack`, driving phase 0: the RefPack decompressor and the decompressed census

Read `docs/atlas/driving.md` and `docs/formats/driving-viv.md` (research-driving) first; this is §6 phase 0.
1. Write a RefPack (EA's 0x10FB LZ) decompressor in `crates/nf-formats/src/refpack.rs` from its documented format (cite the
   source you use), with unit tests on hand-built streams covering every opcode class.
2. Decompress every RefPack entry of the 23 driving archives (the DRIVING folder: `NF_DRIVING`, or extract it from the ISO
   with 7-Zip to a folder OUTSIDE the repo) and pin the decompressed sizes and SHA-1s in a golden (numbers only), with a sister.
3. Cross-check against an independent decompressor if one exists for you to run (e.g. a Python one in a venv under
   `work/third_party/`, never committed); on any disagreement, find out why.

Files: `crates/nf-formats/src/refpack.rs` (new), `crates/nf-formats/tests/driving_golden.rs`, `golden/`, docs/formats/driving-viv.md.

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

**Branch:** `s/drive-refpack`.

---
claimed-by: sandbox2 at 2026-10-15T15:51Z

---
## Report (done, 2026-10-15T17:15Z)

