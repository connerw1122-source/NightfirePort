# fix-sky — the sky remainder: lightning, the rotating sky, the sky pass's GS state

Branch `s/fix-sky`, finish id 260. Doc: `docs/systems/render.md` "Sky remainder" (every fact below is
there with its address); GAPS G-019 updated.

## What's done

**The lightning machine** — new `crates/nf-game/src/sky.rs`, ported operation-by-operation from
`View_DrawSky` 0x1e5ad0 with the EE chop arithmetic (`ps2float`) and the game's own `PS2Sinf`:

- A type-0x2A placement with parameter 1 ≠ 0 is the **flash variant** of its slot; parameter 2 is the
  lightning interval (stored as `param2 × FRAME_RATE`, [L] 0x30d0d0 = 60.0), parameter 3 the flash
  length, parameter 4 the sky group that `View_DrawSky`'s argument selects (1 = pre-world, 0 =
  post-`psiPostSolid`) [M parsemap 0x1d04f0 — the placement's parameter words land at level+0x2c..].
- `tick_slot` reproduces the timers and the RNG order exactly: a flash-start tick draws
  `Rand_Rand(interval)`, `Rand_Rand(100)`, and on the non-reflicker branch `Rand_Rand(120)`; a flashing
  tick one `Rand_Rand(100)`; dark ticks none. The dark-timer test `(int)(x << 16) < 1` fires on **0 and
  1** (the u16 wrap), so a fresh slot flashes on its first frame — then thunder after `Rand_Rand(120)+30`
  ticks (sound 0x5d) or 10 ticks for reflicker-type flashes (sound 0x5e, `LightningType` 1).
- `SuperHappyLightningFlag` is raised during reflicker-type flash ticks and cleared per draw call; in the
  game it shows the 8-beam lightning polyline (`Lightning_Update` 0x18cf68, `FractalThing` re-randomises
  the shape on `Reflicker`). **The flash does not light the level**: nothing writes the ambient and
  nothing calls `Light_Create` anywhere in the lightning path — the level-side effect is the beams only.
  The port exposes `sky::Sky.globals.super_happy` as the marked hook for 050-fix-lighting.
- Disc parameters [D, scanned all 27 levels with `nf-extract placements <level> 2a`]: lightning exists
  only on **0700000a** (slot 3, "Thunderhead": interval param 24 → 1440 ticks, flash length 5) and
  **0700000b** (slot 3, 30 → 1800, flash 10). The three EE snapshots' GfxLists [L] confirm the slot
  layout and that the snapshot levels have no flash variants.

**The rotating sky of 0x0700001B**: `rotatosky.168` [0x30d878] advances once per slot pass whose glist
flags lack bit 0x8 — both of 0700001b's slots (SKY_StarField, SKY_Earth, flags 0x222 [D]) qualify, so
**+2 counts per unpaused frame**. The spin angle is `count × 0.00069444446` (= 1/1440 rad; ≈ 75 s per
revolution), concatenated **outside** the slot rotation: spin · slot · camera, via `RotMatrix` 0x1e2368 /
`Concat` 0x1e4a90 (B := A·B) / `ApplyMatrixLVI` 0x1e4960 — all ported in `sky.rs`. The same block
recomputes the sun vector 0x2d88a0 each pass with the hardcoded angles 0x409f2c8e / 0xbf29c920
(285.00002° / −38.00000°) and rotates it through the same matrix.

**The sky pass's GS state**: the pass opens with `psiSetUpColourBlend(2)` → `AddDMAChainRegSet(0x2a, 0x42)`
[M 0x1214f8] — ALPHA_1 = 0x2a = (0−0)·FIX0 + Cs = **replace, no blend** — plus no tint, no lights, and
`psiFog` is a **stub** (0x121578, size 8; its two `Game_Draw` calls do nothing), so the sky pass writes no
fog register. Disc check (`crates/nf-view/tests/sky_gs_live.rs`, NF_ISO): the 0700000a Thunderhead's first
CNT carries **no A+D writes at all** — it inherits the pass state (0x2a / TEST_1 0x5001b) — while
0700001b's SKY_StarField writes its own ALPHA_1 0x44 (normal blend), TEST_1 0x5001b and ZBUF_1
0x1_31000000. gpu.rs's pass-0/pass-4 pipelines (Always/LessEqual, no depth write, pass-4 far pin) are kept
as modeled; see the live checks for the one open depth question.

**Snow and rain** are *not* sky draws: `DrawDrops` 0x18cb90 runs after the alpha list with blend 2. The
physics/density already live in `nf_game::env.rs` (from a previous task); the look facts are now in
render.md (sprite glists 0x2000290 + the darker 0x2000039 for snow on 07000009-b tinted
(0x24,0x20,0x20,0x20); `psiDrawParticleList(0.2f, argb 0xff808080, glist 0x3000045/0x3000087)` for
types 0/2; max 4096/2048/128-or-64).

## How it was verified

- `crates/nf-game/tests/sky_lightning.rs` — 600 ticks with the real Thunderhead parameters against a
  golden produced by an **independent replica** of the game's arithmetic (`scripts/sky_golden.py`,
  not committed: EE chop add/mul, PS2Sinf, Rand_Rand — the last already proven bit-exact against the
  running game in `rng_live.rs`); plus a short-interval run covering every branch (both thunder sounds,
  reflicker, SuperHappy) tick-by-tick, with sisters: the exact RNG draw count, the final RNG state
  (x=0x3d72941b, y=0x70544b60), the no-flash slot consuming zero draws, interval × FRAME_RATE truncation.
