# mp-map-objects — multiplayer maps' moving objects and vehicles (task 744)

Playtest 1016-1355 (on master 2026-10-16): "on the multiplayer maps, like Gondola, it's missing
the actual gondolas. It has these things called remote tanks and like helicopters that you
should be able to pilot, and you can't."

## What the disc says (census)

All eight on-disc arenas were surveyed straight from the ISO (FILES.BIN LBA 145346; per-level
bins; block 0x1A placements; python reimplementation cross-checked against the existing
`mp_arenas_live` / `nf-extract kind-census` counts on all eight archives — 181/266/343/124/188/
138/264/225 placements):

- **The gondolas (Skyrail 0x07000024)** are a kind-0xD9 script player: placement [4], object
  `OBJECT_CableCarbody` (hash 0x0200037a), translation (0,0,0), params `{0: 0x06000058 script,
  2: 8 (loop mode), 5: 0 (mask)}` [D]. Script 0x06000058 (10,112 bytes, ISO 0x1FE2B420) runs two
  phase-shifted carriages, each with a parented sub-entity (0x0200038a), along a closed cable
  loop [D; script read via the port's Nis parser].
- **Ravine (0x0700004b)** has four kind-0xE2 SS cable cars (25 keys each, rate 8500, paths 0–3
  in the main map's 0x19 blocks — 0x1C-byte vec3+quat keys shuttling x=111.891 / x=102.624 rows
  between z −126.676 and −184.750 with two-key end holds) [D].
- **Sub Pen (0x07000025)** has three script-player lifts (0x060008C2/C4/C6) + touch plates [D].
- **The "remote tanks" are 4 × kind-0xDA `Car_Create` (RCCar 0x020003d6) on every arena except
  Sub Pen** (which has none) [D]. **There is no 0x2F copter placement on any arena**; Snow
  Blind's "helicopter" is three static decoration cels [D]. The playtest's helicopters are the
  same Car objects with the Enviro-Mods "Helicopters" option: `Car_Create` swaps the gfx to
  model 0x2000194 and adds 1.5 to the bounds when `MPSettings+0x460 == 2` [M 0x1305b0:53-58;
  the menu row is `P_MPENVIROMODS_Handler` 0x210030, options Off/Tanks/Helicopters/Random].

## Done

### 1. The gondolas now move (the playtest's first item)

**Root cause** [M]: the game primes EVERY placed script player whose mode A ∉ {0,1} at create,
regardless of the trigger mask — `SP_CreateScriptPlayer` 0x1db9a8:72-76 turns mask 0 into state
6 ("loaded, looping") and `SP_LoadScript` 0x1dbc08 then loads and `Script_Play`s it.
`SP_Update` 0x1dbda8:57-59 stops only on the `+0x36` disabled byte, whose sole writer is
`SP_HideObj` 0x1dcbf0 — called only by the campaign copter (0x17d340:73-74 sets it,
0x17dbb8:95 clears it). The port's `nis::Player::prime` skipped every mask-0 SP (the
pickup-port-check §9 "disabled byte" stand-in), so Skyrail's gondola — mask 0 — never loaded:
a probe showed `info[0] = None` and a stationary identity matrix.

**Fix**: `prime` no longer skips mask 0; a new `Player.hidden` field models `SCRIPTPLAYER+0x36`
(defaults false, gates the whole update including `Script_Update` [M 0x1dbda8:57-59]). The one
SP the game really keeps hidden is 07000005's 0x16ddb40 looping pair (+0x36 = 1 [L,
docs/fields/script-player.md §2]; setter unknown — it is NOT a placed 0xD9 on that level's main
map, so the change cannot unhide it; see Live checks).

**Probe result** (through the real script machinery the sim runs every tick):
`Scripts::new` + `begin_tick` + `update_player` on the Skyrail level: the gondola player loads
(play 0x9 = playing+looping, state 6), and four entities ride the loop — two carriages
(0x0200037a) with their sub-entities (0x0200038a), phase-shifted: e.g. at t=600 carriage A at
(4.05, 30.54, −50.81) while carriage B is at (−10.87, 11.79, 54.60). Pinned in
`crates/nf-game/tests/mp_map_objects_live.rs::probe_gondola_script_ticks` (assertion: the
player loads and the matrix leaves the placement origin).

The Ravine SS cable cars and Sub Pen lifts run through the port's existing `ss.rs` /
script-player machinery (not touched this session; the SS path data is pinned in
`probe_ravine_ss_movers`).

### 2. The remote vehicles — the Car state machine ported

New `crates/nf-game/src/car.rs`, every constant cited:

- **Create** [M 0x1305b0]: gated on `MPSettings+0x384` (MP on) and `+0x460 != 0` (the option
  not Off); **blocked on Ravine 0x0700004b** (`GameState+0x12` check — the disc's 4 placements
  there never create cars); `NumTanks < 8`; variant byte `CAR_INFO+0x112`: option 1 → tank,
  2 → helicopter, 3 → `Rand_Rand(100) < 0x33` (51 % helicopter, one shared-stream draw ONLY on
  this option — placement order matters).
- **Activate** [M 0x131c48]: only from state 0; `Player_ChangeSubState(player, 0xB)` (prev
  saved to BLData+0x942), cam mode 0xD, the camera mounts the turret sub-object (tank) or the
  body (helicopter) at BLData+0x878, state 5 → 1, pilot slot into +0xf6, `Player_Disable(1)`,
  `Player_WeaponNone`, sounds 0x60a (tank) / 0x10b (helicopter) with spin-up timers 10 / 4,
  heat reset.
