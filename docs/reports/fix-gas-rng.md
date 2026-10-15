# fix-gas-rng (G-033) — the gas puffs' RNG consumption

Branch `s/fix-gas-rng` off master 96405b35. Files: `crates/nf-game/src/gas.rs`,
`crates/nf-game/src/effect.rs` (one call + the ricochet sister's record),
`crates/nf-view/src/scene.rs` (one call + comment), `crates/nf-view/tests/fix_gas_rng_live.rs`
(new), `docs/systems/gas.md` (new), this report, WORKLOG.

## What the game does (the doc entry this implements)

`Gas_Init` 0x1aacc8 draws from the shared generator on EVERY puff, before its kind dispatch:
the spin `Rand_FRand(REC_FRAME_RATE · π/2)` (jal 0x1ab060) and the sign coin `Rand_Rand(0x100)`
(jal 0x1ab08c, ≥ 0x80 negates). The velocity branch adds: kind 7 with a NULL velocity pointer →
`Env_GetWindVel` + `Rand_FRand(0.2)` ×3 (0x1ab160/0x1ab184/0x1ab1a4); kind 4 with a NON-NULL
pointer → `Rand_FRand(0.05)` ×2 (0x1ab21c/0x1ab248); all other arms draw nothing. The whole body
is gated on `cel != 0 && life != 0` (:31) — a 0-life call draws nothing. ABI note: the decompile
hides that integer args 7-10 ride t0–t3 and the life is the stack word (`lhu 0xc0(sp)` 0x1aad24) —
the muzzle site's life is 0x80 = 128 ticks, not the guessed 8 (full write-up in the new
docs/systems/gas.md). Per-site totals: the muzzle puff = 6 draws (the caller's quaternion-z
`Rand_FRand(2π)` 0x1a43ec + 2 + 3 wind), the searchlight's = 6 (already consumed by
fix-searchlight-remainder), an `Effect_Bullet` impact entry = 2 (+3 for kind-7 entries; the NPC
owner gate `Rand_Rand(2)` 0x1ce94c draws for non-player shooters only).

## The fix

- `GasPool::init_muzzle_smoke(rng, pos)` (gas.rs): the muzzle site's fixed args [M disasm
  0x1a4404-0x1a442c] — scale 0.05→0.1, kind 7, SMOKE_QUAD, rgba 0x48403080 with end bytes
  (0,0,0,1), life 128 — consuming the 5 internal draws in the game's order from the sim RNG
  (the scene keeps its existing 2π draw one slot ahead of them, unchanged order).
- `GasPool::init_effect_puff(rng, …)` (gas.rs): the `Effect_Bullet` impact loop's real-stream
  form with the game's NULL-velocity dispatch (kind 7 → wind draws; 9/10 → the position-copy arm
  with kind 10's life halving; everything else incl. kind 4 → the rising default (0, REC, 0), no
  extra draws; kind 0xb quarters the spin). effect.rs's impact loop now calls it; the pre-fix
  call passed a zero vector as a stand-in for the game's NULL pointer and kind-4 entries wrongly
  took the jitter arm.
- The draws run BEFORE the pool-cap check (the searchlight precedent) and the rgba rates use the
  game's reciprocal form. The remaining counter-hash sites (`init`/`init_ex`: drone.rs's kind-5
  puffs, the 0xb crate puff, world_fx ripples) are untouched — each is its own game call site
  whose caller-side draws aren't re-derived yet (table in docs/systems/gas.md).

## Reproduce / verify

- Fail-first (shown): `git checkout HEAD~1 -- <the 3 sources>` + run
  `fix_gas_rng_live` → both tests FAILED (the puff signature: pre-fix life 8.0, counter-hash
  fields no stream offset reproduces; the sister's delta was 0 ≠ 6). Restored → 2/2 green.
- `a_muzzle_puff_advances_the_sim_rng_by_exactly_the_games_count` (07000005, real input path
  `nf_view::load` + `Sim::step_bond` with the trigger): the puff exists with the game's args, and
  the 6 draws [2π, spin, sign, 0.2×3] replayed on the pre-shot snapshot at a fixed offset k
  reproduce the puff's spin+vel bit for bit (k = 1 — only the muzzle light's draw precedes them).
  No PCSX2 trace exists in work/ (checked: no `work/*trace*`), so the sequence check is the
  twin-replay form.
- Sister `the_no_smoke_gate_makes_no_puff_and_no_smoke_draws`: the identical shot with the
  weapon's 0x2000 bit set (MuzzleFlash 0x1a4298:92's skip arm) → no puff, and the tick drew
  exactly 6 fewer draws (smoke 22, gated 16, delta 6). Bond is teleported 40 units up first so
  the round's first-tick ray (the `max(1.0)` flight step) meets no geometry — an impact puff's
  draws behind the muzzle draws would coin-flip between the runs.
