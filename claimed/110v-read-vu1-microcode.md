## Your task: `read-vu1-microcode`, read the VU1 microcode the renderer's batches run on (G-014)

G-014 (docs/GAPS.md; docs/formats/ps2gfx.md; docs/models/world.md §8): the port's batch semantics (signed V4-8 = normal
+ no-kick flag, UV use, entry points 0 = clipped / 2 = unclipped, lighting) are measured from the data but the
microcode itself (ACTION.ELF `.vutext` 0x241c80, 0x2880 bytes, plus the DVP overlays) has never been read.

Do:
1. Disassemble the VU1 microcode (write a small VU disassembler in tools/ if none exists: the VU upper/lower
   instruction pairs are documented in the public VU manual. Cite the source you use, facts only).
2. Map the entry points the packets call (0 and 2) and follow what each does with the unpacked qwords: the vertex
   transform, the normal use, the ST/Q handling, the clip test, the lighting terms, the kick.
3. Write docs/formats/vu1-microcode.md: per entry point, the input qword layout and the output to the GIF, with the
   addresses. Then mark which of the port's batch assumptions (ps2gfx.md's "medium" items) are confirmed, refuted or
   still open.
4. No renderer changes in this task. If the microcode contradicts the port, list it as a finding with the evidence.
   The main agent owns the render side.

Files: tools/ (the disassembler), docs/formats/vu1-microcode.md, docs/formats/ps2gfx.md (confidence updates),
docs/GAPS.md (G-014).

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


**Branch:** `s/read-vu1-microcode`.

---
claimed-by: sandbox1 at 2026-10-15T13:52Z
