# Atlas port-check: nf-view `scene.rs` / `gpu.rs` and nf-game `searchlight.rs` vs the game

Plan v2 §2: "Any `+0xNNN` in a comment must exist in the atlas." This file audits the rendering-side
and searchlight-side claims measured for atlas-5 against `docs/atlas/gs-leaf.md`, `cel.md`,
`searchlight.md` and the decomp (ACTION.ELF `e7ffdcb3…`). Verdicts: **OK**, **OK-with-note**,
**MISMATCH** (port does what the game does not — a port bug), **UNVERIFIED** (not re-measured).
Rust line numbers refer to master 5d0d1c5.

## 1. The brightness bug — characters and view models too bright (scene.rs, gpu.rs)

### 1.1 MISMATCH — no per-object ambient multiply for 0x800 objects (scene.rs:943, gpu.rs:39)

Game [M 0x1e5888 (View_SetupRenderModes_Tweak) + 0x1cb178 (Lights_CalcAmbientLight)]:

```
with obj+0xF0 & 0x800:
    tweak.rgb = tint(obj+0x103..0x105) × ambient(obj+0x100..0x102) >> 8
    → VU1 lighting-matrix ambient term (TAmbient 0x2650d0, FillMatrixChainSkin 0x122788)
```

`Lights_CalcAmbientLight` pins obj+0x100..0x102 to the level's ambient colour (07000005:
RGB 0x39/0x46/0x56 ≈ 22–34 % albedo [L]). Port: `scene.rs:943` pushes leaf/skin vertex colours
verbatim for world and `Layer::Character` alike; `gpu.rs:39` `o.c = c * (255.0/128.0)` is the only
colour op in the shader. **Net: characters and the first-person gun render at ~100 % albedo where
the game renders them at 22–34 %.** This is the exact cause of the user's report ("characters and
guns are too bright while the world is right"); the world is right because the game's world-cel
path forces the white tweak (`View_DrawCelGlist` 0x1e6538:57 → TweakRGBA = 0) and the port matches
that by doing nothing.

Fix shape: per Character/view-model instance ambient RGB (stepped toward the level ambient at rate
0.1, doubling under obj+0xF0 & 0x10000), multiplied with the tint into the vertex colour or a
per-instance uniform, for Character-layer and view-model draws only. Details: gs-leaf.md.

### 1.2 OK-with-note — the display gain is not the bug (gpu.rs:18,39)

`GAIN = 145/128` (`SqueezeColourIn`/0x80, [L 0x91] per the code comment) and the 0x80→1.0 vertex
rescale apply to every draw equally, world included; the game applies the same rescale. Not a
contributor to the character/world difference.

### 1.3 MISMATCH (secondary) — missing alpha halving and the close-up flash variant (scene.rs)

Game: alpha obj+0x106 is `>>2` when `obj+0xF0 & 0x20000` and `viewer+0x236 == 1`; RGB gets
`× FlashMod` (float path) when the object or glist carries 0x80000, the viewer is within 4 units and
`DAT_002a38d4` is set [M 0x1e5888]. Port: neither is implemented. Minor (situational flash/fade
states), but part of the same function the port will touch for 1.1.

### 1.4 MISMATCH (secondary) — no dynamic-light pass for world objects (scene.rs)

Game: non-anim world objects get `Lights_CalcClosestLights` + `psiLight_SetLights(1,1,1,…)`
(`View_DrawObjList` 0x1e69a0:77-81) — dynamic lights (muzzle flashes, the searchlight spot's
psiLight node) brighten nearby objects via `TColor0/1/2`. Port: no equivalent (the muzzle light is
already G-026/G-033 open items). World **cels** are unaffected in both. Listing for completeness of
the 0x800 path: the 0x800 ambient multiply and the dynamic lights are separate terms of the same
matrix.

### 1.5 OK — per-batch leaf GS state (scene.rs, the DIRECT-write loop)

