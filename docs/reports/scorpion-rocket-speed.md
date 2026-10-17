# scorpion-rocket-speed — the AT-600 Scorpion's rockets fly at the game's speed

Branch `s/scorpion-rocket-speed` (origin/master bee8c129). Playtest 1016-1355 (the user, on master
2026-10-16, multiplayer): "Scorpion rockets are way too slow." The Sentinel's flight scale was
already measured live (×4.04 on the 0x2c/0x2d, fix-sentinel-camera-play); this task checked the
Scorpion records 0x2E/0x2F the same way and set them to the game's numbers.

## The evidence

The record values, from the live table (`weapon_data`, EE 0x2bf150, 0x10C bytes/record
[docs/fields/weapon_data.md]) — identical in all four snapshots (`ee_start_05.bin`,
`ee_start_07000001.bin`, `ee_start_0700000a.bin`, `ee_snapshot.bin`) [L]:

| w | name | +0x20 speed | +0x6C | shape |
|---|------|-------------|-------|-------|
| 0x2c Sentinel Guided | 0.25 | 0x00012204 | the 0x200 quarter, the 0x4 guided cam, the 0x10000 swoosh, the 0x2000 light |
| 0x2d Sentinel Unguided | 0.333333 | 0x0001a200 | the 0x200 quarter, the 0x8000 spin, swoosh, light |
| **0x2e Scorpion Heatseeker** | **0.25** | **0x00032200** | the 0x200 quarter, the **0x20000 homing**, swoosh, light, **no 0x4** |
| **0x2f Scorpion Unguided** | **0.333333** | **0x0001a200** | **byte-identical to the measured 0x2d's** |

The flight mechanics [M]:
- init speed = `weapon_data[w]+0x20`, ×0.25 when `+0x6C & 0x200` [`Bullet_init` 0x12c5a0:187-191];
- the per-tick step = the speed, clamped at the record's range `+0x1C` [`Bullet_update`
  0x12e7b0:157,380-387]; the first tick steps (0.5 + FRand 0.5)·speed [0x12e7b0:116-120];
- homing runs for `+0x6C & 0x20000` after the move and steers the DIRECTION only —
  `dir += clamp((to − dir)·0.0333, ±0.01)` per component, normalised; it never writes BU+0xF8
  [`Bullet_homing` 0x12f358:126-166];
- the guided cam is init-time only [`Bullet_init` 0x12c5a0:234-254].

