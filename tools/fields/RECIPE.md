# fields-world survey recipe (READ THIS FULLY before any grep)

You are doing a **field-map survey** of one or more runtime structs of 007 Nightfire (PS2 NTSC-U)
for the NightfirePort Rust port. Repo root: `/home/z/NightfirePort`. Your output is a draft
`docs/fields/drafts/<name>.md` per struct plus a written summary of port mismatches. **Do not
modify anything under `crates/` and do not write anywhere else except your draft files and
`/home/z/my-project/worklog.md` (append only).**

## The materials

- Decompiled C, one file per function: `work/decomp/ACTION.ELF/NNNNNNNN_Name__MANGLE.c`
  (NNNNNNNN = function address). ~3039 files. The mangled signature in the filename lists
  parameter types: `P7obj_tag` = obj_tag*, `P9level_tag` = level_tag*, `P12celglist_tag`,
  `P8DOORINFO`, `T0` = same type as param 1, `R16block_header_tag` = reference.
- Disassembly lives in the same repo? No — there is no .s dump; the C is Ghidra output. Where the
  C looks incomplete (missing stores, `unaff_` variables, fused lines), say so in the doc and mark
  the row [M1] with a note rather than guessing.
- EE snapshots: `work/ee_start_05.bin` (level 07000005 at start) and `work/ee_start_*.bin` others.
  **File offset = EE address.** Object list: head 0x2705a0, first object at head+0x14, next =
  obj+0x14, count at 0x30ca98. obj+0xFF = class byte; instance = obj+0xE0 (pointer, = obj+0x110).
- Tools you must use:
  - `python3 tools/fields/param_offsets.py <StructType>` — lists every function whose mangled
    signature takes `<StructType>*` and every offset touched directly on that param.
  - `Grep` tool over `work/decomp/ACTION.ELF` for raw patterns like `\+ 0x2e\)` or `\[0x17\]`.
- The port sources: `crates/nf-game/src/*.rs` (and `crates/nf-formats/src/placement.rs`). Many
  port modules have doc comments citing game addresses — mine them for candidate readers, then
  verify every claim against the decompile before you cite it.

## THE ARITHMETIC RULES (get these wrong and every row is wrong)

Ghidra C uses the base pointer's *declared type* for `[k]` and bare `+ k` forms, but byte offsets
after `(int)` casts:

| declared var type            | `var + 0xe` / `var[0xe]` means byte | `(int)var + 0x38` means byte |
|---|---|---|
| `undefined4*` / `int*` / `float*` | 0x38 (0xe·4) | 0x38 |
| `undefined2*` / `ushort*` / `short*` | 0x1c (0xe·2) | 0x38 |
| `undefined1*` / `char*` / `uchar*` | 0x0e | 0x38 |
| `undefined8*` / `long*` / `double*` | 0x70 | 0x38 |

- `*(undefined2 *)(__s + 0xe)` on an `undefined4*` `__s` is **byte 0x38**.
- `*(undefined2 *)((int)__s + 0x26)` is **byte 0x26** whatever `__s` is.
- Casts also tell the width: `undefined1` = u8, `undefined2` = u16, `undefined4` = u32/float,
  `short` = s16, `ushort` = u16, `undefined8` = 64-bit.
- The doc rows always quote the **byte** offset. When you cite a decompile line, cite
  `FILE.c:LINE` and show the original text so a checker can re-derive the scaling.

## Method, per struct

1. **Creator.** Find it via `docs/atlas/object.md`'s dispatch table (placement type → creator
   address). Read the .c file. Record: `control_create_object(size, …)` instance size, the class
   byte written to obj+0xFF, and every instance field written (byte offset, width, source:
   which level_tag param / literal / computed). This gives you the layout skeleton.
2. **Every reader.** Grep the whole decompile for accesses at each candidate byte offset in both
   scaling forms (`+ 0xHH)` and `[0xHH]`), filter to plausible base pointers, then open each hit
   and follow the base pointer's origin a few lines up (e.g. `*(undefined4 **)(iVar + 0xe0)` =
   the instance; `param_2` with a matching signature; a global). A field is **[M\*]** only when
   you have listed *every* hit and they agree on the meaning. Otherwise [M1] with the count.
   Also check `param_offsets.py <Type>` output for functions that take the instance directly.
3. **Disagreements.** Same offset, different meaning per class or a union/reuse — record each
   case and the selecting condition. Never resolve silently.
4. **Port check.** Grep `crates/nf-game/src/` (start with the module named for your struct) for
   the field by offset or name. State per field: port reads it and agrees / port reads it and
   DISAGREES (quote the port line) / port doesn't use it. Mismatches go in your summary —
   **do not fix them**.
5. **[L-read] spot checks.** For at least the 3 most gameplay-relevant fields, find a live
   instance in `work/ee_start_05.bin` (walk the object list to a matching class byte; instance =
   the u32 at obj+0xE0) and read the field's bytes (little-endian; floats are f32). Record
   address + raw value + interpreted value. Use python3 one-liners; do NOT commit any dump.
6. **Live checks for the main agent.** End the doc with a "Live checks" section: for fields the
   port relies on visibly or for gameplay, write the exact probe:
   `python -I tools/trace/fieldprobe.py poke <addr> <width> <value> --shot out.png` or
   `watch`/`diff`, how to find the instance live (pointer chain from a global, e.g.
   `DynamicObjList 0x2705a0 → head+0x14 → obj+0xE0`), and what the screen/state should show if
   your reading is right. If the game rewrites the field every frame, give the input that drives
   it instead.

## Doc format (follow docs/fields/sprite.md)

```markdown
# <Struct> (the <thing> instance, N bytes at obj+0x110; class 0xHH)

Proof levels: see [README.md](../fields/README.md). Instance size from `Control_Create...` 0xHHHHHH.
Level: ee_start_05 (07000005 at start) unless noted.

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | u32 | ... | [M* 0xHHHHHH:L, 0xHHHHHH:L] [L-read: obj 0xHHHHHH → 0xVVVVVVVV] |

## Param map (from the creator)

| level_tag param | Instance offset |
|---|---|

## Live checks
...
```

## Worklog

Read `/home/z/my-project/worklog.md` first (it records prior agents' work). When done, append:

```
---
Task ID: <given by caller>
Agent: <your name>
Task: <one line>

Work Log:
- <steps>

Stage Summary:
- <key results, esp. port mismatches found>
```
