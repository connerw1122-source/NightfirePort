# GS leaf state and character lighting (obj+0x100..0x106, TweakRGBA, the 0x800 path)

Plan v2 §2 struct 13. "Leaf" here is the PS2_GFX tree leaf (the thing that carries per-leaf GS
DIRECT writes — alpha/test/zbuf — already rendered per-batch by nf-view). This file documents the
per-object colour state the game multiplies under those leaves, the global tweak, and the exact
difference between the world-geometry path and the skinned/view-model path — the difference behind
"characters and guns are too bright while the world is right".

## The per-object colour bytes [M]

| off | type | name | init | writers | readers | tag |
|---|---|---|---|---|---|---|
| +0x100..0x102 | 3×u8 | **ambient light RGB** | 0xff each (`control_init_object` 0x134630) | `Lights_CalcAmbientLight` 0x1cb178 — ramps toward the cel's ambient (`cel + switch_channels[cel+0xa2] + 0x9a/0x9c/0x9e`) at rate 0.1, only when `obj+0xF0 & 0x800`; `obj+0xF0 & 0x10000` + `glb_blokes+0x962` doubles the targets; snap-on-cel-link when still 0xffffff (see cel.md) | `View_SetupRenderModes_Tweak` 0x1e5888 (the 0x800 branch) | [M] |
| +0x103 | u8 | tint R | 0xff | `Control_CreateObjEx` 0x1343c8 (param), `Script_SetColour` 0x1db770, `Player_InitWeapon` 0x1a7ef8 (0x7f), `Player_LaserPointer` 0x1a65d8 (0x40), `MP_UplinkUpdate` 0x186d98, `gases_smoke` 0x1ab2d0 | Tweak (below), `AnimObjectDraw` 0x1c4490, `MP_GetRadarObjects` 0x189840 | [M] |
| +0x104 | u8 | tint G | 0xff | same (Player_InitWeapon sets 0) | same | [M] |
| +0x105 | u8 | tint B | 0xff | same | same | [M] |
| +0x106 | u8 | tint A | 0xff | init only | Tweak (halved `>>2` when `obj+0xF0 & 0x20000` and `viewer+0x236 == 1`), AnimObjectDraw | [M] |

## psiSetTweakARGB (0x124770) and the VU1 ambient term [M]

`psiSetTweakARGB(a, r, g, b)` @0x124770 stores the four bytes and computes:

- `TweakRGBA` (u8 @0x30d714) = 0 if all four args are 0xff, else 1 — the "tweak active" flag;
- `TAmbient` (4×f32 @0x2650d0..0x2650dc) = a/255, r/255, g/255, b/255.

`TAmbient` is the fourth colour vector of `sceVu0LightColorMatrix(m, TColor0 0x2650a0, 0x2650b0,
0x2650c0, TAmbient)` in **both** matrix builders — `FillMatrixChainSkin` 0x122788 (skinned) and
`FillMatrixChainRot` 0x122310 (rotating/world objects) — i.e. the tweak colour is the **ambient
term of the VU1 lighting matrix**. `TweakRGBA != 0` adds two more words to the DMA packet
(0x122788/0x122310), so the ambient multiply is only uploaded when the tweak is non-white.
`psiSetTweakA` 0x124878 rewrites only alpha. `bTweakBrightness` @0x30d2b4 is written by the menu
brightness handler `C_CHCHBRIGHT_Handler` 0x216460; no decompiled reader (suspected in hand-written
VU1 upload) — unverified.

## The 0x800 flag — two distinct tests [M]

1. **`obj+0xF0 & 0x800`** — the "character lighting" object bit. In
   `View_SetupRenderModes_Tweak` 0x1e5888 it selects whether the tint is multiplied by the object's
   ambient light before `psiSetTweakARGB`:

```
without 0x800:  psiSetTweakARGB(A, T_R,             T_G,             T_B           )
with    0x800:  psiSetTweakARGB(A, T_R·L_R >> 8,    T_G·L_G >> 8,    T_B·L_B >> 8  )
```

   where `T_` = obj+0x103..0x105 (tint), `L_` = obj+0x100..0x102 (ambient light). A close-up
   flash variant (flag 0x80000 on the object or its glist, viewer within 4 units, `DAT_002a38d4`)
   multiplies by `FlashMod` via floats instead. Alpha byte obj+0x106 is passed as-is (or `>>2` under
   the 0x20000 flag). `Lights_CalcAmbientLight` uses the same bit to decide whether to *maintain*
   obj+0x100..0x102 at all — objects without 0x800 keep the white sentinel and are never dimmed.
2. **`celglist+0x44 & 0x800`** — in `AnimObjectDraw` 0x1c4490 (per mesh piece): if the glist is
   *not* 0x800, the tweak is re-set from the object tint before drawing that piece; if it *is* 0x800,
   the tweak is left as-is. (The setter for skinned pieces in that path was not traced this pass.)

## Colour math per draw path [M]

All paths end in the VU1 lighting matrix (`sceVu0LightColorMatrix` with `TColor0/1/2` dynamic lights
+ `TAmbient`), applied to vertex RGBA on the PS2 scale (0x80 = 1.0), then modulated with the texture:

