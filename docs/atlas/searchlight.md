# The searchlight family (class 0x22 parts)

Plan v2 §2 struct 10. One `Searchlight_Create` (0x1cb4f8, 1028 bytes) call per part placement; the
placement's param 0 selects which part of a light it is. All parts share one 64-byte instance
(`control_create_object(0x40, …)`: size arg `addiu $a0,$zero,0x40` @0x1cb51c, `jal 0x1344e0`
@0x1cb534), instance pointer at `obj+0xE0`, class byte `obj+0xFF = 0x22` (`addiu $v1,$zero,0x22`
@0x1cb630/0x1cb65c, `sb` @0x1cb684) [M]. Port term for the placements: flags byte 0xDE
(`searchlight.rs` `part_placements`).

## The five kinds ([L] 07000005: 29 class-0x22 objects)

| kind (inst+0x00) | part | snapshot evidence |
|---|---|---|
| 0 | the projected light disc — moved onto each ray hit | r ≈ 0.37 spheres, repositioned by the update |
| 1 | the head (the sweeper) — the only kind `Searchlight_Update` advances | 7 on 07000005 |
| 2 | the spot / light source — gets a `psiLight` node, then hides at pos − 10000 on all axes until the first ray hit repositions it (`sub` @0x1cb8a0–0x1cb8cc) | 2 exist at snapshot time, both at −10000 offsets |
| 3 | the beam volume (large offset sphere) | r 6.1–16.1 |
| 4 | the bulb / glow | r 0.5–0.8 |

## Placement params → fields [M disasm 0x1cb554–0x1cb680]

| param | read | width | lands in | meaning |
|---|---|---|---|---|
| 0 | `level_tag+0x2C` (`lhu` @0x1cb554) | u16 | inst+0x00 | kind (table above) |
| 1 | `+0x30` (`lbu` @0x1cb5a0) | u8 | inst+0x34 | on/off switch channel; nonzero → `obj+0xF4 = 3` (starts off) (@0x1cb688–0x1cb69c) |
| 2 | `+0x34` (`lbu` @0x1cb5a8) | u8 | inst+0x35 | kill switch: raised ⇒ state 3, one-shot (`obj+0xFE |= 1`), light never re-lights (Update 0x1cb954–0x1cba14) |
| 3 | `+0x38` (`lbu` @0x1cb5b0) | u8 | inst+0x36 | stored, read by Update's entry gate (state 3 at entry) — exact effect unverified |
| 4 | `+0x3C` (`lw` @0x1cb5bc) | **s32 degrees** | inst+0x38/0x3C via half/mid | sweep bound |
| 5 | `+0x40` (`lw` @0x1cb5b8) | **s32 degrees** | inst+0x38/0x3C via half/mid | sweep bound |

Params 4/5 math [M 0x1cb5b8–0x1cb680]: `lo = (float)min(p4,p5)` (unsigned `sltu`/`movn`, then the
signed int→float dance `bltz → (x>>1)|(x&1)`, `cvt.s.w`), `hi = (float)max(p4,p5)`, both × DEG2RAD
(`0x3C8EFA36` built @0x1cb604–0x1cb60c), then
**`inst+0x3C = mid = (hi+lo)·0.5`** (`swc1` @0x1cb67c), **`inst+0x38 = half = (hi−lo)·0.5`**
(`swc1` @0x1cb680). They are integers in degrees, in either order.

## Instance field table (0x40 bytes)

| off | type | name | meaning | writers | readers | tag |
|---|---|---|---|---|---|---|
| +0x00 | u16 | kind | 0 disc / 1 head / 2 spot / 3 beam / 4 bulb | Create 0x1cb554 | Update 0x1cb94c (kind≠1 exits) | [M] |
| +0x04 | f32 | phase | sweep phase, `fmodf(phase+K, 480)`; starts at `(float)Rand_Rand(0x1e0)` ∈ [0,480) (jal @0x1cb594) — an RNG draw per part create | Update @0x1cbad0–0x1cbadc | state 0 | [M] |
| +0x10..0x1F | 4×f32 | quat | placement rotation; **+0x14 = the azimuth centre** of the sweep | Create @0x1cb598 region | Update (azimuth clamp/target) | [M] |
| +0x20..0x30 | 4×ptr | links[0..4] | the linked parts **indexed by the other part's kind**: links[0] disc/mount, links[2] spot, links[3] beam, links[4] bulb | Create walk (below) | Update beam block | [M] |
| +0x34 | u8 | on_ch | on/off channel (param 1) | Create | Update state 3 re-light | [M] |
| +0x35 | u8 | kill_ch | kill switch (param 2) | Create | Update kill path | [M] |
| +0x36 | u8 | entry_flag | param 3 | Create | Update gate | [M] |
| +0x38 | f32 | half | (hi−lo)·0.5·DEG2RAD | Create @0x1cb680 | sweep/track clamp | [M] |
| +0x3C | f32 | mid | (hi+lo)·0.5·DEG2RAD — **the polar centre** | Create @0x1cb67c | sweep polar, track clamp | [M] |