- `crates/nf-game/tests/sky_rotation.rs` — the spin angle's f32 bits per pass (counts 1, 2, 60, 1200,
  2881), the pause/level gates (frozen counter when paused, `None` on other levels), and the sun vector's
  bits before and after the full game-side matrix chain (identity camera, slot rotation [0,0,0] — whose
  `RotMatrix` is *not* exactly the identity: `sinf(π/2)` = 0x3f800001, so m00 = 1.0000002; the port
  reproduces that).
- `crates/nf-view/tests/sky_gs_live.rs` (NF_ISO) — the first-CNT A+D census above.
- Full suite: `NF_ISO=$NF_ISO cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio
  -p nf-view -- --test-threads=1` — **interrupted by this session's deadline** (the sandbox kills
  background runs; the foreground re-run could not start before expiry). The full sky test set passed in
  the foreground together with `rng_live` (9 tests: sky_gs_live 1, sky_lightning 4, sky_rotation 3,
  rng_live 1). The changes are additive (one new module, one `pub mod` line, new test files, docs), so
  the merge-time full suite is the remaining gate.

## What's left

- **The render-side hook in nf-view** (no GPU in the sandbox): swap a slot's material to its flash glist
  on `drew_flash` ticks, apply `rotating_sky_matrix` for 0700001b, and draw the lightning bolts when
  `super_happy` (the bolt geometry needs `FractalThing` 0x1a94d8 / `PositionBeam` 0x1a9400 ported). The
  data is all in `sky.rs`; the scene build needs the normal/flash material pairing per slot.
- The 0x0700001B sun vector is computed but not yet consumed by the lighting path (050-fix-lighting owns
  it; `ROTATING_SKY_SUN_ARGS` + `rotating_sky_matrix` are the hook).

## Live checks for the main agent

1. `tools/trace/gdbwatch.py` on **ZBUF_1 (0x4e writes)** during 07000001b's sky draws: expected — the
   StarField's ZBUF_1 0x1_31000000 [D sky_gs_live]; confirm whether bit 20 (ZMSK) is set while the sky
   primitives raster (i.e. does the rotating sky write depth?), and what ZBUF_1 the Thunderhead inherits.
   The port's sky pipelines assume no depth write on both sky passes.
2. Screenshot comparison spots (PCSX2, 0700000a from a fresh level start): the first flash lands on the
   first frame (the fresh-slot rule) — the sky brightens for ~5 ticks with ~50 % flicker, thunder sound
   0x5d at frame ~142; then quiet for ~23 s. `--bond` spot: 100,20,300 yaw 0.9 pitch 0.1 (the Thunderhead
   quadrant, slot 3).
3. 0700000a vs 0700000b: same slot 3 machine, interval 1440 vs 1800, flash 5 vs 10 — the b-level flashes
   are longer and sparser.
4. 0700001b: the starfield/earth should visibly rotate ~4.77°/s (one revolution ≈ 75 s); a 10 s timelapse
   shows ≈ 48°. `--bond` 0,0,0 yaw 0 pitch 0 with the level start camera.

## Files

`crates/nf-game/src/sky.rs` (new), `crates/nf-game/src/lib.rs` (one `pub mod sky;` line — minimal shared
file touch), `crates/nf-game/tests/{sky_lightning.rs, sky_rotation.rs, sky_golden_short.txt}`,
`crates/nf-view/tests/sky_gs_live.rs` (new), `docs/systems/render.md` (Sky remainder section),
`docs/GAPS.md` (G-019 only), `docs/WORKLOG.md` (fix-sky entries appended), this report.
No game bytes, tokens, URLs or scratch outputs committed.
