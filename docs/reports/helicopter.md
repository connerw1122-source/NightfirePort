# helicopter — report

Branch `s/helicopter` (from origin/master 31dd6161). The user, 2026-10-16: "we still havent added
the helicopter". The helicopter actor is now ported: its spawn, its real spline flight, its
attacks, its damage and destruction, its sounds — docs first (`docs/systems/helicopter.md`),
the state + getters for the draw code, tests with failing sisters.

## What's done

1. **The inventory** (`docs/systems/helicopter.md` §1, the probes `tests/probe_helicopter*.rs`):
   five single-player copters — 07000005 map50 #241 (gate 0, flies from load) and #242 (gate 0x42),
   07000008 map51 #45 (gate 5, the extraction Hind), 0700000b map50 #328 (gate 0x2A, the roof
   hover with the parachute), 07000012 map38 #707 (gate 0x25, the attack Hind) — plus the part
   templates (05/08/0b/12's map-level `0x2F`s: the gun Hind's Rotor/TailRotor/Gun/MuzzleFlash/
   MissileLeft/Right parts, 0x12's RocketPod set) and the MP pad copter (07000046). The static
   helipad props (0x19-kind) are scenery, not the actor.
2. **The data path, decompile-verified**: `parsemap_block_map_data_dynamic` (0x1d1b68) builds a
   level tag with the placement's params at `tag+0x2c + 4·k` — that is what `Copter_Create`
   (0x17d340) reads (the decomp's `param_3` is the TAG, not the record; the disasm 0x1d0b08-14
   passes `s1`). The framelist table = the main map's 0x19 blocks (`parsemap_block_path_data`
   0x1d24d0 — the port's existing `Level::path_blocks`, already EE-verified for the doors),
   indexed by `extra[8]` low; the key count = `extra[7]` low (measured equal to the block's keys
   on all five [D]); the per-tick advance = count/divisor with the divisor = param 10 or
   `extra[7]` high (routes take exactly `divisor` ticks [M 0x17d340:33-41 + GS_SetRefreshRate
   0x17d2b8]); the wrap-back = param 3 (0x08's #45 loops to key 684); the +0x74 == 100 wrap latch
   [M 0x17d740].
3. **The actor** (`crates/nf-game/src/copter.rs`, wired in `drone.rs`): `Copter_Move` 0x17d628
   through the ported `Spline_Interp` (`door::interp` — one fix: the key index clamps to the list
   when `last` = the key count, the copter's case); `Copter_Update` 0x17dbb8's machine — the gate
   early-return, the countdown pulse, state 0's route count, state 1's cone spot (COPT_COS/SIN =
   0.70710678 [M 0x17e468's static ctor: PS2Sinf(3π/4), PS2Sinf(π/4)]; the 1e6 square-unit radius;
   `Collide_LineOfSight`), state 2's attack: `Copter_FireGun` 0x17d828 (the 4-shot turret rhythm,
   weapon 0x71 / 0x61 on 07000008, `Sound_Play3D(100, muzzle, -1, -1, 0x5df)` [M :85]) and
   `Copter_FireMissile` 0x17d9f0 (every 60·DiffMod ticks, the pod by the shared RNG, weapon 0x6d /
   0x62 on 07000008, the direct-aim roll on 07000008); the Bits damage [L ELF 0x2a37e8: mult
   2/3/1/2/100/1, Bits[4] filter 16] gathered through the real bullet pass and consumed per tick;
   the destruction = the ONLY +0x72 writer [M cases 4/5/6]: state 4/5 (the death-segment switch)
   or the 07000012/0700000b `Explode_Create` arm (the port's shared blast queue, script 0x6000052).
   The getters: `pos()`, `matrix()`, `mesh_hash`, `visible()`, `state`, `damage`, `t`, `wrap_flag`.
4. **Two tests corrected to the decompile** (the task's rule): fix_objects2's gondola test and
   campaign_flow's 08 chain pinned "the copter's arrival writes ch 0x4B at the flight script's
   length (299 ticks)" — that was the [S] the decompile contradicts (cases 4/5/6 — the
   destruction — are the only +0x72 writers; the flight never writes it). Both now drive the
   shoot-down through the real bullet pass and pass, including the mission-complete chain
   (0x4B → mux #276 → ch 100 → the level ends into 0x0730008).
5. **ai_live's seed re-picked 4 → 13** (the repo's established practice at RNG-stream merges):
   the copter's decompile-mandated `Rand_Rand(10)` at the cone spot joined the shared stream on
   07000005 (#241 flies from load). `difficulty_gates`'s `Copter::create` calls take the new
   path-table arg.

## How it was verified

- `helicopter_live` 4/4: the 05 patrol copter rides its framelist (the body = `Spline_Interp` at
  the record's own playhead; the sister pins the parked pre-gate state on 0b); the 0b copter
  parks until its gate (the sister: no start, no movement for 300 ticks); the 0x12 Hind spots
  Bond inside the 45° cone and fires the game's weapon ids (the sister: no fire outside the
  cone); the 0b copter is destroyed through the game's damage (the real bullet pass → the Bits
  sums → hp 299 → the case-6 arm → ch 0x66; the sister: below the threshold no channel).
- `fix_objects2_live` 5/5 (the gondola chain with the shoot-down), `campaign_flow_live` 5/5,
  `walk_07000012_live` 4/4, `enemies_07000012_live` 3/3, `walkreal_0700000b_live` 1/1,
  `fix_missing_objects_live` 5/5, `fix_drone3_live` 3/3, `oddjob_hat_live` 2/2,
  nf-game's full unit + live set (ai_live's 36 after the re-seed, difficulty_gates 3/3, the lib
  251) — all green.
- The full fast suite (`tools/test/suite.py`) was started on the final tree in the background
  near the end of the window; it did not finish before the sandbox deadline — the crates the
  task touches were run directly instead (the list above). The 3 `disc_golden` failures are the
  slim-ISO allowance.

## What's left

- **The per-part damage routing**: the game attributes each part's hit chain separately
  (`Bits[i].mult` per part); the port gathers through one body sphere at the record's spline
  position with the max part radius (the flight script's part entities play on a static base in
  the port, so their world poses are not the game's). The Bits × hp math and the destruction are
  exact; the per-part sphere routing needs the flight script's base to ride the copter's matrix
  (the game's `SP_SetPosRot(sp, obj+0x90)` [M 0x17d628]) — the same change makes the port's
  copter visuals follow the spline instead of the placement. [S] documented in the docs §4.
- **The visuals**: the draw code consumes the getters (`pos`/`matrix`/`mesh_hash`); the main
  agent's render pass should compare each copter against PCSX2 (`--bond` spots in the docs §3).
- The smoke-puff arm of the damage scan (`Gas_Init` at >50% damage, 0x17e2xx) is not ported
  (visual; documented deviation).
- Live checks for the main agent: (1) on 0700000b after ch 0x2A, watch the copter's
  `Copter_Update` record (+0x54 path node, +0x5c t) advance ~0.281/tick and the body's
  `obj+0x90` translation follow the framelist; (2) on 07000012 with ch 0x25 set and Bond in the
  cone, confirm `Copter_FireGun`'s `Sound_Play3D` id 0x5df and the bullet's weapon word 0x71;
  (3) the 07000008 destruction: shoot the Hind down, confirm `switch_channels[0x4B]` writes at
  the death (not at the flight script's end).

## Usage

not visible to me (no token, cost or context-window figures are shown in this environment).