| path | chain | tweak set from |
|---|---|---|
| world cel geometry | `View_DrawObjList` 0x1e69a0:100 → `View_DrawCelGlist` 0x1e6538:57 → **`psiSetTweakARGB(a, 0xff, 0xff, 0xff)`** | forced white → TweakRGBA = 0, **no ambient term at all** (baked vertex lighting dominates) |
| world object, non-anim (no 0x800) | `View_DrawObjList`:77 → `View_SetupRenderModes_Tweak` + `Lights_CalcClosestLights`/`psiLight_SetLights(1,1,1,…)`:78-81 | tint only (default white → off); dynamic lights still apply |
| **0x800 object (characters, guns)** | same chain | **tint × per-object ambient light** (the multiply above) |
| anim object (`obj+0xFA & 8`) | `View_DrawObjList`:96 → `AnimObjectDraw` 0x1c4490 → per piece: non-0x800 glists get the tint re-set; then `psiDrawObjectMatrix` 0x124708 or `psiDrawSkinObjectMatrix` 0x111828 (bones via `SkinIt`, then `psiDrawClonedObject` 0x122928:197) | tint for plain glists; kept/stale for 0x800 glists (exact setter untraced) |

Effective game math: **0x800 object** `GS_RGB ≈ texel × vertex_rgba × (tint/255 × ambient/255 + Σ
dynamic lights)`; **world cels** `GS_RGB ≈ texel × vertex_rgba × (Σ dynamic lights)` (no ambient
multiply — that is why the world is right while characters are not).

## Root cause of the too-bright characters and guns [M+S]

The game dims every `obj+0xF0 & 0x800` object (characters and the first-person view model, which is
a skinned object on the same path) by `tint × obj+0x100..0x102` — and `Lights_CalcAmbientLight`
keeps those bytes pinned at the **level's ambient colour**. On 07000005 that colour is
**RGB (0x39, 0x46, 0x56) = (57, 70, 86)** — the characters render at ≈ 22–34 % albedo [L below].
The port never applies that multiply:

- `crates/nf-view/src/scene.rs:943` — `vs.push(Vertex { pos: p, uv, rgba: v.rgba })`: leaf/skin
  vertex colours are emitted verbatim, for world **and** `Layer::Character` (the pass mapping at
  scene.rs:902 gives Character (2, 1); "characters are drawn like the world", scene.rs:7–9). View
  models are built from the same skins (scene.rs:218–220).
- `crates/nf-view/src/gpu.rs:39` — `o.c = c * (255.0/128.0)` (with the display `GAIN` 145/128,
  gpu.rs:18) is the only colour operation in the shader; grep finds no per-object light multiply in
  nf-view.

So the port renders characters/guns at ~100 % albedo (× the 1.992 PS2→float rescale) where the game
renders them at 22–34 %. **The missing operation is the ambient multiply for 0x800 objects**:
`vertex_rgba × (obj+0x103..0x105 tint × obj+0x100..0x102 ambient / 255)` — the `TAmbient` VU1
ambient term uploaded by `FillMatrixChainSkin` 0x122788 / `FillMatrixChainRot` 0x122310.

Fix shape (renderer work, not this task): per Character-layer/view-model instance, maintain an
ambient RGB stepped toward the level's ambient bytes as `Lights_CalcAmbientLight` does (rate 0.1
per frame, doubling under the 0x10000 flag), and multiply it (times the tint) into the vertex
colour or a per-instance uniform for Character/view-model draws only. World geometry must stay
untouched — the game's world-cel path forces the white tweak.

## Snapshot check [L] (work/ee_start_05.bin)

| addr | raw | meaning |
|---|---|---|
| 0x30d714 (TweakRGBA) | `00 00 00 00` | tweak flag 0 — no tweak active at capture (consistent: nothing drawn) |
| 0x30d2b4 (bTweakBrightness) | `00` | default brightness |
| obj 0x01382bc0 (sniper, class 02, +0xF0 = 0x00000880 → **0x800 set**) | +0x100..0x107 = `39 46 56 ff ff ff ff 00` | ambient (57,70,86) dim blue; tint white |
| obj 0x0137bfa0 (class 3d, +0xF0 = 0x00008880 → 0x800 set) | `39 46 56 ff ff ff ff 00` | same |
| obj 0x0137b5b0 (class 1d world, +0xF0 = 0x00000050 — no 0x800) | `39 46 56 ff ff ff ff 00` | same bytes, but the game **bypasses** the multiply at draw time for it |

The ambient bytes are identical across objects because every object snaps/ramps toward the same
level ambient; the 0x800 bit alone decides who gets dimmed.

## Not verified this pass

- The readers of `bCharacterLight` (0x30cdd8) / `bShadowCharacter` (writes only in the decomp) —
  suspected in hand-written VU1 asm; the exact VU1 branch semantics for skinned output are inferred.
- Which `sceVu0LightColorMatrix` input slot is "ambient" vs light 0/1/2 (naming-based; the function
  0x229108 and `psiLight_SetLights` were not disassembled).
- Who sets the tweak for **0x800-flagged celglists** inside `AnimObjectDraw` (the stale/kept path);
  if characters draw through it, the ambient may arrive via the VU1 route rather than
  `View_SetupRenderModes_Tweak` — the multiply happens either way, only the carrier differs.
- The meaning of the +2 DMA words gated by `TweakRGBA`; `bTweakBrightness`'s reader.
- The view model's exact draw function (its psiDraw call site was not isolated this pass; it is
  skinned geometry on the 0x800 path per the user-visible symptom).
