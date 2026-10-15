# fix-sg5-casings — the SG5's casings flew huge across the screen while walking forward

Playtest 1014-1725 (user 05, real-input build, walking forward + firing the SG5): "I could see the
bullet casings flying across my screen, like super large." The PCSX2 ground truth
(docs/live/2026-10-14-weapon-measurements-pcsx2.md, "The SG5 Commando 0x16 casings while walking
forward") measured the real game: small brass shells, about the size of the view model's front
sight, ejected from the right side of the receiver, flying up and to the LEFT away from the camera,
gone off the top-left within ~2 frames; walking forward never brings them back and they never get
large.

## The cause

**The port ejected the shell at Bond's eye; the game ejects it at the view-model gun bone.**

- [M] `Bullet_init_casing_ex` 0x12f678 (ACTION.ELF decomp, line 160): after the weapon's +0x78
  delay the casing is re-positioned with `Vec_Copy2(auStack_e0, obj+0x30, obj+0x40)` —
  `auStack_e0` is the VECTOR out of `AnimGetBoneWorldTrans(shooter, weapon_datum_bone, 2|1, …)`
  (the branches at 0x12f710-0x12f830: the player path reads the datum id from
  `AnimDatumGetWeaponInfo`, the guard path bones 1/5). So the eject origin is the weapon datum's
  bone world position — the view-model gun — not the eye.
- [L] The gun sits on the record's +0xE0 first-person offset (weapon_data.md; measured
  ee_start_05 @0x2bf150+0x16·0x10C): the SG5 (0.028, 0.203, −0.244) — 0.25 ahead, 0.20 up; the P2K
  (0.028, 0.075, −0.903). A shell drawn at the eye (distance ≈ 0.02) fills the frame: the
  "super large" frames. The forward walk then sweeps the camera straight through it: "flying
  across my screen".
- [M] The port's `fire_player` (drone.rs) ejected at `from` and the only real caller (scene.rs
  fire block) passed `camera` — Bond's eye. The game's eject velocity is the weapon-local vector
  `(+0xf8, −+0xfc, +0x100)` through that same bone matrix (the +0xfc negation at 0x12f83c;
  `FRAME_RATE_MUL` = 60/FRAME_RATE = 1.0 at the fixed 60 Hz [M GS_SetRefreshRate 0x17d2b8], so the
  init multiplies are no-ops — the port's velocity model was already right).

## The fix

- `crates/nf-game/src/casing.rs`: new `Casings::eject_bone(ammo, weapon, bone, shooter_y, rng)` —
  the game's own transform: local `(+0xf8, −+0xfc, +0x100)` through the bone matrix (row-major,
  `ApplyMatrixLVI` convention; row 0 = the datum's +x = the viewer LEFT per `Player_PositionGun`
  0x1a3b40's gl, row 1 = up, row 2 = the barrel forward), origin `bone[3]`. The old `eject`
  (yaw/pitch stand-in at the passed origin) delegates to it unchanged — the guard path and direct
  callers keep their behavior. RNG order preserved exactly (3× MVar2(0.1, 0.05), 3×
  MVar2(0.01, 0.005), 1× FRand(0.5)) — the shared RNG stays bit-exact.
- `crates/nf-game/src/drone.rs`: `Drones.eject_gun: Option<[[f32;4];4]>` (own line, merge lesson 4).
  `fire_player` consumes it for the shot's casing; `None` falls back to the old stand-in.
- `crates/nf-view/src/scene.rs` (shared file, one additive hook): the fire block sets
  `self.drones.eject_gun = Some(self.gun_matrix())` before `fire_player` — `Sim::gun_matrix` is the
  port's `Player_PositionGun` 0x1a3b40 (the posed first-person gun bone, +0xE0 offset + bob +
  crosshair turn), the same matrix the game's `AnimGetBoneWorldTrans` returns.

With the SG5's record ([L] +0xF8 = (0.02, −0.05, 0) → local (0.02, 0.05, 0)): the shell now ejects
0.26–0.31 from the eye at the drawn gun, drifting up and to the LEFT (0.02·left + 0.05·up per tick)
— the measured behavior.

## Verification