So the ×4.04 flight scale measured on the Sentinel twins is the Scorpion twins' too: 0x2f's
`+0x6C` is byte-identical to the measured 0x2d's, and 0x2e differs from the 0x2c only by homing
(0x20000) instead of the guided cam (0x4) — neither bit is read by the mover. Predicted game
flight: **0x2e = 0.25 × 0.25 × 4.04 = 0.2525 u/tick**, **0x2f = 0.333333 × 0.25 × 4.04 =
0.336667 u/tick** (the 0x2d's measured 0.33666 — its flags are identical [L]). The port flew
them init-only (0.0625 / 0.0833) — 4.04× slow, the playtest's complaint to the digit.

## The fix

`crates/nf-game/src/drone.rs`, `fire_player` — the measured ×4.04 flight scale now covers
`0x2c | 0x2d | 0x2e | 0x2f`, with the full evidence chain cited inline. Nothing else needed
changing: the port already models the 0x200 quarter (fix-fire-modes3), the 0x20000 homing
(fix-weapon-modes), the first-tick half step, the range clamp and the gravity arm
(`update_player_bullets` / `PlayerBullet`).

## Verification (failing first)

`crates/nf-view/tests/scorpion_rocket_speed_live.rs` — through the REAL input path (the give,
the switch and the fire ride the window's `Input` lanes through `Sim::step_bond`), like the
sister `sentinel_camera_live.rs`:

- `the_scorpion_rockets_fly_at_the_game_flight_speed` — **on the pre-fix src it FAILED**:
  "the 0x2e rocket flies at 0.06248 u/tick, the game's flight speed is 0.25250". Post-fix:
  0x2e = 0.2525, 0x2f = 0.336667 u/tick (±5% window), and the pins ride along: the 0x2e bullet
  carries `homing: true` / the 0x2f `false` [L +0x6C], and neither twin rides the camera
  (`guided_cam: false` — no 0x4 [L]).

An existing test contradicted the game's numbers and was fixed:
`crates/nf-view/tests/scorpion_modes_live.rs` (scorpion-modes-12) was calibrated on the 4.04×
SLOWER pre-fix flight. Telemetry (a scratch run, not committed): at the game's own speed the
old scenario (24° off-axis @ 18 units) passes the guard by ~0.9 units — the closing sweep beats
the ±0.01/component steer in the last units [M Bullet_homing 0x12f358:126-166], so the game's
own kinematics miss there too; the pursuit rides a residual ~17° equilibrium (the dot-to-target
stays ~0.956) while the heading itself swings 30°+. Recalibrated: 12 units at 17° (inside the
0.66-dot acquisition cone) converts — the rocket locks, curves (the heading rotates 0.59 rad by
the hit) and kills the guard. The curve detector changed from "dot-to-target rises > 0.03"
(full convergence — a slow-flight artefact) to "the world-frame heading rotates ≥ 0.15 rad"
(the spread contributes ≤ 0.0028 rad [L +0x24 = 2 × 0.0014], no gravity bit [L +0x6C], so the
rotation is the homing's [M 0x12f358:159-166]). The lock and hit asserts are unchanged. No game
pin was weakened.

## What was run

- The new test + the sisters, post-fix, all green: `scorpion_rocket_speed_live` (1),
  `scorpion_modes_live` (1), `sentinel_camera_live` (3), `fire_modes3_live` (5),
  `fix_weapon_fire_modes_live` (5), `fix_weapon_sounds_live` (3),
  `fix_rocket_sound_leak_live` (2), `guided_rocket_sound_stop_live` (2) — 22 passed, 0 failed
  (+ the pre-fix failing-first run of the new test).
- The full suite (fast runner `tools/test/suite.py`) on the final tree (= origin/master
  bee8c129 + this branch; master had not moved), in 5 interleaved foreground chunks
  (background jobs die between tool calls in this sandbox) at `--limit 100`:
  chunks 0/2/3/4 = 571+311+329+355 passed, 8+4+5+15 ignored. Known failures, none from this
  task:
  - 3 × `disc_golden` — the slim ISO's expected failures (the task brief allows exactly these);
  - `fix_0c_pylon_live` (2) and `probe_pylon_render` — `wgpu adapter NotFound`: this sandbox
    has no GPU; render tests are main-agent surface (merge lesson 6);
  - the 100 s per-binary limit TIMEOUTed the big sweeps (`frame_cost_live`, `weapon_matrix_live`,
    `levels_sweep_live`, `npc_doors_live`, `pss_vag_golden`); two of them re-run directly green
    after the chunks: `weapon_matrix_live` 6 passed / 0 failed (98.6 s) and `frame_cost_live`
    5 passed / 0 failed (126.4 s) — limit artefacts, not failures. `levels_sweep_live`,
    `npc_doors_live`, `pss_vag_golden` and chunk 1's tail stayed unrun (the sandbox's 570 s
    foreground cap); all pre-date this task and none touches the rocket path. The next
    session / the main agent should run the full suite once with the default 600 s limit.

## Live checks for the main agent

1. gdbwatch BU+0xF8 (the bullet block's speed word) through a 0x2e Heatseeker flight — expect
   a constant ~0.2525 u/frame (the ×4.04 scale uniform onto the Scorpion; the measured 0x2d was
   0.33666). A different constant → the scale is per-record (then the port's 0x2e pin needs the
   measured value).
2. The same through a 0x2f Unguided flight — expect ~0.336667 (byte-identical flags to the
   measured 0x2d).
3. The homing at the real speed: fire the Heatseeker at a guard ~12 units away, ~17° off the
   guard's bearing — the rocket locks (BU+0x40), the heading swings visibly and the guard dies.
   At wider angles (24° @ 18 units) the game's own steer cannot converge — that miss is the
   game's kinematics, not a port bug (the scratch telemetry in this sandbox matched the steer
   constants exactly).

## Left / open

1. The ×4.04's internal source is still unpinned (the same gap fix-sentinel-camera-play left):
   the state-1 mover, the control integration and the globals FRAME_RATE_DIV=1.0 /
   REC_FRAME_RATE=1/60 at 0x30d0d4/dc are all read; the scale is measured, not derived. Live
   checks 1-2 settle whether it is uniform across all four 0x200 rocket records.
2. The drone-fired bullet path (`Drones::fire` → `Bullet`, the SP guards' path) reads the raw
   `+0x20` without the quarter or the ×4.04. For the rocket records that lands within ~1% of
   the game's step by coincidence (0.25 raw vs 0.2525 game); no guard in the SP loadouts fires
   0x2c-0x2f in the port's levels today. Left untouched — out of this task's scope, no
   user-facing symptom, and touching it risks the pinned AI RNG streams.

## Usage

not visible to me — this sandbox exposes no token, context-window or cost figures to the agent.
