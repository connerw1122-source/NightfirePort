# Corona (placement type 0xFC) — NO instance (0 bytes at obj+0x110); class 0x01

Proof levels: see [README.md](README.md). Instance size from `Corona_Create` 0x18da00:
`control_create_object(0, …)` [M 0018da00:11] — a zero-size instance; obj+0xE0 still points at
obj+0x110 (one past the 0x110-byte allocation) and **nothing ever reads it**: the placement corona
is a pure header object. Level: ee_start_05 (07000005 at start) unless noted.

Readers/writers census: grep `Corona` over work/decomp/ACTION.ELF → only the creator, the update,
the dispatcher case 0xFC, index.tsv. `Corona_Update` 0x18da78 sits in `control_funcs[0x45]`
(object.md's table); **the placement corona's class is 0x01, whose control_funcs row is empty**
[object.md] — so `Corona_Update` never runs for it. No decompile function stores 0x45 into
obj+0xFF (grep `\+ 0xff\) = 0x45` → only drone-weapon/menu/GS hits, none an obj class store):
the class-0x45 runtime corona has **no creator in the static decompile** (port's U-042, verified).

## Layout (all offsets are obj_header offsets — there is no instance)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0xD8 | ptr | glist (the placement's object record) | [M* 0018da00:17; only writer outside generic init] |
| +0xF0 | u32 | = glist+0x44 flags (merge per `Control_SetGList`). On 07000005's 13 coronas: 0x22D (Corona_Glow2/Corona_test1 ×6) or 0x229 (Window_corona ×7) — bit 0x1 = alpha draw list | [M* 0018da00:18-20] [L-read: obj 016dbcc0 +0xF0 = 0x00022d, obj 016df690 = 0x000229] |
| +0xF4 | u16 | 0 — the parity seed `Corona_Update` would compare against (dead for class 1) | [M* create never writes it; init 0] [L-read: 0 on all 13] |
| +0xFF | u8 | class = **0x01** (not 0x45 — 0x45 is the uncreated runtime corona class) | [M* 0018da00:19] [L-read: 0x01 on all 275 class-1 objects of ee_start_05] |
| +0x106 | u8 | alpha byte, init 0xFF by `control_init_object` 0x134630 [M 00134630:21]. Corona_Update is its only class-specific reader/writer (±0x10 per parity tick toward 0/255 by LOS, `obj+0xF0` bit 0x2000 tracks LOS) — never runs for class 1, so placement coronas keep 0xFF for life | [M* 0018da78:13-35] [L-read: 0xff on all 13 coronas of ee_start_05] |
| +0xE8 | f32 | scale, 1.0 at init; the searchlight corona *part* (a class-0x22 object using the same header bytes) is the one that animates it | [L-read: 0x3f800000 on all 13] |
| +0x30 | vec3 | position from the placement; static | [L-read: e.g. 016df690 = (24.15, 39.76, 51.53)] |
| +0xFC | u8 | draw counter (2↦1↦0 by the renderer), no corona writer | [L-read: 2 on in-view coronas, 0 on the rest] |
| +0xDC | ptr | 0 — the AnimObject fade branch is skipped (corona.rs's reading, confirmed) | [L-read: 0 on all 13] |

`Corona_Update` 0x18da78 in full (the dead-for-placement code path) [M 0018da78]:
gate `GameState+52 & 1 != obj+0xF4` (:12) — runs only when the frame parity differs from the
stored halfword; then `obj+0xFC == 0` → alpha = 0 (:13-14); else LOS(glb_viewer+0xC0, obj+0x30,
*(glb_viewer+4) cel, self, glb_players, 8): fail → alpha = max(α−0x10, 0), `obj+0xF0 &= ~0x2000`
(:20-26); pass → alpha = min(α+0x10, 0xFF), `obj+0xF0 |= 0x2000` (:28-34).

## Param map (from the creator)

**None.** `Corona_Create` never dereferences its level_tag param (`param_3` unused) [M
0018da00:4-24]. Everything visual comes from the glist (model) and the placement transform.

## Port mismatches (crates/nf-game/src/corona.rs — report only)

None found. Every claim in the module doc checks out against the decompile + snapshot:
- "class-1 object via control_create_object(0)" = 0018da00:11,:19 ✓
- "control_funcs[1] is empty / no update ever runs" = object.md's table ✓
- "alpha +0x106 = 0xFF, scale 1.0, +0xFA = 0, +0xF4 = 0, drawn +0xFC" ✓ [L]
- "obj+0xD8 = glist; obj+0xF0 = glist+0x44" ✓ = :17,:20
- "0x22D / 0x229 flags; bit 0x1 → draw list 1" ✓ [L histogram: 6×0x22d + 7×0x229 = 13]
- "Corona_Update sits in control_funcs[69] … no class-0x45 creator found" ✓ (grep verified)
- placements filter `flags & 0xff == 0xfc && flags & 0xa000 == 0` ✓ = dispatcher case 0xFC
  (001d04f0:525-530, which also gates on its `param_7` char — see note).

Note (not a mismatch today): the dispatcher's case 0xFC is gated `if (param_7 != '\0')` while
case 0xDE (searchlight) is not [M 001d04f0:341-344,525-530] — the port creates both from the main
map's placements only, so the gate is invisible; if the port ever parses secondary placement
lists, coronas/destroy/camsubjects must respect that gate and searchlights must not.

## [L-read] spot checks (work/ee_start_05.bin)

Walk: head 0x2705a0 → u32@0x2705b4 → obj+0x14 …; 523 objects; class byte obj+0xFF == 1 → 275
objects (class 1 is shared with other no-update creators — glass/quads — so coronas are picked by
their glist flags +0xF0&0xffff ∈ {0x22D, 0x229}):

1. **class + alpha (+0xFF/+0x106)** — obj 0x016df690: class 0x01, alpha 0xff; all 13 flagged
   coronas read 0xff (the port's "keep their create-time state forever" ✓).
2. **flags (+0xF0)** — 0x00022d on obj 016dbcc0 / 0x000229 on obj 016df690 … 6+7 = 13, matching
   corona.rs's "13 live coronas".
3. **scale (+0xE8)** — 0x3f800000 (1.0) on all 13.
4. **no instance** — obj+0xE0 = obj+0x110 for every class-1 object and the update that would read
   the instance (Corona_Update) is unreachable for class 1; nothing else dereferences it.

## Live checks (for the main agent)

Pointer chain: `DynamicObjList 0x2705a0 → u32@head+0x14 → obj → +0x14 …`, class byte obj+0xFF ==
1, glist flags obj+0xF0. On 07000005 at start, corona 0x016df690 (Window_corona, in view of the
start camera, +0xFC = 2).

1. **Alpha byte** — `python -I tools/trace/fieldprobe.py poke 0x016df796 1 0x40 --shot out.png`
   (obj 016df690 + 0x106). Predicted: that window corona dims to ~25% and nothing else changes
   (no update rewrites it, so the poke sticks — the clean [L-verified] case).
2. **Scale** — `poke 0x016df768 4 <f32 3.0>` (obj+0xE8): the corona billboard triples in size.
3. **Flags** — `poke 0x016df690+0xf0 4 0x000228` (clear bit 0x1): the corona should leave the
   alpha draw list (blend change / depth-sort flicker). Restore afterwards.
