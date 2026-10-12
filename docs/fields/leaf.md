# Leaf state — the per-leaf GS DIRECT state and the per-object colour bytes it multiplies

Proof levels: see [README.md](README.md). "Leaf" = the PS2_GFX tree leaf that carries per-leaf GS
DIRECT writes (alpha / test / zbuf), rendered per-batch by nf-view. The deep GS walk is
docs/atlas/gs-leaf.md (kept as the render-side reference); this doc is the field-map for the
per-object colour bytes the leaves multiply and the global tweak. Level: ee_start_05 unless noted.

## The per-object colour bytes (obj header)

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x100..0x102 | 3×u8 | ambient light RGB, init 0xFF (`control_init_object` 0x134630); `Lights_CalcAmbientLight` 0x1cb178 ramps toward the cel ambient at rate 0.1 when `obj+0xF0 & 0x800`; snap-on-cel-link when still 0xFFFFFF (`control_link_object_to_cel` 0x1346b8) | [M* docs/atlas/gs-leaf.md table — writer+reader cited there] |
| +0x103 | u8 | tint R, init 0xFF | [M* writers `Control_CreateObjEx` 0x1343c8, `Script_SetColour` 0x1db770, `Player_InitWeapon` 0x1a7ef8, `Player_LaserPointer` 0x1a65d8, `MP_UplinkUpdate` 0x186d98, `gases_smoke` 0x1ab2d0; reader `View_SetupRenderModes_Tweak` 0x1e5888, `AnimObjectDraw` 0x1c4490] |
| +0x104 | u8 | tint G, init 0xFF (`Player_InitWeapon` sets 0) | [M* same set] |
| +0x105 | u8 | tint B, init 0xFF | [M* same set] |
| +0x106 | u8 | tint A, init 0xFF — the corona alpha byte for class-0x45 objects (`Corona_Update` 0x18da78 ±0x10 per parity tick), halved `>>2` under `obj+0xF0 & 0x20000` in the tweak | [M* gs-leaf.md + docs/fields/corona.md] |

## The tweak

`psiSetTweakARGB(a, r, g, b)` 0x124770: `TweakRGBA` u8 @0x30d714 = 0 when all four args are 0xFF
(the flag), `TAmbient` 4×f32 @0x2650d0 = the args /255. `TAmbient` is the fourth colour vector of
`sceVu0LightColorMatrix` in BOTH VU1 matrix builders — `FillMatrixChainSkin` 0x122788 (skinned)
and `FillMatrixChainRot` 0x122310 (world) — the ambient term of the lighting matrix; the
non-white tweak adds two DMA words [M* 0x124770, both builders cited in gs-leaf.md].

## The per-leaf GS state (summary — full walk in gs-leaf.md)

A leaf's alpha/test/zbuf/blend reach the GS as DIRECT register writes; nf-view already renders
them per-batch. The fields the world-object structs rely on are the obj-side bytes above (the
leaf multiplies obj tint × ambient) and `obj+0xF0`'s draw-list bits (docs/fields/obj-tag.md
+0xF0). No leaf-side field is re-mapped here.

## Port check

`crates/nf-view/src/` (ps2gfx.rs) implements the per-batch GS state; the tint/ambient path was
the fix-lighting lane's subject (WORKLOG 050). Cross-checked claims: the +0x103..+0x105 tint
writers listed above all exist in the port's lighting model [M gs-leaf.md]; **no mismatch found**
in the field meanings this doc covers.

## [L-read] spot checks (work/ee_start_05.bin)

Every object at level start carries +0x100..0x106 = 0xFF (init) except where the creators wrote
tints [L — e.g. the coronas' +0x106 = 0xFF on all 13, docs/fields/corona.md]. The drones
ramp +0x100..0x102 during play — not snapshot-visible at frame 0.

## Live checks

1. `poke` an object's +0x103..0x105 (tint RGB) to (0x20,0xFF,0x20) with `--shot` — the object
   should render green-tinted (the VU1 ambient term).
2. `psiSetTweakARGB` probe: `watch` TweakRGBA @0x30d714 during a script `SetColour` — it flips to
   1 exactly when a non-white colour lands and back to 0 on 0xFFFFFFFF.
3. The +0x106 halving (`obj+0xF0 & 0x20000` + viewer mode): drive the NIS viewer over an object
   with bit 0x20000 and compare the rendered alpha.