The port applies leaf `DIRECT` A+D writes per batch (alpha 0x42 / test 0x47 / zbuf 0x4e, with
trailing writes carried to the next leaf) — consistent with the game's "each DIRECT write applies at
its packet position" semantics documented in ps2gfx.md/render.md. No disagreement found this pass;
the sniper-gun transparency regression it mentions is not contradicted by the decomp reading.

## 2. searchlight.rs (G-036 follow-up — corrected by the atlas-5 audit)

The audit falsified the working hypothesis that the port's sweep channels are swapped. The game's
`obj+0x50` is a **polar/depression angle** (y = −sin) and `obj+0x54` the **azimuth**
(`Vec_Spherical_2_Cartesian` 0x1e3948 disassembled and re-derived; beam call site
0x1cbdbc–0x1cbdd0). The port's names (`pitch` for the 0x50-value, `yaw` for the 0x54-value) are
unconventional versus the game's internal semantics but internally consistent — the rendered beam
is identical. Verdicts below reflect that.

### 2.1 OK-with-note — sweep values match the game exactly (searchlight.rs:92–100)

Game [M 0x1cba68–0x1cbacc]: `obj+0x50 (polar) = sinf(phase·2π/480)·half + mid`;
`obj+0x54 (azimuth) = sinf(phase·2π/480 + π/2)·half + inst+0x14` (π/2 = `0x3FC90FDB` @0x1cba88;
an earlier "1.2450" reading was a float-decode error, caught by the audit — 0x3FC90FDB = 1.5707964).
Port: `pitch = sinf(phase·SWEEP)·half + mid` (:94), `yaw = sinf(phase·SWEEP + HALF_PI)·half +
base_yaw` (:95), both normalised to (−π, π] (:96–99) — the live heads store normalised values too
(head 016e03f0's obj+0x50 = −1.838 = 4.445 − 2π [L]). `base_yaw` = placement rot y = the stored
`inst+0x14`. Values match field for field; only the naming differs from the game's semantics.
Suggested (not required): a comment mapping port pitch/yaw to obj+0x50/obj+0x54 explicitly.

### 2.2 OK — tracking targets and clamps match (searchlight.rs:244–252)

Game [M 0x1cbaec–0x1cbc58]: azimuth (obj+0x54) steers toward `atan2f(dx,dz)` (store @0x1cbb70),
clamped to **inst+0x14±half** (@0x1cbb78–0x1cbbc8); polar (obj+0x50) steers toward
`−atan2f(dy−1, √(dx²+dz²))` (sqrt in the atan2f delay slot @0x1cbbe0, negated @0x1cbbe4, store
@0x1cbbfc), clamped to **mid±half** (@0x1cbc00–0x1cbc58). Port: `h.yaw` steers toward `dx.atan2(dz)`
and clamps to `base_yaw±half` (= inst+0x14±half); `h.pitch` steers toward `−(dy−1).atan2(√(dx²+dz²))`
and clamps to `mid±half` — the same channel assignments under the port's naming.

### 2.3 OK — beam construction matches (searchlight.rs spherical/align2dir use)

Game: direction `( r·sin(obj+0x54)·cos(obj+0x50), −r·sin(obj+0x50), r·cos(obj+0x54)·cos(obj+0x50) )`
[M 0x1e3948 disasm; call @0x1cbdcc]. Port: `spherical(r, yaw, −pitch)` =
`(r·sin(yaw)·cos(p), r·sin(p), r·cos(yaw)·cos(p))` with yaw ≔ the 0x54-value, pitch ≔ the 0x50-value
— identical operand for operand. The earlier "mid feeds pitch so the beam points up" MISMATCH was
an artefact of comparing names instead of values; G-036's "beam points UP" observation for head
016e95a0 is what the game itself renders (polar −0.1745 rad ≈ 10° above horizontal along +x) —
see docs/atlas/searchlight.md G-036.

### 2.4 OK-with-note — the 3-unit create link is right; the cel scoping differs (searchlight.rs:161–175)

Game [M 0x1cb6c0–0x1cb798]: walk **the own cel's object list**, skip class ≠ 0x22, link only if
`Vec_Dist3D < 3.0` strict, kind-slot pairing (`my.links[other.kind]` / other-links-me backfill),
oldest in-cel match wins, and the kind-2 −10000 move runs after the walk (order-dependent orphans).
Port: walks all parts created so far with the same 3-unit² test — equivalent on real layouts, but it
would also link two different lights' parts that sit within 3 units across a cel boundary (the game
would not). Low risk; note for the eventual cel-list port.

