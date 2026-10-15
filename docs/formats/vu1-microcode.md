# VU1 microcode (ACTION.ELF `.vutext`) — the renderer's batch program

Status: read 2026-10-15 (task 110v, G-014). The disassembler is `tools/vu1dis.py`
(python3, no deps). Everything below is [M] code — disassembly of the bytes in
`work/elf/ACTION.ELF` — unless tagged otherwise. Addresses are VU1 micro-program
qword indexes (`micro 0x000` = program start) unless a `data 0x..` prefix marks
VU1 data memory. File facts: [D] disc.

## Sources

- Opcode tables and field layout: PCSX2's public VU disassembler
  (`pcsx2/DebugTools/DisVUmicro.h`, `DisVUops.h`, fetched from
  github.com/PCSX2/pcsx2, GPL-3.0) and its hardware interpreter
  (`pcsx2/VUops.cpp`, v1.6.0 tag) for execution semantics. Both derive from
  Sony's EE/VU manuals; the field layout used: upper op = I/E/M/D/T flag bits
  31..27, dest xyzw bits 24..21, ft 20..16, fs 15..11, fd 10..6, op 5..0;
  lower op = major 31..25, it 20..16, is 15..11, id/imm5 10..6, minor 5..0,
  special group major 0x40 with minor 0x3c..0x3f sub-indexed by fd.
- VU1 data-memory addressing (LQ/SQ/LQI/LQD/SQI/SQD quadword-indexed, addr
  & 0x3ff × 16 bytes) and ILW/ISW (×16 bytes; dest lane x/y/z/w = the u16 at
  byte 0/2/4/6 of the quadword): PCSX2 `mVUaddrFix` (x86/microVU_Misc.inl) and
  `_vuILW`/`_vuLQD` (VUops.cpp).
- MSCALF/MSCNT/E-bit resume protocol: standard VIF1 command set (MSCALF 0x15,
  MSCNT 0x17; the E bit stops the microprogram, MSCNT resumes at the following
  instruction).

## Where the program lives

- `.vutext`: EE addr 0x241c80, file offset 0x13bb80, 0x2880 bytes [D section
  table; readelf]. The first 0x10 bytes are an upload header, not microcode:
  a DMA tag word 0x7000025c and the VIF code 0x4a000000 = MPG with dest
  address 0. The microprogram is `.vutext+0x10` onward = 1294 file qwords.
- The linker's overlay table `.DVP.ovlytab` (file 0x209d90, 6 entries of
  (id, EE addr, VU byte dest)) maps the program in 6 chunks to VU byte
  offsets 0x0, 0x800, 0x1000, 0x1800, 0x2000, 0x0 — i.e. micro qwords
  0-127, 128-255, 256-383, 384-511, 512-691 (692 qwords = the whole program,
  micro 0x000-0x2b3), plus a 77-qword chunk mapped over micro 0x000-0x04c.
- The six `.DVP.overlay.*` sections (file 0x207588, 0x207d88, 0x208588,
  0x208d88, 0x209588, 0x209b28; sizes 0x800×4, 0x5a0, 0x268) are **all zero
  bytes** [D; verified byte-for-byte]. There is no second microprogram on the
  disc image: everything the game can run is in `.vutext`. The 77-qword
  overlay slot would zero micro 0x000-0x04c if MPG'd; no code for it exists,
  so the two entry trampolines below are the only ones the retail disc ships.

## VU memory map

Micro memory (instructions, 692 qwords resident): 0x000-0x2b3.

Data memory (16 KB, 1024 qwords, separate from micro memory):

- `data 0x00-0x0f`: the per-object parameter block the setup loads into
  vf01-vf04 (matrix rows 0-3), vf13-vf16, vf06, vf05, vf08, vf07, vf12
  (micro 0x251-0x25d). Rows 0-3 are the transform matrix used on positions.
- `data 0x14` (20): z lane = the output-buffer selector, toggled between
  0x25 and 0x3e by micro 0x1c5-0x1d0 (double-buffered GIF output).
- `data 0x16` (22): x lane = the jump-table addend (read at micro 0x022,
  0x067); w lane at `data (vi02+20)` = the batch control pointer chain.
- `data 0x18-0x1b` (24-27): a second copy of 4 matrix rows, loaded at
  micro 0x1dd-0x1e0.