- **Fail-first** (`crates/nf-view/tests/fix_sg5_casings_live.rs`, real input path
  `nf_view::load` + `Sim::step_bond` with the window-built `Input`; SG5 through the user's path,
  the dev give `do=weapon&id=0x16`, on 07000005): on the pre-fix tree the walked burst's shells
  ejected at eye distance **0.021 / 0.071 / 0.091** (the eye itself) — assertion `d >= 0.15`
  FAILED. Post-fix: **0.309 / 0.273 / 0.257**, and the eject velocity is view-left/up positive
  (the measured up-left drift) — 2/2 green.
- Sister assertions in the same test: while walking forward no shell ever comes back within 0.1 of
  the eye (the camera never overtakes one — "they never get large"), and the SG5 (ammo 5) ejects
  its own mesh 0x02000447 [L ammo-data.md].
- Regression sweep (tools/test/suite.py --only, on this exact tree = origin/master 2cbfd7e3 +
  this fix): casings16_live, fire_modes3_live, fix_weapon_fire_modes_live,
  fix_aims20_modes_lights_live, fix_grenade_release_live, fix_grenade_throw_live,
  fix_party_mission_live, fix_weapon_sounds_live, fix_weapons_all_live, weapon_switch_live,
  fix_sg5_casings_live + the nf-game and nf-view unit-test binaries (the casing module's tests
  among them) — **47 passed, 0 failed, 6 ignored**.
- The sandbox reset at 05:04 UTC (right after the sweep and the checkpoint push) destroyed the
  local build; the branch survived on origin. The re-cloned tree (origin/master 2cbfd7e3 + this
  fix, commit 6b0f45c2) was rebuilt from scratch and re-validated with a wider sweep — 25 binaries
  (the list above plus weapon_fx_levels_live, weapon_assets_live, models_weapons_live,
  weapon_anims_live, fix_weapon_effects_live, fix_aim_raise_live, fix_guard_weapons_live,
  fix_grenade_types_live, fix_remote_mine_play, fix_samurai_fire_live, fix_auto_switch_live,
  decal_order_live): **91 passed, 0 failed, 9 ignored** (tools/test/suite.py, --test-threads=1).
  The full 305-binary suite did not run this session — the window went to the fail-first cycle,
  the two sweeps and the reset recovery; the 25-binary sweep covers every firing/casing/weapons/
  effects suite that touches the changed path plus both crates' unit-test binaries.

## How to reproduce (headless)

`NF_ISO=<iso> cargo test --offline --release -p nf-view --test fix_sg5_casings_live -- --test-threads=1`
— `the_sg5_shell_ejects_at_the_gun_not_the_eye` (gives the SG5 through the dev give, walks forward
40 ticks holding fire, records each shell slot's first flying frame).

## Live checks for the main agent

1. **PCSX2 side-by-side** (07000006, the guard's dropped SG5, the measurement's setup): walk
   forward + hold fire; the shells should be small, eject right of the receiver, fly up-left, gone
   in ~2 frames. nf-view: `nf-view <iso> 07000005.bin --dev` then give 0x16, walk + fire.
2. **--bond render spots** (first 10 fire frames, the render was not produced in this session — no
   GPU after the reset; the main agent's render owns the visual, merge lesson 6):
   - 07000005 start, give 0x16, fire tick 0-3 from the spawn spot — each frame should show at most
     a small shell near the gun's right side, never a frame-filling object.
   - The same while walking forward — no shell may pass through the camera plane.
3. **gdbwatch** (optional): `Bullet_init_casing_ex` 0x12f678 — confirm obj+0x30/0x34/0x38
   (post-Vec_Copy2) equals the weapon datum's bone position on the frame the shell activates, and
   that data+0x00 (velocity) ≈ the bone matrix · (0.02, 0.05, 0) for the SG5.

## Known stand-ins (documented, not guessed)

- The bone matrix is the port's `gun_matrix` (the whole gun's pose), not the game's per-datum bone
  from the view-model anim — the datum's true local axes are unmeasured; the up-LEFT direction is
  anchored to the PCSX2 measurement and holds for the SG5's record. The P2K's casing direction is
  unmeasured in PCSX2 (the port gives it up-right: local (−0.01, +0.03) through the same basis).
- The landing plane keeps the existing `shooter_y = from[1]` (the eye) rather than the game's
  shooter obj+0x34 — pre-existing behavior, unmeasured, out of this task's symptom.
- The first 10 frames were not rendered in this session (the sandbox lost its GPU setup at
  05:04); the test's geometry assertions (eject distance, view-space direction, no camera
  overtaking) are the headless evidence. Spots above for the main agent's render.