- **Deactivate** [M 0x131d88]: state 99, driver moved to +0xf0, sound stop, `Explode_Create`
  (10, 10, 50, blast script 0x06000052, colour 0x2d6210, radius 100), pose restored from
  `TankSpawns[slot]`, respawn timer 0x708 ticks (30 s) in +0xf6.
- **Update** [M 0x1307d8]: the state-99 hide + countdown → state 0; the heat accumulator cools
  1/tick unconditionally and clears the latch at 0; the heli drive input conditioning
  (:157-246): yaw `+input·FRM·0.05`, pitch negated, throttle/steer lerp toward `2·input·FRM`
  at 0.15 with ±0.125 / ±0.1 clamps and the ±0.0002/0.01 dead-zone zero; shells: `Bullet_init`
  0x2c55d0, +10 heat, 6.0 s (0x40c00000) shell cooldown, overheat latch past 100. Note: shells
  alone can never overheat (10 heat per 6 s vs −1/tick cooling) — reaching 100 needs the MG
  arm, which is next-session work.

`crates/nf-game/tests/car_state_machine.rs` pins all of the above (5 tests). **Not modelled /
not asserted**: the tank drive branch, the matrix composition (RotMatrixZYX + quat integrator),
the MG arm, `Car_CollisionHandler` 0x131838, `Car_PlayerHasDied` 0x132e98, the bullet/actuator
flags.

### 3. What is NOT wired yet (the honest list)

- The 0xDA placement walk does not call `Car::create` yet (the game creates in placement order
  via `parsemap_create_dynamic_objects` 0x1d04f0 case 0xDA); the hook list is in car.rs's
  header. `Player_Activate`'s class-0x28 arm (0x1a5170:142-145, the use-key sphere cast) and
  the control-walk tick are the two remaining sim hooks; without them the vehicle is data +
  tests but not yet drivable live. Phase 1 of the MP core deliberately drove no MP objects —
  this session built the machine to drive.
- The renderer (main agent's side): the gondola entities are `Obj::Entity` records under script
  (0x06000058, streams 0..4) — the existing script-object path draws them once the sim's
  `set_dynamic` feed is enabled on MP arenas; `nf-view/src/scene.rs:5379`'s script-object block
  is gated on `level.main_map()` (MP arenas' placements ARE the main map, so it should flow
  through unchanged).

## Verification

- `cargo test --offline --release -p nf-game -- --test-threads=1` on the final tree: **475
  passed / 0 failed** across all nf-game test binaries (includes the new
  `mp_map_objects_live` 4 tests + `car_state_machine` 5 tests).
- All four crates compile in release (`cargo test --no-run` over nf-game/nf-formats/nf-audio/
  nf-view) — nf-view/nf-formats/nf-audio tests were not RUN this session (nothing in them
  changed except the nf-game public struct they only read; the full suite runner could not fit
  the session window — the background start was killed by the sandbox between tool calls, and
  the foreground run does not fit the remaining time after the two full nf-game runs).
- The 2-hour sandbox window went to the census + the gondola root cause + the Car core; the
  fast-runner `suite.py` output therefore has no final-tree total line this session — the
  nf-game binary-by-binary totals above are the push-gate evidence.

## Live checks for the main agent

1. **07000005's hidden looping pair**: what writes SP 0x16ddb40's `+0x36 = 1` before the
   snapshot? gdbwatch the byte at the SCRIPTPLAYER record (instance at obj+0xE0 of the
   class-0x1B object 0x16ddb40) during a fresh boot of 07000005 — expected: a 0→1 write whose
   caller is NOT SP_HideObj 0x1dcbf0 (the only decompile writer). This settles whether the
   port's `hidden` stand-in needs a real setter.
2. **Skyrail gondola draw**: fresh boot 07000024, spawn at a spawn point, screenshot the two
   carriages mid-loop (e.g. `--bond 12,32,-50,0,-0.5` looks along the station platform; the
   carriages cycle the loop between the station (12.2, 30.5, −50.8) and the far end near
   z ≈ 54.6). The port's carriage at t should match the game's to within the loop phase.
3. **Car_Create arg order** (disasm, not live): `Mat_Copy(obj+0x90, TankSpawns+slot*0x40)` at
   0x1305b0:74 and 0x131d88:74 — confirm the create SAVES the placement pose into
   TankSpawns[slot] and the deactivate RESTORES it (the decompiler's const qualifier suggests
   the opposite direction; the respawn-at-spawn behaviour needs the disasm word).

## Remaining work (next session)

1. Wire `Car::create` into the placement walk (placement order, the RNG draw only on option 3)
   + `Player_Activate` case 0x28 + the control-walk `Car::tick`; then the full `Car_Update`
   drive (tank branch + matrix composition + MG arm) from 0x1307d8.
2. The renderer-side carriage drawing on MP arenas (with check 2 above).
3. The 0x16ddb40 setter (check 1) → a real `SP_HideObj` path if a script op surfaces.
4. Ravine's SS cable cars: pin their 25-key shuttle in the sim's `SsList` (the data is pinned;
   the ride test is one level-load away).

## Usage

Not visible to me. This sandbox exposes no token, credit or context-window figures on any
surface I can read (no status line, no /usage output, no environment variable carrying them);
the subagent summaries printed their own model-token counts, which are not this session's
figures and are not copied to avoid estimating.