### 2.5 OK — 1 of 7 heads linked at level start matches the original [L]

Snapshot: exactly head 016e95a0 has links[2] = 016e9020; six heads have links[2] = 0 (incl.
016e03f0, and the measured orphan spot 016e8ec0 ≈1.5 units from head 016dfe70 — unlinked in the
original too, because the spot's create moved it before the head's walk). 29 class-0x22 objects in
the global object list [L recount during audit]. The port's create-time reproduction is correct at
level start. What remains open is late spot creation (only 2 spots exist at snapshot; their
create-walks would backfill the remaining heads) — mechanism unverified, carried in
searchlight.md / G-036.

### 2.6 OK — the 0→1 arm test is spot-only, in both game and port (searchlight.rs:~320)

Game [M 0x1cbdb0, 0x1cbf14–0x1cbf54]: the beam block reloads `s1 = links[2]` unconditionally
(links[0] only receives the flag/azimuth propagation @0x1cbd9c–0x1cbdac); links[2] == 0 skips the
whole block (`beqz $s1 → 0x1cbfc8`), and the transition tests `d ≤ 2.0` against the **spot**
position. The port tests the spot only — same behaviour. An earlier "the game prefers links[0] for
the arm test" note was a misreading of the beam block; retracted.

### 2.7 UNVERIFIED — state 2 decay target (searchlight.rs:255–259)

The port droops `h.pitch` toward 0.8 (+0.05/tick). The game's store offset in the state-2 block
(@0x1cbcfc–0x1cbd3c) was not re-pinned this pass. Under the corrected convention obj+0x50 ≔ port
pitch, so agent A's original "obj+0x50 decays toward 0.8" reading would make the port consistent —
but the store offset needs pinning when the state machine is next touched.

## 3. wire.rs (measured for wire-grapple.md; recorded here for the renderer-adjacent data)

### 3.1 MISMATCH — the wire's param fields are swapped/misnamed (wire.rs:16–19)

Port: `channel = placement.extra[2]`, "glist hashcode" = `extra[3]` (0xC510). Game [M disasm
0x198224–0x19824c; L inst 016dc510 = `00 00 02 00 00 00 00 00`]: inst+0x00 = param 1 = **wire type**
(0 hand-over-hand, 2 zipline), inst+0x02 = param 0, inst+0x04 = param 2, **inst+0x06 = param 3 =
switch channel**. The claimed 0x016D/0xC510 values do not exist in the live object.

### 3.2 MISMATCH — endpoints from the wrong geometry (wire.rs:25–32)

Port: placement translation + glist extent. Game: world sphere radius − 2.0 (grab) / − 0.5 (travel)
along yaw+π/2, zipline pitch from rot.z [M 0x1971f8–0x197208 etc.]. Also missing: type 2 zipline
(substate 0xF), the substate-6 vs 7 split (the port's "creep substate" comment is wrong), auto-drop
timer +0x8CC, regrab lockout +0x951, hang offsets 1.0207/1.17363, per-tick `Music_Event(0xD, 1)`.
Details: wire-grapple.md.

## Regeneration note

Regenerate whenever scene.rs/gpu.rs/searchlight.rs/wire.rs touch the audited areas: the atlas files
are searchlight.md, gs-leaf.md, cel.md, wire-grapple.md.