- Unit (gas.rs, NF_ISO-free): the muzzle puff's 5 internal draws replayed on a twin stream bit
  for bit + the exactly-5 count (4/6 land elsewhere); the impact kinds' dispatch and counts
  (kind 7 = 5, kind 0 = 2, NULL-vel kind 4 = the default, 0xb quartered, 0-life draws nothing).

## Suite

Foreground chunks (this sandbox reaps background jobs — the suite runner with `--only` lists;
every chunk's FAILED lines checked):

- nf-game: **170 passed / 0 failed** (after updating `ricochet_draws_once_and_reflects`'s sister —
  see "expected stream shifts" below).
- nf-view + nf-audio + nf-formats, by chunks: first 38 binaries all ok (incl. `fix_gas_rng_live`,
  `walk_07000005_live`, `weapon_matrix_live`, `fix_weapon_fire_modes_live`, `fix_actors_live`);
  chunk A 105/2, chunk B 130/0, chunk C 144/0, chunk D (31-level walk sweep) 30/0, chunk E 119/0,
  chunk F 71/3, stragglers 5/0.
- Total **774 passed / 5 failed / ~18 ignored**. The 3 `disc_golden` failures are the allowed
  slim-ISO ones. The 2 real failures are the analyzed flips below.

## Expected stream shifts (the 2 failures, investigated)

1. `nf-game ai_live::guard_takes_cover_like_the_game`: the guard still walks the same state
   sequence (0x58, 0x5a, 0x8c, 0x93, 0x94, 0x95, 0x96 ✓) and covers at node 1, but
   `UnderCoverAim` lasted 90 ticks vs the live 37 (window 32..=40). Mechanism: the scenario's
   shots now impact with the game's draw count — every impact puff consumes 2 (+3) shared draws
   and the NPC impact coin `Rand_Rand(2)` 0x1ce94c draws even when it kills — so the guard's
   fire-roll stream shifted and its aim phase stretched. The port's stream is now draw-count
   exact at every ported site; the pinned window was calibrated on the pre-fix (draw-short)
   stream.
2. `nf-view fix_guard_weapons_live::the_fire_rate_matches_the_game_interval_clamp_and_burst_cut`:
   the candidate guard stalled its shot collection (the fire-interval rolls shifted the same
   way). Same mechanism.

Both need a PCSX2 side-by-side re-calibration (the live checks below) — I did not loosen other
tasks' calibrated windows.

## Live checks for the main agent

1. PCSX2 + gdbwatch, 07000005 courtyard, pistol: put a watchpoint on the generator state
   SEED_X/SEED_Y (0x30d0a0/0x30d0a4), fire ONE shot into a wall, count the advances across the
   tick. Expected: +6 for the shot when the round's impact makes puffs beyond the muzzle's own 6
   (the impact total depends on the surface record's gas entries — the muzzle's 6 is the floor).
   This settles whether the port's per-shot count now matches the game's.
2. PCSX2 side-by-side for `ai_live::guard_takes_cover_like_the_game`'s scenario: measure the
   game's UnderCoverAim duration on the current build. If it measures ~37 while the port (now
   draw-exact) gives 90, the residual drift is in the sites the port deliberately does not
   consume (the flash-sprite branch 0x17925c-0x1792b0, the fire-script op's puff 0x1c1b38) —
   the report's table in docs/systems/gas.md lists them.
3. PCSX2 + fieldprobe, muzzle puff: poke a shot from the hip and read the fresh puff obj's
   data+0x38 (life) — expected 0x80 = 128 (the decompile-dropped stack word), and obj+0xE8
   (scale) ≈ 0.05.
4. nf-view render (`--bond` spots): fire one shot and hold: the muzzle puff should read as a
   small dust quad growing 0.05→0.1 over ~2 s at the muzzle, fading; the impact puffs on stone
   should hug the wall and rise only slightly (their velocity is now the game's ≤ 0.008/tick
   wind spread, not the old counter-hash stand-in ~60× larger).

## Left

- The remaining counter-hash sites (docs/systems/gas.md's table): the drone.rs kind-5 puffs,
  the 0xb crate puff, the world_fx ripples — each needs its game call site's caller-draw
  re-derivation (Copter/GT/RB/Effect_Laser/Ripples_Update) before it can join the real stream.
- The drone fire-script muzzle op (0x1c1b38:445-453, 6 draws) is not ported at all.
- The two AI re-calibrations above.

## Final tree

origin/master 404d7dc1 merged (WORKLOG conflict resolved keeping both; gas.rs/effect.rs/scene.rs
auto-merged clean). Re-runs on the merged tree: `fix_gas_rng_live` 2/2, the nf-game chunk
**172 passed / 0 failed**. Branch pushed at this state.
