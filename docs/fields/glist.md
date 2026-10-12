# celglist_tag (the "glist" — per-placement geometry record, 0x50 bytes in the pCelList array)

Proof levels: see [README.md](README.md). Size 0x50: the map parser strides the cel/glist array
at `pCelList + index * 0x50` and hands `pCelList + index*0x50` (+ a +0x50 sibling) to the creators
[M `parsemap_block_map_data_dynamic` 0x1d1b68:63,79,96]. An object holds its glist at obj+0xD8;
a glist-less object (obj+0xD8 == 0) is skipped by the renderer's object pass and gets a 0.1-radius
world sphere [M `View_AddObjects` 0x1e62d8:27; `Control_BuildWorldSph` 0x133070:24-33]. This doc
covers the fields the port and the world objects lean on; the full per-field census for the draw
path is the models lane's gs-leaf/gs work. Level: ee_start_05 (07000005 at start).

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x10 | vec3 | the model's LOCAL bounding-sphere centre: `Control_BuildWorldSph` VU0-transforms it by the object matrix (rows +0x90..+0xCC) into obj+0x80 (the world sphere centre) | [M* read 0x133070:36-40 (`_lqc2(iVar1 + 0x10)` …); writer: the map builder (disc)] |
| +0x1C | f32 | the local sphere radius: × abs(obj scale +0xE8) → obj+0x8C | [M* 0x133070 (same block; the atlas row +0x8C cites the same fn)] |
| +0x40 | u32 | the glist id/name: `Destroy_Create` stores it into its instance (+0x00) and `Destroy_Update` compares it against the CURRENT glist's +0x40 to detect a model swap | [M1 read 0x12c318:25-30, 0x12c4f8:29] |
| +0x44 | u32 | **the draw-flags word** — copied to obj+0xF0 (masked `old & 0x30008000`) by `Control_SetGList` and by every placement creator: bit 0x1 = the alpha/mirror draw list; bit 0x10 = object not drawn; bit 0x40000 = clear the object's display-mask bit 0x10 (+0xF8 &˜ 0x10); bit 0x80000 = the render-mode tweak gate; the high bits 0x30008000 ride through `Control_SetGList` untouched | [M* writer→obj: 0x134960:16-18; bit 0x40000: 0x134960:19-21; bit 0x80000: `View_SetupRenderModes_Tweak` 0x1e5888:24; low bits' meaning via obj+0xF0: docs/atlas/object.md +0xF0 and the creators (`*(undefined4 *)(iVar + 0xf0) = *(undefined4 *)(param_4 + 0x44)` — e.g. Trigger 0x1b1720:95, FuseBox 0x1b0448:27, Wire 0x1981d8:27)] |

The rest of the 0x50 bytes (the per-batch GS packet data the renderer walks — the models lane's
territory, see docs/atlas/gs-leaf.md) is not re-mapped here.

## Port check

- `crates/nf-game/src/triggers.rs` `glist_0x10`: "the port does not read glist+0x44 yet — live
  data: every TouchOnce on 07000005 carries the bit" — consistent with +0x44 → obj+0xF0 bit 0x10
  [M above]. No fix needed; the field map confirms the bit's source.
- `crates/nf-view/src/` draws from the parsed glists directly (nf-formats::level) — the parse
  side is the models lane's ground truth, not re-checked here.

## [L-read] spot checks (work/ee_start_05.bin)

Corona objects (class 1) carry obj+0xF0 = 0x22D/0x229 straight from their glist+0x44 [L —
docs/fields/corona.md's table], and their obj+0xD8 points at the placement's glist in the
pCelList array [L]. The 0x44 words of those glists read back exactly 0x22D/0x229 — the copy is
verified both directions.

## Live checks

1. Find a glist live: any object → obj+0xD8.
2. `poke` glist+0x44 bit 0x10 (set 0x10) on a door's glist — the door should vanish (obj+0xF0's
   0x10 makes `View_AddObjects` skip it); clear it again to restore.
3. `watch` glist+0x40 on a Destroy pair (07000002's shot-out scenery): shoot the breakable — the
   paired Destroy's `Destroy_Update` sees obj+0xD8's +0x40 differ from its stored +0x00 and
   smashes (the model swap + explosion).
