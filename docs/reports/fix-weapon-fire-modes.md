# fix-weapon-fire-modes — the MGL's impact mode, the guided missile camera, the modes table

Branch `s/fix-weapon-fire-modes` (from `origin/master` eed9a87). Every fact is tagged [M]
decompile (`work/decomp/ACTION.ELF/file:line`), [L] measured bytes (`work/ee_start_05.bin`,
weapon_data @ 0x2bf150, record size 0x10C), [D] disc, [S] suggested.

**Sandbox note:** this session's sandbox was wiped at ~04:20 UTC (repo, work/, ISO, toolchain all
gone; rebuilt from scratch — clone, work-slim, the 370 MB ISO, rustup, cargo fetch — losing ~40 of
the 120 minutes). The suite never ran here; see "Verification" for exactly what did and did not run.

## The mechanics found (all new evidence, from the decompile + the snapshot)

1. **The MGL's two modes are the records' `+0x70` bullet-flags, and the port had neither half.**
   - 0x2A 'Grenade': `+0x70 = 0x00000004` [L] — `Bullet_CollisionHandler` 0x12ce80:194-206 runs
     `Explode_Create(wd+0x8, wd+0x8, wd+0xC, …)` at the FIRST world contact. **Impact.** No bounce
     bit, no fuse bit.
   - 0x2B 'Delay': `+0x70 = 0x00000280` [L] — 0x100 bounce + 0x200 fuse, NO 0x4. The fuse is
     BU+0xFC = 210 ticks (3.5 s — matches the PCSX2's "~3–4 s"), decremented per tick and exploded
     at < 0 in `Bullet_DoTrails` 0x12d890:262-281 (the fuse lives in DoTrails, gated on
     `+0x70 & 0x200`; 0x2000 → the stun-flash branch, 0x20000 → the smoke branch).
   - The same flags read every other explosive: the AIMS-20 underbarrel 0x1B (+0x70 = 0x4, impact),
     the frag 0x34 / stun 0x35 / smoke 0x36 (0x280-family, fuse 210), the mine 0x37 (fuse 420, or
     7777 for a player owner — trigger-detonated [M 0x12c5a0:219-229]), the satchel family
     `((id−0x3B)·5+5)·60` [M 0x12c5a0:199-208], the tripbomb 0x3A.
   - The range-out arm (`Bullet_update` 0x12e7b0:158-162, record `+0x1C` vs `BU+0xF4` distance) is a
     SILENT state-4 death — no `Explode_Create`. The port's old Grenade exploded EVERY grenade at
     its fuse and bounced off every wall: that is exactly the user's "always delayed".

2. **The guided missile camera is `weapon_data+0x6C & 0x4` — and only the Sentinel 'Guided' 0x2C
   has it** (+0x6C = 0x12204 [L]; the 'Unguided' 0x2D = 0x1a200 and the Torpedo 'Guided' 0x30 =
   0x20200 lack it — matching the PCSX2 pass where 0x30 showed no rocket camera).
   - Handover on fire [M `Bullet_init` 0x12c5a0:234-254]: `Player_ChangeSubState(owner, 10)`
     (0x190630 clears the anim set), `BLData+0x878 = the bullet obj`, `BLData+0x942 = the old
     substate`, `Player_SetCamMode(BLData, 0xc)`, `BLData+0x944 = 0`.
   - The view [M `Player_PositionCamera` 0x18f5d8 case 0xc]: the view matrix IS the missile's
     `obj+0x90` (nose cam); the rocket body draws only in other views; Bond draws in all views; the
     first-person gun is hidden.
   - The steering [M `Bullet_update` 0x12e7b0:177-244, gated on owner class 3 ∧ `owner+0xF6 == 10`]:
     the look analogs scale by 0.024543693 (π/128) per tick, yaw about world-up, pitch clamped
     inside the ±0.995 pole; roll `BU+0x58` accumulates/decays ×0.965 (feeds only the HUD ring); a
     0.1-radius sphere at the bullet kills it on contact.
   - The end [M `Bullet_Delete` 0x12e660:26-30 + `Player_Update` 0x193600:242-256]: on death
     `BLData+0x878 = 0`, `BLData+0x944 = 0xf`; substate 10 counts it down and restores cam mode 0 +
     the saved substate — the PCSX2's red-static freeze is those 15 ticks with cam mode 0xc but no
     missile (case 0xc returns early: a frozen view).
   - The HUD [M `HUD_CreateRedeemer` 0x19dec0 / `HUD_UpdateRedeemerPane` 0x19e178]: ring/bracket
     sprites (texture 0x200045e) rotate with `BU+0x58`; a bracket locks onto copter bodies on screen
     (bright within 32 px of centre); the static overlay (0x2000465) alpha climbs past
     `elapsed/lifetime 0.85` by `(ratio−0.85)·3400`; scanlines jitter every other frame;
     health/crosshair hidden while cam mode = 0xc.

3. **The mode table** (every weapon's modes with the record bytes that differ) is now
   `docs/systems/weapons.md` — the button (square, "Alt. Fire", in Classic Bond [D PCSX2]), the
   twin pairs, the `+0x28` shots-per-pull bursts, the MGL/Sentinel/Scorpion flag pairs, the
   0x60/0x61 mode-BYTE cyclers, and every `+0x6C`/`+0x70` flag reader with its address.

## What I changed (files, all additive)

- `crates/nf-game/src/grenade.rs` — `Grenade.explode_on_impact` (own line): a world contact
  explodes (`0x12ce80:194-206`), a range-out of a no-fuse record dies silently (`0x12e7b0:158-162`).
- `crates/nf-game/src/drone.rs` — `PlayerBullet.guided_cam` (own line, set from `+0x6C & 4` in
  `fire_player`); `steer_missile` (the `0x12e7b0:177-244` steering, deterministic); `blast_at`
  (the `+0x70 & 4` impact blast through `explode::propagate`, called from the player-bullet world
  arm — the rockets' impact explosions, which the port never had).
- `crates/nf-view/src/scene.rs` — `Sim.missile_cam` + `MissileCam` + `view_override()`; the
  substate-10 block in `step_bond` (input zeroed, the look steers the missile, the 15-tick frozen
  restore); the grenade spawn feeds `explode_on_impact` and gates the fuse on `+0x70 & 0x200`
  (impact records range out at `+0x1C/speed` ticks instead); the `+0x6C & 4` fire sets the camera.
- `docs/systems/weapons.md` (new) — the table above.
- `crates/nf-view/tests/fix_weapon_fire_modes_live.rs` — 4 tests (below).
- Mechanical: `explode_on_impact: false` added to the existing `Grenade{}` literals in the older
  tests + `drone.rs`'s guard-thrown grenade (compile-required by the additive field).

Shared files touched: `drone.rs`, `scene.rs` (the task named them); every change is an additive
field/block on its own lines, no renames/reorders (merge lessons 3-4).

## Verification

(Updated after the deadline: the build finished with time to spare and everything ran.)

- `cargo check --release -p nf-game -p nf-view`: **clean**.
- **nf-game: all 34 test binaries ok** (two consecutive full `-p nf-game` runs; includes the
  grenade fuse/impact unit tests and `ai_live`'s grenade fixture, which gained the new field).
- **`fix_weapon_fire_modes_live` 4/4** (NF_ISO headless, the real `load_with_carry` + `Sim::step_bond`
  path):
  1. `mgl_grenade_0x2a_detonates_on_impact_and_delay_0x2b_runs_its_fuse` — through the real fire
     path: 0x2A explodes on the wall (< 120 ticks), the REAL B press swaps to 0x2B
     (`switch_request == 0x2b`), and 0x2B must NOT detonate on the wall (bounces, fuse);
  2. `grenade_impact_flag_matches_the_records` (unit): 0x2A-flag explodes at contact + silent
     range-out; 0x2B-flag bounces + explodes at the 210-tick fuse;
  3. `the_sentinel_guided_mode_rides_the_missile_and_the_unguided_twin_does_not` — the camera
     hands over, `view_override` tracks the flight, the turn input curves the missile, the impact
     restores within ≤ 15 ticks; sister 0x2D never rides;
  4. `the_unguided_twin_never_rides_the_camera`.
- The neighbouring weapon/grenade binaries, all green post-change: `fix_weapon_modes_live` 2/2
  (the prior task's twin-toggle + homing — unaffected), `explode_live` 6/6, `fix_weapons_all_live`
  4/4, `walk_07000004_live` 4/4, `walk_07000001_live` 4/4, `walk_07000014_live` 6/6,
  `fix_player3_live` 3/3, `fix_finale_live` 2/2, `fix_actors_live` 2/2 — 33 tests across 9
  binaries, zero failures.
- **The full `tools/test/suite.py` sweep did not run** (the sandbox reset ate ~40 min; the runs
  above are the targeted weapon/grenade slice). The next session should run
  `NF_ISO=… nohup python3 tools/test/suite.py > suite.out 2>&1 &` first thing.

## What's left (honest gaps)

- **The Redeemer HUD pane is documented, not drawn** — the ring/bracket/static overlay and the
  hidden health/crosshair need the nf-view renderer (the sim exposes `Sim::view_override()` for the
  camera; hud.rs needs the pane per `HUD_CreateRedeemer`/`UpdateRedeemerPane` above).
- The steering's roll bank (`BU+0x58`, ×0.965) is not modelled (HUD-only effect in the game).
- The steering runs in the port BEFORE the drones' tick (the game steers inside `Bullet_update`
  after the move) — a one-tick ordering difference, [S] negligible.
- The mode table's `+0x30` label ids per record (the HUD lane strings) are cited in
  `docs/systems/weapons.md` from the PCSX2 labels, not swept per-record from USATxt.dat.
- The remote mine's detonator arm, the satchel's countdown beeps (`Bullet_DoTrails:418-446`), and
  the Phoenix Samurai's charge/beam modes are documented but not ported.

## Live checks for the main agent (PCSX2)

1. MGL: `--give`-equivalent grant of 0x2A + 0x2B rounds, fire at a wall in 'Grenade' — detonates ON
   contact; press the mode button (SQUARE) — the lane reads 'Delay'; the same shot bounces and
   explodes ~3.5 s later. gdbwatch `BU+0xFC` on the 0x2B round: 210 → 0.
2. Sentinel: grant 0x2C, fire — the view snaps to the rocket (red tint, ring overlay, health bar +
   crosshair hidden); hold the turn input — the rocket curves within ~1 s; impact → 1-2 frames of
   static then Bond. Spot: any corridor (the trench shot from the doc).
3. nf-view renders: `--bond` at the 07000005 start, fire 0x2C — the camera should ride the missile
   (`Sim::view_override`), the rocket mesh (0x2C `+0x5C` = 0x02000432) visible ahead; after impact
   the normal view returns.

## ntfy

(sent after the finish steps; recorded in the WORKLOG.)