Header fields this family uses: `obj+0xF4` u16 state (0 sweep, 1 track, 2 broken, 3 off); `obj+0x50`
f32 **polar/depression angle**, `obj+0x54` f32 **azimuth** (see the convention below); `obj+0xFA`
bit 0x20 propagation to linked parts; `obj+0xD8` = cel arg, `obj+0xF0` = `cel+0x44`
(@0x1cb6a0–0x1cb6bc) [M].

## The two angle channels and the beam convention (settled during the atlas-5 audit)

`Searchlight_Update` keeps two angles on the head object:

- **`obj+0x50` — the polar / depression angle**, centre = `inst+0x3C` (mid);
- **`obj+0x54` — the azimuth**, centre = `inst+0x14` (the placement quat component).

The beam direction comes from `Vec_Spherical_2_Cartesian(out, a1, a2, r)` @0x1e3948. Disassembly
[0x1e3948–0x1e39e4; two dual-sine calls to 0x11f968, each adding π/2 (`0x3FC90FDB`) to both
incoming angles and storing sin(a+π/2) = cos(a) and sin(a)]:

```
out.x =  r·sin(a2)·cos(a1)
out.y =  r·sin(a1)          with a1 = −(obj+0x50)
out.z =  r·cos(a2)·cos(a1)
```

i.e. **out = ( r·sin(obj+0x54)·cos(obj+0x50), −r·sin(obj+0x50), r·cos(obj+0x54)·cos(obj+0x50) )**.
The beam call loads `$f14 = −obj+0x50`, `$f13 = obj+0x54`, `$f12 = 100.0` (@0x1cbdbc–0x1cbdd0,
`jal 0x1e3948` @0x1cbdcc) [M]. So `obj+0x50` is the polar angle with **y = −sin: 0 = horizontal,
+π/2 = straight down, −π/2 = up**; `obj+0x54` is the azimuth (0 → +z, π/2 → +x). The port's
`spherical(r, yaw, −pitch)` — `(r·sin(yaw)·cos(p), r·sin(p), r·cos(yaw)·cos(p))` — reproduces
exactly this with port-yaw ≔ obj+0x54's value and port-pitch ≔ obj+0x50's value: the port's naming
is unconventional versus the game's internal semantics, but the rendered direction is identical
(verified operation by operation against the disasm during the atlas-5 audit).

Head 016e95a0 carries mid = 6.1087 rad as its polar centre — normalised to (−π, π] that is
−0.1745 rad, i.e. the game itself aims this head ≈10° **above** horizontal, azimuth centre
inst+0x14 = 1.5708 (along +x). That is genuine game data, not a port defect (see G-036 below).

## Searchlight_Update (0x1cb900) — the state machine [M]

Gate: only kind 1 proceeds (`lhu` @0x1cb94c). Kill: inst+0x35 ≠ 0 and `switch_channels[0x26FC90][id]`
≠ 0 (@0x1cb954–0x1cb970) → `obj+0xF4 = 3`, `obj+0xFE |= 1`, links[2..4] cleared, params zeroed
(@0x1cb978–0x1cba14). Switch on `obj+0xF4` (`lh 0xf4($s3)` @0x1cba28):

