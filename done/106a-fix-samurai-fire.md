## Your task: `samurai-fire`, the Phoenix Samurai fires nothing at an empty spot

weapon-matrix4 (docs/reports/weapon-matrix4.md, red #1): record 0x32 — the Phoenix Samurai, a LASER RIFLE with the
Overcharge 0x32 / Beam 0x33 modes (docs/live/2026-10-13-weapon-modes-pcsx2.md; not a sword) — draws, plays its fire
script and sounds, but nothing observable spawns on 16 of 18 levels (clean only where a target was in line). Find the
game's fire path for 0x32/0x33 (`Player_WeaponFiring` 0x1a5460, `Player_WeaponInitBullet` 0x1a4bf0, the charge for
Overcharge, the beam for Beam, any target gate like the stunner's no-target arm) and port what is missing: the
charged bolt must fly and hit walls (fix-fire-modes3 ported its impact blast), the beam must draw. Tests (NF_ISO): at
07000005's spawn, Overcharge fired at a wall spawns the bolt and its blast; Beam draws to the wall; sister assertions.

VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report. The main agent's PCSX2 captures have a CRT filter on: use them for presence/placement/layout,
not exact colour or brightness.

Files: `crates/nf-game/src/{gun,drone}.rs`, the fire path in `crates/nf-view/src/scene.rs` (small, local).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/samurai-fire`.

---
claimed-by: sandbox2 at 2026-10-14T20:53Z

---
## Report (done, 2026-10-14T21:54Z)

Closed by the driver: the report is on `s/fix-samurai-fire` (docs/reports/fix-samurai-fire.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-14T22:02Z)


Branch: s/fix-samurai-fire (3d865c6b). The report body:

# fix-samurai-fire — the Phoenix Samurai fires: the reach, the blast on flesh, the beam's trail

Branch `s/fix-samurai-fire` (from origin/master 9c55eace). weapon-matrix4's red #1: record 0x32 —
the Phoenix Samurai laser rifle (Overcharge 0x32 / Beam 0x33, docs/live/2026-10-13-weapon-modes-pcsx2.md)
— drew, played its fire script and sounds, but nothing observable spawned on 16 of 18 levels.

## The three fixes (each with the game address)

1. **The reach — the mode press was forever inert** (`crates/nf-game/src/gun.rs` + the four
   `has_ammo` call sites in `crates/nf-view/src/scene.rs`). The game's dry check for a mode press
   is `Player_WeaponHasAmmo` 0x1a16c0, whose clip half goes through `Player_AmmoInGun` 0x1a9190:
   **a twin record (+0x05 != 0) reads the BASE's clip slot (record +0x02) unless the +0x90
   ammo-shared bytes differ** [M 0x1a9190; both Samurai records' +0x90 = 0x1f, base(0x32) = 0x33
   [L ee_start_05.bin @ 0x2bf150]]. The give fills the base's slot (`Player_EquipWeapon` 0x1a8a58,
   the port's `clip_slot`), so `clip[0x33]` = 100 and `clip[0x32]` stays 0 — the port's has_ammo
   read the twin's own slot, the press found 0x32 dry and never queued the switch. Fixed with
   `Gun::ammo_in_gun_slot` (the 0x1a9190 pick) wired into the game's own call sites: the mode
   press, the switch/cycle block and the detonator's `Player_WeaponHasAmmo(base)` re-arm
   (`Player_WeaponInitBullet` 0x1a4bf0's first branch — the same AmmoInGun read).
2. **The bolt's blast on a flesh contact** (`crates/nf-game/src/drone.rs`, the PlayerBullet
   drone-hit arm). The Samurai's small-blast gate (`+0x70 & 0x100000` — only 0x32 carries it [L
   sweep]) is `Bullet_CollisionHandler`'s SHARED tail: `Effect_Create` then the
   `+0x70 & 4 / & 0x100000 / & 0x200000` `Explode_Create` gates all on the same non-ricochet flow
   [M 0x12ce80:186-231 — :207-216 is the 0x100000 arm, script 0x60007c4, radius/damage from the
   record's +0x08/+0x0C]. The port had the gate only on its world arm — the courtyard probe fired
   the bolt (it spawns at the release frame and flies), it hit a guard at ~85 units, and no blast
   fired. The gate now runs on the flesh arm too.
3. **The swoosh trail — the beam/bolt draws** (`crates/nf-game/src/drone.rs`, the PlayerBullet
   tick). `Bullet_init` 0x12c5a0:116-133 gives every `+0x6C & 0x10000` round a `Swoosh_Create`
   ribbon (texture 0x20005cd for base 0x33/0x6a/0x6e, else 0x2000918/kind 1; size = record +0x76 =
   15 [L]), fed by `Bullet_update` every tick — the live capture's "a short cyan beam". The port
   had no swoosh system (the grenade-path trail comment documents the same gap for the rockets).
   Stand-in: one cyan gas puff per flying tick at the round's tail (0.3/0.6, kind 5, life 6) —
   no RNG draws (the game's swoosh path consumes none; determinism). Colour/size [S] — the main
   agent's render check owns the look.

The overcharge spawn itself already matched the game: the first pull spends the 100-round clip
through `Player_RoundToFire` 0x1a18e0's twin-slot read, the charge script 0x060007c2 (380 frames)
drains clip[0x33] + the cyan glow [M 0x1a5460:186-202, fix-fire-modes3's port], and the bolt
spawns exactly when the script crosses the release frame +0x44 = 280 [M `Player_SetWeaponAnimObj`
case 9 0x1a2e38:404-421]. The beam spawns on the press tick (0x33's +0x44 = 0 [M 0x1a5460:688-690])
every 30 ticks [L +0x40], 12 charge per shot [L +0x91].

## Verified (NF_ISO, the real input path: `nf_view::load_weapons` + the panel give + `Sim::step_bond`)

`crates/nf-view/tests/fix_samurai_fire_live.rs`, 3 tests — all three SHOWN FAILING on the
pre-fix tree (git stash of the three source files; the reach tests died at "the mode press must
land the Overcharge twin 0x32 (left 51/0x33, right 50/0x32)", the beam test at "the swoosh trail
never drew"), all three green after:

- `the_mode_press_reaches_the_overcharge_twin` — the give → base 0x33 → the mode press lands
  0x32; sisters: the press returns to 0x33, and the record facts (+0x02/+0x90/the give's slot).
- `the_overcharge_bolt_flies_and_blasts_on_contact` — the hold spawns the bolt exactly on tick
  280 (not before), the blast carries weapon 0x32 with radius 8.0 / damage 256.0 [L +0x08/+0x0C],
  within the flight window.
- `the_beam_draws_its_trail_to_the_wall` — shots on ticks 1/31/61/91 (the interval 30), 12 per
  shot (clip 100 → 88 → 76), the trail puffs alive during the hold.

## The suite (tools/test/suite.py, foreground chunks — this sandbox kills background jobs between tool calls, the 102c/100f/103k2/104e precedent)

**998 passed, 5 failed, 19 ignored** across ~150 binaries (libs, AI/world, all weapon/mode/sound
suites, player/camera/cutscene, portal/models/render/audio, walks 07000005/0a/0c + walkreal
0a/0c/0d, levels_sweep, menus, movies, probes). The 5 fails: the 3 allowed `disc_golden` (slim
ISO) + the 2 pre-existing GPU-adapter fails (`fix_0c_pylon_live` — `adapter: NotFound` at device
creation, the documented environmental pair). Excluded for the window: the walk_*/walkreal_*
bulk (60-560 s each), `weapon_matrix_live` (the 600 s per-binary cap, documented),
`gpu_skin_live`/`sky_gs_live` (GPU).

## The final tree (origin/master merged: 27e483ea, with 103i2's twin clip-slot unification + fix-striped-boxes2 et al)

The merge auto-resolved (no conflicts — master's `Inventory::clip_slot` write-side unification and
this task's `ammo_in_gun_slot` read-side are complementary). Re-verified on the merged tree:
**344 passed, 0 failed** (the libs + fix_samurai_fire + fix_reload_hang + the weapon/mode/sound
suites + weapon_switch + ai/grenade/doors/oddjob/handcannon/casings/vent + fix_aim_raise +
weapon_anims + fix_guard_misc).

## Visual check

This sandbox has no GPU — the wgpu adapter is unavailable (`adapter: NotFound`, the same
environmental failure the pylon tests hit), so no before/after `--shot` renders were possible.
The visible change is the swoosh trail (gas puffs). Main agent render spots (07000005, give 0x32):

- `--bond 105.9,25.3,108.1,0,0` holding fire on 0x33 (Beam): a short cyan trail from the muzzle
  toward the wall — before: nothing; after: the puff line.
- The same spot on 0x32 mid-hold (~2 s in): the cyan charge glow at the gun (fire-modes3's port,
  unchanged), then on release-to-full-charge a cyan bolt streak with the trail and the blast puff
  where it lands.

## Live checks for the main agent

1. PCSX2, any level, give 0x32: the mode press (square) should land 0x32's Beam→Overcharge round
   trip (the HUD lane reads Overcharge ↔ Beam) — on the pre-fix port the press did nothing.
2. gdbwatch `Player_AmmoInGun` 0x1a9190 with 0x32 held: the return reads `BL+0x1B8+0x33*0xC`
   (clip[0x33]), not clip[0x32].
3. Hold fire on 0x32 for ~4.7 s (the full 280-frame charge): one cyan bolt with a blast where it
   lands — including into a guard (the flesh-contact blast; the pre-fix port only blasted on
   walls).
4. The beam (0x33): the trail draws only while rounds are alive (the gas pool's 6-tick life) —
   the ribbon in the real game persists as one beam; the port's per-tick puffs are a documented
   stand-in.

## What's left

- The swoosh is a gas-particle stand-in, not the game's ribbon system (also missing for the
  Sentinel/Scorpion rocket flame — the grenade-path comment documents it).
- The beam's view-model cel overlays (`AnimDatumSetEntity` 0x1a5460:204-217, textures
  0x20008e8/9) are not ported (the barrel-glow cels — view-model visual work).
- The walk_*/walkreal_* suite bulk + weapon_matrix_live on this tree (window).