- Batch buffers (the unpack targets, triple-buffered B ∈ {0x2e1, 0x1d8,
  0x0cf} per ps2gfx.md): per-vertex records of 3 qwords starting at B+1 —
  +0 position.xyz (V3-32), +1 RGBA (V4-8u), +2 normal+flag (V4-8s, the
  sign-extended w ∈ {0,−1}) — and UV pairs at B+0xc7+i (one qword each).
  The microcode walks exactly this layout (stride 3: micro 0x26f/0x27e
  `IADDIU vi01, vi01, 6` = two vertices; UV via `vi03 = vi01 + 0xc7` at
  micro 0x264/0x2a9/0x347 with LQI).
- Output staging areas at 0x57 and 0x72 (register save/restore + the clip
  emitters' scratch).

## Control flow

```
MSCALF 0 (clipped)     → micro 0x000: NOP | B 0x034
MSCALF 2 (unclipped)   → micro 0x002: NOP | B 0x018

0x034: vi14 = 0x400            ← the clip-enable flag (entry 0 only)
0x035: BAL 0x24f, vi15         ← 0x24f is "JR vi01": jump to the setup
0x036: vi02 = 2                ← first batch cursor
0x037: E                       ← stop; MSCNT resumes at 0x039

0x018: BAL 0x24f, vi15         ← entry 2 path: same call, no vi14 flag
0x019: vi02 = 2
0x01a: E                       ← MSCNT resumes at 0x01c/0x01e
```

The common setup (micro 0x251-0x25d) loads the parameter block (data 0x00-0x0f)
into vf01-vf04/vf13-vf16/vf05-vf08/vf12 and returns (`JR vi15`, 0x25e).

Per batch the VIF stream ends in MSCNT, which resumes the microprogram in the
dispatch loop:

```
0x01e/0x03f: vi01 = data16[(vi02+20)+6]    ← halfword: this batch's control ptr
0x025/0x06a: JR (data16[vi01+6] + data16[22])   ← jump table in the data
0x027/0x06c: (return label in vi15) reload vi01, XGKICK vi01   ← kick output
0x02c/0x08b: vi02 += 15; E                  ← stop; next MSCNT = next batch
```

`vi02` starts at 2 and advances 15 per batch (qwords); the per-batch control
qwords sit at data (vi02+20). The XGKICK fires once per batch with the output
buffer address held in vi01 — the VU-built GIF packet — after which the
program stops on E until the next MSCNT.

Dispatch targets seen in the jump-table range: 0x25e (return), 0x2a3, 0x2e0,
0x340 (micro 0x004-0x017 hold the matching constants; the 0x400000xx words
there double as float constants and branch-shaped data).

## Entry point 0 (clipped) vs entry point 2 (unclipped)

Both run the identical setup and dispatch; the only static difference is
`vi14 = 0x400` (micro 0x034) on entry 0. The clip machinery (below) is real
code reached from the dispatch loop; entry 0's flag marks the clipped mode.
The ps2gfx.md packet-level observation (MSCALF 0 = clipped, 2 = unclipped) is
**confirmed** at the microcode level [M].

## The vertex pipeline (verified paths)

Unclipped transform handler (micro 0x2a5-0x2a4 family and 0x260-0x289):

```
vf20 = vf01·p.x + vf02·p.y + vf03·p.z + vf04      ← matrix rows from data 0..3
DIV Q, 1.0, vf20.w                                 ← Q = 1/w
position out = FTOI4(vf20 · Q)                     ← GS XYZ2 fixed point (×16)
ST out = LQIxy(B+0xc7+i) · Q ; out.zw = (1.0, ...) ← ST scaled by Q, output Q=1
```

- The ST·Q handling: the disc stores raw ST; the VU multiplies ST by Q=1/w and
  writes the output vertex with **Q = 1.0** (vf17.zw = (1,1) built at 0x2a5
  `MAXw.zw vf17, vf00, vf00.w`; applied by `MOVEzw vf24, vf17` at 0x274/0x27b).
  So the GS receives texture coordinates already divided by w. The port's
  direct-UV reading therefore produces the same values the GS interpolates [M].
- Clip test: three vertices at a time (micro 0x049-0x063): transform with
  matrix B (vf13-16 = data 4..7), `CLIP` each, `FCGET` per vertex, `FCAND
  0x3ffff` (18 bits = 3×6 flags); `vi07` ANDs all vertices' flags (nonzero ⇒
  the whole batch is outside one plane ⇒ trivial reject at 0x0a2: output
  count 0, `XGKICK`, `vi02 += 15`, E); `vi06` ORs them (zero ⇒ nothing
  clipped ⇒ pass-through path at 0x06c-0x089).
