# World rendering: fog, GS state per leaf, lighting, snow, output (07000005)

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] decomp, [L] snapshot `work/ee_start_05.bin`, [S] suggested. Source: a
research pass (subagent, 2026-10-09; decoded all 713 ps2gfx leaves of 07000005); not yet re-checked here.
**Status: spec only — the render subsystem is not ported yet** (planned after animation).

Ranked by visual impact: depth fog, snow, per-leaf blend / test / depth-write state, the ×1.13 output gain, the
alpha-list distance fade, environment mapping, dynamic lights / motion blur / breath puffs.

## Fog: a screen pass from the depth buffer [M, L]

- No GS fog (PRIM 0x5C, FGE 0; `psiFog` 0x121578 is empty). `psiPreGameRun` (0x11e2e8) per level sets `Ps2Far1`, the
  fog colour, `FogOn`, the snow size: 07000002 9987520 / 24 2e 4c; 04 9379776 / 11 15 21; **05 19118528 / 3a 4d 68,
  snow 28990**; 06 8059232 / 2e 3c 52, 12826; 08 13191350 / 3b 50 69, 19053; 0C 20475840 / d1 d5 d6; 0D 19707840 /
  92 85 69; others 4194240, no fog.
- Snapshot [L]: `FogOn` 0x30c765 = 1, colour 0x30c981.. = 58, 77, 104, `Ps2Near1` 8.0, `Ps2Far1` 19118528,
  `Ps2Near2` 0.1, `Ps2Far2` 16777216.
- Depth (`PS2WorldViewVU1Kick2` 0x10b2b0): zbuf(d) = N1 + (F1 − N1) · n · (f/d − 1) / (f − n) (≈ 8 + 1911852 / d).
- `psiPostSolid` → `psiDrawFogPass` (0x10b960): Z bits 8–15 into frame alpha (`CreateMoveRG2BA` 0x118718), then a
  full-screen fog-colour sprite (`CreateFogSprite` 0x119228, Z 0x7FFF, ZTST GREATER, ALPHA 0x11: (Cd − Cs)·Ad + Cs).
  Port: for list 0 and sky pass 1 only, `if zb < 32767 { a = floor(zb/256)/128; rgb = fog + (rgb − fog)·a }` —
  hyperbolic: none nearer than ~58 m, 50 % at 117 m, 90 % at 584 m. Not fogged: the alpha list, sky pass 0, shards,
  snow, list 2.
- Alpha list fade (`FillMatrixChainRot` 0x122310 with `AlphaFogObjects`): alpha × min(zb >> 7, 255) / 255.

## Per-leaf GS state (closes most of G-018 / U-021) [measured over all leaves]

- One (sometimes two) `DIRECT` A+D packet per leaf writes only ALPHA_1, TEST_1, ZBUF_1; leaves without one inherit
  the previous leaf's state.
- ALPHA_1: 0x44 normal (Cs − Cd)·As + Cd; 0x7F_00000048 additive Cs·As + Cd; 0x7F_00000042 subtractive Cd − Cs·As
  (bullet holes).
- TEST_1: 0x5001B (689 leaves): alpha test passes for As ≥ 1 — **only A = 0 is discarded** (the viewer's 0.5 test is
  wrong); depth GEQUAL. 0x507FD (2 leaves: `Leaf`, `spark leaves`): As > 0x7F.