- **0 sweep** @0x1cba68: `obj+0x50 (polar) = sinf(phase·SWEEP)·half + mid` (SWEEP `0x3C567751` =
  2π/480 @0x1cba68; store `swc1 0x50($s3)` @0x1cba98); `obj+0x54 (azimuth) = sinf(phase·SWEEP +
  π/2)·half + inst+0x14` (π/2 = `0x3FC90FDB` @0x1cba88–0x1cba90; store `swc1 0x54($s3)` @0x1cbacc);
  `phase = fmodf(phase + K, 480.0)` (0x43F00000 @0x1cbad0–0x1cbadc; K from literal pool @0x30FFD0D8
  — not read; the port notes the fmod store sits in the jal's delay slot, so the phase grows
  unbounded and the sine's own wrap keeps precision [M per searchlight.rs]).
- **1 track** @0x1cbaec: Bond obj from global `0x2DF88E0` (@0x1cbb08/0x1cbb10);
  **azimuth**: `obj+0x54 += 0.15·Vec_AngleDifference(obj+0x54, atan2f(dx,dz))` (gain `0x3E19999A`
  @0x1cbb40–0x1cbb48; atan2f 0x232e48 @0x1cbb50; AngleDifference 0x1e2b78 @0x1cbb5c; store
  @0x1cbb70), **clamped to inst+0x14±half** (@0x1cbb78–0x1cbbc8);
  **polar**: `obj+0x50 += 0.15·AngleDiff(obj+0x50, −atan2f(dy−1, √(dx²+dz²)))` (the `sqrt.s` sits
  in the atan2f delay slot @0x1cbbe0; negated @0x1cbbe4; store @0x1cbbfc), **clamped to mid±half**
  (@0x1cbc00–0x1cbc58).
- Shared beam block @0x1cbd94: the **mount/disc (links[0])**, if present, gets `+0xFA |= 0x20` and
  a copy of the head's azimuth (obj+0x54 → its +0x54) (@0x1cbd9c–0x1cbdac); then `s1 = links[2]`
  unconditionally (`lw $s1,0x28($s2)` @0x1cbdb0) — **the ray and everything after target the spot
  only**; links[2] == 0 → the whole block is skipped (`beqz $s1 → 0x1cbfc8`).
  `Vec_Spherical_2_Cartesian(&v, −obj+0x50, obj+0x54, 100.0)`; `v += headPos`; `Collide_RayIntersect`
  (0x1ec690 @0x1cbe28, cel = obj+0x20) → hit: move the spot to the hit point (stores to s1+0x30 and
  angles to s1+0x50/0x54 @0x1cbe90–0x1cbea8; matrix via 0x1e7c18 @0x1cbee0), `View_SetDrawInAllViews`;
  miss: `View_SetDrawInNoViews` (0x1e7c80 @0x1cbef0); `Collide_FreeHitList` (0x1ede00 @0x1cbef8).
- **0→1 transition** @0x1cbf14–0x1cbf54: `d = Vec_Dist3D(s1+0x30, Bond+0x30)` where s1 is the
  **spot (links[2])** — if `obj+0xF4 == 0` and `d ≤ 2.0` (`c.ole.s` vs 2.0 @0x1cbf40) ⇒
  `obj+0xF4 = 1`, spot `inst+4 = 0` (@0x1cbf50/0x1cbf54). An unlinked head (links[2] = 0) never
  arms — the block is skipped entirely. State-1 maintenance (dwell timer) follows @0x1cbf58+.
- **2 broken** @0x1cbc5c: every 7th frame (`(0x2A379C + obj+0xEC) % 7`) `Gas_Init` smoke
  (jal 0x1aacc8 @0x1cbcf0); an angle decays toward 0.8 by 0.05 (@0x1cbcfc–0x1cbd3c — store offset
  not re-pinned this pass; the port droops pitch, the atlas store list says obj+0x50: re-check on port).
- **3 off** @0x1cbd40: on-channel raised → `obj+0xF4 = 0`, `Sound_Play3D(0xE5, pos, 100.0, −1.0, …)`
  (jal 0x1dd548 @0x1cbd64–0x1cbd80) — the relay clunk. (Port also plays 0xE6 on break — not
  re-verified here.)

Update **never writes links[]** except clearing them on the kill path — linking is create-time only [M].

## Spot linking — the real rule [M 0x1cb6c0–0x1cb798]

At create, the new part walks **its own cel's object list** (`obj+0x20` = cel, `cel+0x8` = list head,
next via `obj+0x8`):

1. skip self (`beql $s0,$s1` @0x1cb6e0);
2. skip class ≠ 0x22 (`lbu 0xff($s0)` @0x1cb6e8, 0x22 in $s4);
3. link only if `Vec_Dist3D(me+0x30, other+0x30) < 3.0` — strict (`c.olt.s` vs $f20 = 3.0,
   0x40400000 @0x1cb6d0; `bc1f` @0x1cb708);
4. pairing is purely by kind slot, no pair id: my kind == 1 ⇒ `my.links[other.kind] = other`
   (`sw` @0x1cb728, index `inst+0x20 + 4·other_kind`); else other kind == 1 ⇒
   `other.links[my.kind] = me` (`sw` @0x1cb76c);
5. optional `View_SetDrawInNoViews` for dark heads' non-disc parts (@0x1cb744/0x1cb788).

The walk does not stop at a match — later (older, since `control_link_object_to_cel` prepends)
candidates overwrite, so the **oldest** in-cel match within 3 units wins. Order matters: the kind-2
epilogue (psiLight node @0x1cb79c–0x1cb89c, then the −10000 move) runs **after** the walk, so:

- spot created **after** its head: the spot's own create-walk sees the head in place and backfills
  `head.links[2]` (branch 4b);
- spot created **before** its head: its own create already moved it −10000, and the head's later
  walk never sees it — a permanent orphan. [L] measured orphan: spot 016e8ec0 sits ≈1.5 units from
  head 016dfe70 on 07000005 and is unlinked **in the original**.

`control_link_object_to_cel` (0x1346b8) is **not** part of this — it is the generic object→cel list
link (see cel.md); the earlier GAPS attribution was a guess.

