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

## Sky remainder (fix-sky, GAPS G-019) [M unless noted]

`View_DrawSky` (0x1e5ad0) ticks and draws the 21 sky slots (stride 0x40 in `GfxList` at 0x2d6270) whose
`+0x4a` sky id matches its argument — the argument is the slot's **sky group**: 1 = before the world list,
0 = after `psiPostSolid` / before the alpha list (`Game_Draw` 0x1e6f50). Placement parameter 4 is that
group [M parsemap 0x1d04f0 case 0x2a — the caller copies the placement's parameter words to level+0x2c..,
so param 0 = slot, 1 = flash variant, 2 = lightning interval, 3 = flash length, 4 = group].

**Lightning** (`nf_game::sky`): a type-0x2A placement with param 1 ≠ 0 is the flash variant of its slot
(`View_AddSkyObj` 0x1e79a0 stores param 2 × `FRAME_RATE` [L 0x30d0d0 = 60.0] as the dark-interval
`Rand_Rand` argument and param 3 as the flash length). Per tick, when not flashing: the dark timer wraps
0→0xffff and the `(int)(x << 16) < 1` test takes **0 and 1** — a fresh slot flashes on its first tick —
then flash := flash length, dark := `Rand_Rand(interval)`, and `Rand_Rand(100) < 51` makes it a
reflicker-type flash (thunder in 10 ticks, `LightningType` 1, sound 0x5e) with a normal one otherwise
(thunder in `Rand_Rand(120) + 30` ticks, sound 0x5d). While flashing: ~50 % of ticks (`Rand_Rand(100) > 50`)
draw the flash glist instead of the normal one; reflicker-type flash ticks raise `SuperHappyLightningFlag`
[0x30d0b8], which shows the 8-beam lightning polyline (`Lightning_Update` 0x18cf68, re-fractaled by
`Reflicker` via `FractalThing`; the beams follow the player). **The flash does not light the level**: no
ambient write and no `Light_Create` in the path — the level-side evidence is the beams only; the port
leaves a marked hook (`sky::Sky.globals.super_happy`) for 050-fix-lighting. RNG order per tick: flash start
= `Rand_Rand(interval)` then `Rand_Rand(100)` (plus `Rand_Rand(120)` on the non-reflicker branch); a
flashing tick = one `Rand_Rand(100)`; dark ticks draw nothing [M 0x1e5ad0:92-158].

Disc parameters [D]: 0700000a Thunderhead slot 3 = interval 24 → 1440 ticks, flash 5; 0700000b slot 3 =
30 → 1800, flash 10. The other 25 on-foot levels have no flash variant.

**The rotating sky of 0x0700001B**: inside the per-slot body, when the glist flags lack bit 0x8, the frame
counter `rotatosky.168` [0x30d878] increments once per slot pass (both of 0700001b's slots — SKY_StarField
and SKY_Earth, flags 0x222 [D] — qualify, so +2 per frame while unpaused; the two layers' angles stay one
count apart), and the draw matrix gets `RotMatrix([count × 0.00069444446, 0, 0])` concatenated on the
outside: spin · slot-rotation · camera. 0.00069444446 = 1/1440 rad per count → a full revolution in
~4524 frames ≈ 75 s. The same block recomputes the sun vector 0x2d88a0 every pass:
`Vec_Spherical_2_Cartesian(1000.0, 0x409f2c8e, 0xbf29c920)` = 285.00002° azimuth, −38.00000° elevation —
`Env_Create` 0x17b968 fills it from the level's +0x54/+0x58 degree parameters at load; here the hardcoded
angles overwrite it each frame and the rotated matrix is applied (`ApplyMatrixLVI`) [M 0x1e5ad0:186-198,
0x1e2368 RotMatrix, 0x1e4a90 Concat = B := A·B, 0x1e4960].

**The sky pass's GS state**: the pass opens with `psiSetTweakARGB(0xff…)` (no tint), `psiSetUpColourBlend(2)`
→ `AddDMAChainRegSet(0x2a, 0x42)` [M 0x1214f8] → ALPHA_1 = 0x2a = (0−0)·FIX0 + Cs = **replace, no blend**,
`psiLight_SetLights(0,…)` (no lights), and `psiFog(...)` — **a stub** [M 0x121578, size 8; also in
`Game_Draw`], so the sky pass itself writes no fog register. The glists confirm the inheritance [D
sky_gs_live]: the 0700000a Thunderhead's first CNT carries **no A+D writes at all** (effective state =
the pass-initial 0x2a / TEST_1 0x5001b), while 0700001b's SKY_StarField writes its own ALPHA_1 0x44
(normal blend), TEST_1 0x5001b and ZBUF_1 0x1_31000000. gpu.rs's sky pipelines (pass 0: depth Always, no
write; pass 4: LessEqual, no write, far-pin) stay as modeled; the exact ZMSK state the GS holds during the
sky draws is a live check for the main agent below.

**Snow and rain** are not sky-pass draws: `DrawDrops` 0x18cb90 runs after the alpha list (blend 2, no
lights, `Game_Draw`). The physics/density live in `nf_game::env.rs` (`InitDrops` 0x18bf68 etc.). The look,
from the decompile: DropType 1 draws each drop as sprite glist 0x2000290 at the drop position (plus a
second, darker sprite 0x2000039 tinted `psiSetTweakARGB(0x24,0x20,0x20,0x20)` on levels 0x07000009-b);
DropType 0/2 draw through `psiDrawParticleList(0.2f, …, argb 0xff808080, glist 0x3000045 / 0x3000087)`.
Max counts: 4096 (snow), 2048 (rising), 128/64 (rain, split by level ≥ 0x0700000c).