- Clip-edge interpolation (micro 0x18b-0x1a6, 0x1a8-0x1c4): the two edge
  vertices are rolled with `MR32` (vi12 times), the plane distance ratio via
  `DIV Q` + `ABS`, the new vertex = lerp with **w forced to 0**
  (`MFIRw vf27, vi00` at 0x1a7/0x1c4) — Sutherland–Hodgman on the GS clip
  volume, emitting 1 or 2 new triangles per crossing edge (the 0x0d6-0x148
  blocks carry the five plane masks 0x800/0x400/0x200/0x100/0x80 and the
  output-primitive counter vi11, which is written into the output GIF tag by
  `ISW.w vi11, 2(vi07)` at 0x099 — the output NLOOP is built by the VU).
- Output kick: one `XGKICK` per batch (0x02b/0x087/0x0a9/0x1ff), from the
  double-buffered output at data 0x25/0x3e (selector toggled at 0x1c5-0x1d0).
  The output tag starts with the word 0x8000 (EOP, NLOOP placeholder — ISW at
  0x1d5/0x1dc/0x0a5) and the VU patches the real NLOOP from vi11.

## Findings vs the port's batch assumptions (ps2gfx.md "medium" items)

| Assumption | Verdict |
| --- | --- |
| Entry points 0 = clipped / 2 = unclipped | **Confirmed** [M] (trampolines at micro 0x000/0x002; vi14 = 0x400 on entry 0; clip machinery present). |
| UV: disc ST used directly, no ST·Q tail on the disc | **Confirmed** — and the mechanism is now known: the VU computes ST·(1/w) and outputs Q = 1.0, so the GS interpolates the raw ST values [M]. |
| Lighting: none in the batch path | **Confirmed as far as the code was read** — no light-vector dot products anywhere in the transform/clip/emission paths; vertex RGBA passes through. The EE-side lighting (if any) happens before the unpack. |
| Signed V4-8 = normal + no-kick flag | **Partially confirmed.** The interleaved per-vertex record (pos/rgba/nrm at stride 3, nrm.w sign-extended to {0,−1}) is exactly what the microcode walks. The consumption point of the w flag (the per-vertex strip-break) is in the not-yet-read handler tails (0x2e0-0x4f4); the output-NLOOP counter vi11 proves the VU decides per-vertex what enters the output packet. Keep "medium" until the tail is read. |

No renderer changes were made in this task (per the task brief); nothing above
contradicts the port's implemented behavior.

## Not yet read / open

- Handlers 0x2e0-0x33f and 0x340-0x50e (≈40% of the program): the strip-break
  (w-flag) consumption, the skinning matrix palette walk visible at 0x4f4-0x50c
  (LQ stride 0x40 qwords — the CacheSkin/SkinIt upload path, G-015), and the
  second-matrix use (data 24-27 loaded at 0x1dd).
- 14 lower-op words the public tables do not cover: 0x810273ff-class
  (minor 0x3f, fd 0x1f, it/is pattern = LQD-like register restore paired with
  the SQD save block at 0x208-0x216; also 0x810a73fe-class at 0x230-0x23d).
  They sit in the register save/restore around the `JR vi01` trampoline at
  0x24f (the skin-dispatch tail-call: `BAL 0x206, vi15` at 0x09a = jump to
  [vi01]). PCSX2's interpreter tables have no entry either, so this path may
  be dead on real hardware too.

## Live checks for the main agent (PCSX2)

1. MSCALF immediate: break VIF1 on MSCALF; confirm imm 0 for clipped batches
   and 2 for unclipped ones, once per object (not per batch).
2. VU1 data memory: after MSCALF, dump data 0x00-0x1b — expect the matrix in
   0-3, the selector 0x14.z ∈ {0x25, 0x3e}, the addend 0x16.x = 0x16.
3. VU1 PC trace across one batch: expect the resume at 0x01e/0x03f, the JR
   through the data jump table, the transform handler, the XGKICK at
   0x02b/0x087, then the E stop.
4. Step the 0x208-0x23e save/restore block if it ever executes (skinning);
   log vi14 and vf02-vf05 to identify the 14 unknown encodings.
5. Watch data 0x14.z toggle between 0x25 and 0x3e across successive batches.

nf-view `--bond` spots: none — this task made no renderer changes and asserts
nothing visible.
