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

---
## Report (done, 2026-10-15T15:19Z)

# Report: read-vu1-microcode (110v, G-014)

Branch `s/read-vu1-microcode`, cut from origin/master 404d7dc1, merged with
origin/master (a3759bfe) at the end. No renderer or game-code changes — this
was a read task.

## What's done

1. **`tools/vu1dis.py`** — a VU1 microcode disassembler (python3, stdlib only).
   Opcode tables and the field layout follow PCSX2's public VU disassembler
   (`pcsx2/DebugTools/DisVUmicro.h`, `DisVUops.h`) and its hardware interpreter
   (`pcsx2/VUops.cpp`) for execution semantics (ILW lane mapping, VU1 data
   addressing = quadword-indexed ×16 via `mVUaddrFix`). All cited in the doc.
   The decoder was self-tested against hand-built encodings (ADDx, LQ, SQ, ILW,
   XGKICK, IADDIU, B, CLIP, FCAND/FCGET, JR, BAL, SQD, MOVE, MAX) before use.
   It dumps `.vutext` and the DVP overlays straight out of ACTION.ELF
   (`--elf`) or raw qword files.
2. **The microcode located and framed.** `.vutext` = EE 0x241c80, file offset
   0x13bb80, 0x2880 bytes. First 0x10 bytes = an upload header (DMA tag +
   VIF MPG 0x4a000000), not microcode; the program is `.vutext+0x10` = micro
   qword 0, 692 qwords resident (the ovlytab maps 5 chunks to VU byte offsets
   0x0/0x800/0x1000/0x1800/0x2000). **The six `.DVP.overlay.*` sections are all
   zero bytes** (verified byte-for-byte) — there is no second microprogram on
   the retail disc.
3. **The program read and documented** — `docs/formats/vu1-microcode.md`
   (control flow, data-memory map, per-entry-point behavior, the vertex
   pipeline, the clip path, the kick), with every claim carrying a micro
   address. Headlines:
   - MSCALF 0 → micro 0x000, MSCALF 2 → micro 0x002; both call the same setup
     (load the matrix from data 0-3), set the batch cursor vi02 = 2 and stop
     on E. Entry 0 additionally sets vi14 = 0x400 (the clip-enable flag).
     **Entry 0 = clipped / 2 = unclipped: confirmed.**
   - Per batch, MSCNT resumes a dispatch loop; handlers end at an XGKICK of a
     VU-built output buffer (double-buffered at data 0x25/0x3e), vi02 += 15, E.
   - The transform: matrix (data 0-3) · position, DIV Q = 1/w, output position
     FTOI4 (GS fixed point), **ST out = raw disc ST × Q with output Q = 1.0** —
     the ST·Q mechanism is now proven, and the port's direct-UV reading matches
     what the GS interpolates. **Confirmed.**
   - **No lighting terms anywhere** in the read paths; vertex RGBA passes
     through (pre-lit upstream). **Confirmed** for the read portion.
   - The clip path is a full Sutherland–Hodgman (CLIP + FCGET/FCAND 0x3ffff
     per 3 vertices, trivial-reject and pass-through branches, MR32/DIV-Q/ABS
     edge interpolation with w forced to 0), and the VU builds the output GIF
     tag itself (NLOOP counter vi11 written via ISW).
4. **Confidence updates:** `docs/formats/ps2gfx.md` (header rewritten —
   entries/UV/lighting confirmed, the V4-8 w byte stays medium) and
   `docs/GAPS.md` G-014 (marked done with a precisely-scoped remainder).

## What's left (honest list)

- Handlers at micro 0x2e0-0x50e (~40% of the program) were not fully analyzed:
  the consumption point of the signed V4-8 normal's w byte ({0,−1}) — the
  per-vertex strip-break — sits there, as does the skinning matrix palette
  walk (0x4f4-0x50c, LQ stride 0x40; ties into G-015) and a second matrix
  (data 24-27). The interleaved pos/rgba/nrm record layout (stride 3) and the
  VU's per-vertex output-emission counter are confirmed; only the w byte's
  exact reader is not, so ps2gfx.md keeps "medium" for that one item.
- 14 lower-op words the public tables don't cover (0x810273ff-class, minor
  0x3f/fd 0x1f, LQD-like, in the register save/restore around the skin-dispatch
  `JR vi01` trampoline at 0x24f). PCSX2's interpreter tables lack them too, so
  the path may be dead on hardware; a live check can settle it.
- The full 4-crate suite: the sandbox kills background processes between tool
  calls, so the fast runner (tools/test/suite.py) was restarted three times and
  was running on the final merged tree at report time; no result line yet. The
  task changed no Rust code (only docs/ and tools/), so the code under test is
  byte-identical to origin/master's apart from other workers' merges that
  arrived in origin/master itself.

## Verification

- The disassembler's opcode tables were cross-checked against two independent
  PCSX2 sources (disassembler tables + hardware interpreter) and spot-tested
  against hand-assembled encodings before use.
- Every structural claim in the doc cites a micro address read from the actual
  bytes; the DVP overlay zero-check was done byte-for-byte.
- Shared files touched: `docs/WORKLOG.md` (append-only; merged with a parallel
  worker's entries, both kept) and `docs/GAPS.md`/`docs/formats/ps2gfx.md`
  (the task's named files). No code files touched.

## Live checks for the main agent (PCSX2)

1. VIF1 MSCALF immediate: expect 0 for clipped batches, 2 for unclipped, once
   per object (the per-batch trigger is MSCNT).
2. VU1 data memory after MSCALF: matrix at 0x00-0x0f, selector 0x14.z toggling
   0x25/0x3e across batches, addend 0x16.x = 0x16.
3. VU1 PC trace over one batch: resume at 0x01e/0x03f → JR through the data
   jump table → transform handler → XGKICK at 0x02b/0x087 → E stop.
4. Step the 0x208-0x23e save/restore block if it executes (skinning path);
   log vi14 and vf02-vf05 to identify the 14 unknown encodings.
5. nf-view `--bond` spots: none — no renderer changes, nothing visible asserted.

## ntfy

(sent after this report; see the finish step)