- ZBUF_1: ZMSK 0 (write, 570 leaves) / 1 (no write, 150). Blending is always on (PRIM ABE).
- Blend layers [M, D]: a leaf draws an opaque base batch (vertex alpha 0x80) and then the same triangles again with
  another texture weighted by vertex alpha (0 where it does not show) — 822 of 2000 list-0 batches on 07000005 (road,
  outer walls, gate towers). Order within a leaf is the batch order, and the overlay passes the depth test at equal
  depth (ZTST GEQUAL on the GS's inverted Z = "≤" in the port). WORKLOG 0059.
- Typical: 463 normal + depth write (rooms); ~130 additive no write (blasts, coronas, moon, searchlights, glows);
  16 normal no write (glass, footprints, beams); 14 normal + write flagged 0x1 (pine trees, skybox, shadows — as
  statics their cel primary still draws in list 0 parse order; the flag matters for the +0x18 chain records and
  dynamic objects, models-world).
- Draw lists: `View_AddCels` (0x1e6ba0) pushes every visible runtime cel's **primary glist** into
  **list 0** with a synthetic key −90000 + 10k in GfxList walk order (the VisCelList QuickSort
  orders the chain by cel pointer ≈ allocation ≈ placement-parse order), *without consulting the
  glist flags for the list choice* — the primary push has none [M decompile re-read, models-world].
  The cel's +0x18 chain records (LOD variants — never present on this disc: no 0x1f blocks) and the
  per-object entries (`View_AddObjects` 0x1e62d8) pick their list by flags: 0x1 → list 1
  back-to-front (key −dist²), 0x2000 → list 2 after `psiClearZ`, else list 0 with +dist².
  `View_DrawObjList` (0x1e69a0) QuickSorts ONE mixed array of cel and object entries per list and
  draws each (cel entries through `View_DrawCelGlist`, objects through
  `View_SetupRenderModes_Tweak` + `View_DrawObjectGlist`). So list 0 = **all visible statics in
  placement-parse order, then the dynamic objects front-to-back**; list 1 = the flagged objects
  back-to-front. Flags 0x8 billboard, 0x4 scale jitter 1.0–1.05 per frame, 0x10 hidden (the
  cel-pass skip — `View_AddCels` never pushes a record with glist flags bit 0x10); `cel+0xA3`
  alpha pulse for placement types 0x8018 / 0x8019.
- Textures: bilinear, no mipmaps (TEX1 0x60); REGION_REPEAT inside the page; MODULATE, TCC 1;
  animated frame (ticks per frame = 60 / rate byte for 1–59, else 1)
  (GameState+0x38 / rate) % frames. Per-object GS state: only ALPHA_1/TEST_1/ZBUF_1 occur in the
  packets (ps2gfx.md §GS state; the 0x58/0x64 env-map/mod shapes included).

## Lighting [M, L]

Prelit: texture × vertex colour (0x80 = 1.0). Up to 2 point lights per draw, selected with the
source object's glist flags word (`celglist+0x44`, copied to `obj+0xF0` by `Control_SetGList`
0x134960; cels carry the same word at `cel+0x94`): kind 0 (level/ambient-radiator lights) and kind
2 (searchlight beams) need bit 0x80, kind 1 (muzzle flashes) is skipped when bit 0x100 is set
(`Lights_CalcClosestLights` 0x1cb2e0). Measured on the disc [D, fix-lighting2]: world cels carry
almost no 0x80/0x100 (the radiators light only characters — skins 0x8a0/0x28a0 — not rooms), the
first-person weapon parts are 0x28a0, and the effect objects are 0x0321 (0x100: muzzle-excluded);
`View_DrawSky` 0x1e5ad0 / `View_DrawSprites` 0x1e5290 upload no lights at all. The VU1 point-light
program (.vutext ~832-960) adds `TColor.rgb · max(0,(1/d²−1/r²))·max(0,N·L)` in BYTES over the
vertex colour and clamps the sum to byte 255 (`MINIi`, I = 255.0 at I-mem 919/954) — the port's
shader mirrors that at /255 over its Unorm8x4 vertex colour with a min(·,1) clamp (fix-lighting2;
the earlier /128 over an Unorm8x4 colour was 2× the game's add). Tint (`obj+0x103..0x106`, × room
ambient with flag 0x800); environment map on 31 objects (box flag +0x34 bit 0, e.g.
`OBJECT_windows`): UV ≈ 0.5 + rot·n/256 [S].

## Snow (`Env_*`, `*Drops`) [M, L]

- Placement 0xDF `SNOW_gen` → `Env_Create` 0x17b968 → `InitDrops(0)` 0x18bf68: 4096 flakes, texture 0x03000045
  (32 × 32); 32 velocities x, z ∈ [−0.02, 0.02], y ∈ [−0.1, −0.025] per tick (±0.002 x/z on 07000007–08).
- Volumes: placements 0x32 `Effect_Snow_N` (18; `RainBox_Create` 0x18d840): the entity-params AABB.
- `UpdateDrops` 0x18c580: cube centre = eye + 7.5 along the player's facing; boxes clipped to ±15; wind random walk
  ±0.0002 / tick clamped ±0.01; up to 21 respawns per tick at x, z ± 15, y = centre.y + 7.5 + rand(7.5), only inside an
  active box; a flake dies below max(box min y, centre.y − 7.5). First frame on 05 / 06: all 4096 in the cube.
- `DrawDrops` 0x18cb90 → `psiDrawParticleList(0.2, …, 0x808080, 0x03000045)`: GS sprites, additive, depth-tested,
  no write, colour (63, 63, 63, 127) × texture; size ≈ 0.2 m [S: full or half width open].

## Output [M, L]

`CreateSqueezeFrame` 0x11b140 each frame: RGB × `SqueezeColourIn` / 0x80 = 0x91 → ×1.133, bilinear with a 7/16-texel
offset (`PS2Filter` 2); motion blur only while `GlobalBlur` ≠ 0 (damage flash, camera shear, zoom; −0x10 / frame).
Sky: `View_DrawSky` twice — slots with param 4 = 1 before the world (fogged), others after the fog pass.

## Open

VU1 microcode (snow size, light falloff, env map), who writes `SqueezeColourIn`, the FBMSK reading of the fog pass.
