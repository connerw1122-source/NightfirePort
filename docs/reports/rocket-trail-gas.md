# rocket-trail-gas — the rocket's smoke: count, size, life

Task 723. The 08 side-by-side showed the port filling the corridor with grey haze while a
rocket flew; the PS2 stayed mostly clear with a thin trail. The audit found the haze was
port-only: the game spawns **no gas at all** during a rocket's flight — the thin trail is a
different system (the swoosh ribbon) that the port does not have. One rocket shot's entire gas
output in the game is the single muzzle-smoke puff, whose life the port had 16× too short.

## What the game does (the audit)

Every fact below is from the decompile (`work/decomp/ACTION.ELF/`), checked against the
disassembly where the decompiler dropped arguments, and the weapon records swept in the EE
start snapshot (`ee_start_05.bin`, `weapon_data` at 0x2bf150, 0x10c bytes per record).

- **One 0x2d shot = exactly one `Gas_Init` puff: the muzzle smoke.** `Player_MuzzleFlash`
  0x1a4298 spawns `Gas_Init(0.05 → 0.1, 0x02000039, kind 7, rgba 0x48403080 → end 1)` per
  shot [M 0x1a4298:100-102], unless the record's `+0x68` has bit 0x2000 [M 0x1a4298:92; the
  skip holds for 0x32/0x33/0x4e/0x4f only [L sweep]]. The 0x2d 'Unguided' record has
  `+0x68 = 0x00014000` — no skip, so the puff runs. The decompiler drops `Gas_Init`'s 11th
  argument (the life): the disassembly loads it at 0x1a43f8 (`li $v0, 0x80`) and stores it at
  0x1a442c (`sw $v0, 0($sp)`) — **life 128 ticks** [M]. The packed end rgba `1` (loaded in the
  delay slot at 0x1a4440) decodes to bytes (0,0,0,1) — the alpha fades (1−0x80)/128 per tick
  [M 0x1aacc8:56-71]. The only RNG the site consumes is the caller's `Rand_FRand(2π)`
  (0x1a43ec) — the velocity is a NULL pointer, so `Gas_Init` takes the kind-7 wind branch
  (3 more draws) [M 0x1a4458 a3 = 0; 0x1aacc8:107-116].
- **The flight spawns nothing.** `Bullet_DoTrails` 0x12d890 runs every flying tick; every gas
  arm reads a record flag: bit 0x2 → `Bullet_trail_effect_smoke` (the 0x200001f trail, every
  4th tick) [M 0x12d890:144-146], bit 0x20 → the 0x2000039 exhaust puff (every 3rd tick) [M
  :152-166], bit 0x80000 → the small 0x2000039 puff (every 2nd tick) [M :172-188]. The 0x2d
  record's `+0x6c = 0x0001a200` carries none of those bits [L sweep]: it has 0x2000 (the
  per-tick flight light, already ported) and 0x10000 (the swoosh ribbon). The sweep found the
  flag owners: bit 0x2 = 0x1b/0x36/0x69, bit 0x20 = 0x2a/0x2b, bit 0x80000 = 0x41.
- **No launch puff.** `Bullet_init` 0x12c5a0 contains no `Gas_Init` — the muzzle flash is the
  whole launch effect [M].