## G-036 root causes (final, after the atlas-5 audit)

Both of B-9's claims dissolve under the corrected convention — **the port matches the game**:

1. **"The 3-unit link is wrong" — disproven.** The rule above is 3.0-strict, same-cel, create-time
   only, and the original snapshot shows exactly **1 of 7 heads linked at start** (head 016e95a0 ←
   spot 016e9020) [L] — the port reproduces the original at level start. The open question is not
   the radius but **late spot creation**: only 2 kind-2 spots exist at snapshot time; if the game
   spawns the other 5 later (spawner/script — mechanism unverified), their create-walks backfill the
   remaining heads. The port creates every part at load, so it can never reproduce that recovery.
2. **"mid = 6.11 points UP is a port bug" — misdiagnosis.** mid = 6.1087 rad is genuine game data
   [L], and it is the **polar** centre: normalised to (−π, π] that is −0.1745 rad, so the game itself
   aims head 016e95a0 ≈10° above horizontal (azimuth centre 1.5708 → along +x). The port's `sweep()`
   computes exactly the game's two stored values (its `pitch` ≔ obj+0x50's value including the
   (−π, π] normalisation the live heads show, its `yaw` ≔ obj+0x54's value with the same π/2 phase
   constant `0x3FC90FDB`), its tracking targets and clamps match (@0x1cbb70/@0x1cbb78–0x1cbbc8 and
   @0x1cbbfc/@0x1cbc00–0x1cbc58), and its `spherical(r, yaw, −pitch)` beam reproduces the game's
   `Vec_Spherical_2_Cartesian` direction operation for operation (see the convention section). The
   G-036 "implausible for a downward sweep" judgment assumed mid was a ground-sweep yaw; the
   placement just points this light at the wall/gate above the road.
3. What remains actionable for the searchlight scenario: the arm test is spot-only in both game and
   port, so the six unlinked heads cannot track **in the game either** until their spots spawn (if
   they do) — `searchlight-tracks-bond`'s removal was based on the misdiagnosis and can be
   reconsidered once the late-spot question is settled (trace or loader/spawner code path).

## Snapshot check [L] (work/ee_start_05.bin, file offset = EE address)

Head 016e95a0, inst 016e96b0, pos (−57.03, 37.84, −59.97):

| addr | raw | meaning |
|---|---|---|
| 016e96b0 | `00000001` | kind 1 (head) |
| 016e96b4 | `42be0000` | phase = 95.0 — inside [0,480) as `fmodf` requires |
| 016e96c4 | `3fc90fdb` | inst+0x14 = 1.5708 — the azimuth centre (π/2, along +x) |
| 016e96d0/d8/dc/e0 | `016e9440 / 016e9020 / 016e92e0 / 016e9180` | links[0] disc, links[2] spot, links[3] beam, links[4] bulb |
| 016e96e4 | `000000a4` | inst+0x34 = 0xa4 (on-channel 164); +0x35/+0x36 = 0 |

Head 016e03f0, inst 016e0500, pos (59.80, 46.69, 53.57): kind 1, phase 321.0, **links[2] = 0** (no
spot — one of the 6 unlinked heads), half 0.6109 (35°), mid 4.9742 (285°) ⇒ params 4/5 ≈ (250, 320);
its live obj+0x50 = −1.838 = 4.445 − 2π (the normalised polar store the port matches [L per
searchlight.rs]). Spot obj 016e9020 inst 016e9130 pos (−10057.84, −9963.38, −10059.73) — the
−10000 offset, its head is in state 3 (off), so no ray has repositioned it yet. Cross-check of
half/mid: head 016e95a0 half 0.0873 (5°), mid 6.1087 (350.06°) ⇒ params 4/5 ≈ (345, 355) —
integers in degrees, either order.

## Not verified this pass

- The `fmodf` addend K (literal pool 0x30FFD0D8 unread); the game-side store(s) that normalise
  obj+0x50/0x54 into (−π, π] (the port claims [L]-pinned on five heads — the live −1.838 store for
  head 016e03f0 confirms the game normalises too; the store location in the Update tail
  0x1cc050–0x1cc308 was not read).
- The state-2 decay's exact store offset (@0x1cbcfc–0x1cbd3c; agent A's first read said obj+0x50
  → toward 0.8 by 0.05, which would match the port's drooping `h.pitch` under the corrected
  convention — but the offset needs pinning).
- The psiLight node layout (node+0x3C = 500.0, +0x40 = 1.0, +0x44..0x47 = ff/ff/ff/2, +0x48 = 1
  observed @0x1cb89c region) and `Rand_Rand(0x1e0)`'s exact range.
- What creates the missing 5 spots late (if anything) — see G-036 above; needs a trace or the
  loader/spawner code path.