- **The impact spawns no `Gas_Init` either.** `Bullet_DoTrails`' impact arm needs
  `+0x70 & 0x200` (the timer) and its gas puff needs `+0x70 & 0x2000`; 0x2d's `+0x70 =
  0x00000004` [L sweep] — the timer-arm records are the grenades 0x2b/0x34-0x40 and 0x59/0x5b,
  and the impact-gas (`Gas_Init(0.5, 1.67, 0x200001f, kind 0xb)`) records are 0x35/0x59/0x5b
  only. The collision handler 0x12ce80 calls `Effect_Create` (gated off: flags 0 without
  `+0x70` bit 2, which 0x2d also lacks) and `Explode_Create` — which itself contains no
  `Gas_Init`; the blast's visuals ride the 0x6000052 script, not the gas pool [M].
- **The thin trail is the swoosh ribbon.** For bit-0x10000 records `Bullet_init` calls
  `Swoosh_Create(spawn, spawn, weapon_data+0x76, …)` and `Bullet_update` feeds the round's
  position every tick [M 0x12c5a0:116-133] — a sprite strip (mesh 0x2000481 for the rockets'
  type-0x2c base, size +0x76 = 240 [L]), not a gas puff. The port has no swoosh system (a
  known, documented gap).

## What the port did wrong, and the fixes

1. **The haze (the headline).** `update_player_bullets` spawned one cyan stand-in gas puff per
   flying tick for EVERY bit-0x10000 record (`gas.init(…, 0.3, 0.6, kind 5, [0x30,0xd0,0xff,0x90], life 6)`).
   For the Samurai bolt (0x32/0x33) that approximates its live-checked "short cyan beam"; for
   the rockets (0x2c/0x2d/0x2e/0x2f) the game spawns no such puff — over a long corridor flight
   the port laid a chain of growing translucent puffs down the whole path: the grey haze.
   **Fix:** the stand-in now gates to the 0x20005cd swoosh family (record base type
   0x33/0x6a/0x6e) — the bolt keeps its approximation, the rockets get none
   (crates/nf-game/src/drone.rs).
2. **The muzzle puff's life was [S] 8 ticks; the game's is 0x80 = 128** [M 0x1a43f8/0x1a442c],
   and the fade ends at alpha byte 1, not 0 (the packed end rgba `1`) [M 0x1a4440].
   **Fix:** scene.rs spawns it through `GasPool::init_ex(…, end 1, life 128)`.
3. **`gases_smoke` case 7 dropped the puff's own velocity and sank it.** The game's case 7 is
   the case-0 fallthrough body: `pos += vel + WldGravity·(REC_FRAME_RATE·−0.02)` — the wind
   drift is kept and the negative factor is an upward buoyancy (+0.0033/tick) [M 0x1ab2d0:47-65].
   The port sank puffs at `WldGravity·0.05·REC` (downward, 2.5× the magnitude, wrong sign) and
   ignored `vel`. **Fix:** gas.rs update case 7 (this also corrects the broken-searchlight
   smoke's motion, same kind).
4. **`Gas_Init`'s kind-7 velocity lacked the ×REC_FRAME_RATE.** The game computes
   `(wind + Rand_FRand(0.2) − 0.1) · REC_FRAME_RATE` per axis [M 0x1aacc8:108-115]; the port's
   generic `init_ex` arm produced ~12× the drift. **Fix:** gas.rs `init_ex` kind-7 arm.

No shared-RNG-stream change: the fixes touch no `Rand_*` draws (the muzzle site's 5
unconsumed `Gas_Init` draws remain the documented G-033 deviation, listed under "left").

## Verification

- `crates/nf-view/tests/rocket_trail_gas_live.rs`, through the real input path
  (`nf_view::load` on **07000008**, dev-panel give of the Sentinel 0x2c, the ○ mode twin
  switch to 0x2d [L +0x05 = +1; M `fire_mode_switch`], `Sim::step_bond` with the trigger):
  - `one_unguided_rocket_shot_spawns_only_the_games_puffs` — the pool holds at most ONE live
    puff at any tick across the shot, the flight (≥5 ticks) adds none, the round resolves to
    its blast, and the one puff carries the game's data: mesh 0x02000039, kind 7, scale
    0.05 → 0.1 over 128 ticks, rgba 0x48403080 fading toward (0,0,0,1), life 128.
  - `the_guided_sentinel_flight_spawns_no_gas_either` — sister on 0x2c (`+0x6c = 0x00012204`
    [L]): same, no flight gas.
  - **Both tests FAIL on the pre-fix tree** (verified by checking out 161bc0d3~1 and
    re-running: the flight puffs and the 8-tick life trip the assertions) — the pin is real,
    master did not already behave like the game.
- Ran before push (per-crate, `--offline --release`, after the origin/master merge b4161012):
  - `nf-game` lib: 241 passed; full `nf-game` targets green.
  - `nf-view`: `rocket_trail_gas_live` 2/2, `fix_weapon_effects_live` 6/6,
    `weapon_fx_levels` 5/5, `lighting_live` 5/5.
  - The full suite (`tools/test/suite.py`) totals are in the WORKLOG entry; the final tree
    adds only this report and the WORKLOG close-out (markdown) after the suite's code state.

## What's left

- **The swoosh ribbon itself** — the thin trail the PS2 shows. It is a sprite-strip system
  (`Swoosh_Create` 0x1a4e40 / `Swoosh_Update` 0x18d200) the port does not have; until someone
  ports it, the rocket flight reads clear-with-flame-light instead of clear-with-thin-trail.
- The AR-GL round 0x1b carries bit 0x2 (the 0x200001f trail smoke) but flies as a plain bullet
  (no gravity bit), and the port's trail-smoke site lives only in the grenade path — its trail
  has no bullet-path site yet.
- Record 0x41 carries bit 0x80000 (the small every-2nd-tick 0x2000039 puff) — no port site.
- The muzzle site's 5 unconsumed `Gas_Init` draws (rotation is consumed; spin, sign and the 3
  wind draws are not) — the G-033 deviation; re-basing it would shift every weapon shot's RNG
  stream and needs its own task.
- `gases_smoke` kinds 3/6/8 motion and kind 0xb's `alpha = life·8.5` override are not ported
  (no rocket-scope caller; the stun/smoke grenades' impact puff rides the grenade path).

## Live checks for the main agent

All on 07000008. The known corridor spot: `--bond 155.19,43.12,-113.38,1.57,0`.

1. **The corridor clears.** At that spot, fire one 0x2d shot level down the corridor
   (nf-view render, pre-merge branch vs this branch): this branch must show NO chain of
   translucent puffs along the flight path; the corridor reads like the PS2's "mostly clear"
   (minus the thin ribbon, which is the documented unported swoosh).
2. **The muzzle puff hangs.** Same spot, right after the shot: one small brown-grey puff at
   the muzzle, roughly constant size (~0.05→0.1 over 2.1 s), drifting up slowly, gone after
   ~2 s — not the old 8-tick blink, not a sinking cloud.
3. **PCSX2 cross-check (gdbwatch).** Watch `Gas_Init` 0x1aacc8 hits during one 0x2d flight:
   expected exactly one call (the muzzle, at the shot) and none until the blast's script
   effects; confirms the sweep on live play.
4. **The bolt keeps its beam.** Fire the Samurai bolt (0x32): the cyan per-tick puff stand-in
   still renders (the 0x20005cd family gate kept it).

## Usage

not visible to me (no token, context-window or cost figures are exposed in this environment).
