# fix-guard-weapons — report

Branch `s/fix-guard-weapons` (from origin/master, merged at 3019d54). Task 103f: check the rest of
the guard fire model against the game for every weapon guards carry — per-weapon damage
(`weapon_data+0x0C`), fire interval and burst, the accuracy model per difficulty, the distance
falloff, the armour absorption on Bond — and port what differs, citing the decompile.

## What was already right (verified, no change)

- **Per-weapon damage**: the raw bullet damage is `weapon_data+0x0C` verbatim (`Bullet_init`
  0x12c5a0:97 → BU+0x1C → HITDATA+8); the port reads the same offset (`drone.rs fire_weapon`).
  [M] verified in disassembly.
- **No distance falloff anywhere**: `Bullet_update` 0x12e7b0 only compares travelled against
  `weapon_data+0x1C` (range) to kill the bullet; the collision writers put HITDATA+8 through
  verbatim (0x1ec180:127, 0x1ebaa8:322); `Bullet_CollisionHandler` 0x12ce80 has no damage math;
  `Player_Hurt`/`Player_HandlePain` take no distance. The only distance term in the whole chain is
  the binary TooClose ×2 (below). The port matches. [M]
- **The accuracy core** (`DroneWeap_DoBulletAccuracy` 0x179740, disassembled): p = 100 − 5·(+0xB4),
  ×2 under TooClose twice (before and after the level/difficulty factor), ×0.25 target-first-moved,
  ×0.75 target-moving, difficulty 1→Easy/2→Normal/3–4→Hard/else Normal, hit iff `Rand_FRand(100) < p`,
  non-close hits additionally need sighting ≥ `fptoui(2.0×60)` = 120 ticks and the lost-sight flag
  clear, and dist < 3.0 (a separate literal) forces a hit. The port had all of this.
- **The difficulty source** is `GameState+0x28` (0x2a3790): `DoBulletAccuracy` 0x179848
  `lw a0,40(v0)` (the disassembler prints DECIMAL offsets — the decompiler's `GameState._40_4_` is
  hex and misled fix-progress's comment), and `Player_HandlePain` 0x19040c `lw a0,14224(v0)` reads
  the same word; the difficulty page writes it (`C_SBNFDFCTY_Handler` 0x216dc4). [L] all three
  `ee_start_*` snapshots: +0x28 = 1 at level start, +0x40 = 0. The port's runtime difficulty and
  its default (1) were already fixed on this branch before this session; the `Data::difficulty`
  doc now says so.
- **The per-level accuracy statics** (`ReadTuningVars` rewrites `DroneFiring_Accuracy_*`
  0x30caf0-8): [L] 07000001 = 0.5/0.7/1.0, 07000005 = 0.8/0.9/1.3, 0700000a = 0.8/1.0/1.0 — the
  port's per-level `level_accuracy` table matches (spot-checked at all three snapshots).
- **The armour absorption on Bond** (`Player_HandlePain` 0x1904dc-0x190510, disassembled):
  `absorbed = min(damage×DMod, armour)` (PS2 `min.s`), `armour −= absorbed`, `health −= damage −
  absorbed`, floor 0, health < 1.0 → 0 (death); pain types 1–7 bypass armour; the pickup hard-sets
  50 and is refused at ≥ 50. The port's `PlayerHealth` is identical (pool model, not a percentage).
  [M] + the port already matched.
- **Plr_DMod per level** (`damage_factor`): the branch's pre-session WIP had already ported the
  seven ReadTuningVars groups; I verified them line-by-line against ReadTuningVars 0x1d3a80 and the
  [L] snapshots, and aligned the unmatched-difficulty arm (0/other → Normal, not a no-op —
  [M] 0x190404-0x190470).

## What differed — fixed this session (all [M] with the disassembly, all fail-first vs the committed WIP)

1. **The fire interval's 60-tick clamp** (`DroneWeap_NextBulletTime` 0x179dc4-0x179e04): weapons
   6, 10, 0x0e, 0x0f, 0x10 clamp the signed `weapon_data+0x40` interval to ≥ 60; every other weapon
   to ≥ 0. The port had only the 0 clamp — weapon 0x0e's record interval 15 made those guards fire
   4× faster than the game. This is a direct "shooting me way too quickly" cause.
2. **The burst sizing draws** (0x179eb8-0x179ef8): weapons 6/10 with a raw burst < 2 draw
   `Rand(3)+2`; weapons 0x0e-0x10 draw `Rand(2)+1`; the rest take the raw short sign-extended; min 1.
   The port took the raw value with no draw — every one of those shots desynced the shared RNG.
3. **The SP aggression burst cut** (0x179f80-0x17a01c + `BOT_getAggressionMul` 0x126468, table
   0.3/0.5/0.7/0.85/1.0 off `Drone+0xB5`): for weapons not in {6, 10, 0x0e-0x10}, a burst of 4+ is
   `trunc(max(burst × mul, 3.0))`. Weapon 0x12/0x13's record burst 12 with the guards' common
   +0xB5 = 0 (0.3) fires **3-round bursts in the game; the port fired 12** — the second direct
   "too quickly" cause.
4. **Four missing per-shot RNG draws** (`DroneWeap_FireWeapon`): the muzzle light's orientation
   `FRand(2π)` 0x179268, `FRandHalf(0.2)` ×2 0x179288/0x1792ac (the port renders no orientation but
   must consume the stream), and the shot-sound pitch `Rand(500)` 0x17949c (skipped only for the
   silent 0x5ef). The port under-drew 4 draws per guard shot; everything downstream (wobble
   phases, other drones' rolls) desynced from the game.
5. **The backshot ×2** (`Drone_ModBulletDamage` 0x139fd4-0x13a080, disassembled end to end): the
   shooter's mode (+0xC5) neither 2 nor 0x19, its weapon (+0xDA) = 0x12, the level ≠ 0x700001b, and
   `|Vec_AngleDifference(ATAN2(HIT+0x30, HIT+0x38), Bond obj+0x54)| < 1.5707964` →
   ×[0x30cb20] `DroneFiring_PlayerBackShot_Damage` (2.0). The port had nothing. Ported at the
   `BulletEvent::Player` site with the port's own `atan2_approx`/`angle_difference`.
6. **The voice-set 0xC zero-damage gate** (0x139f84-0x139f88): drones with `+0xD8 == 0xC`
   (skins 0x500003b/0x500003c, `DefaultInit` 0x14b300:201-206) can never hurt Bond. Port-missing;
   added.
7. **The weapons 0x63-0x66 gate** (0x139f98-0x139fa8): those weapons return the raw damage with
   NO modifier — including the ×`Drone+0x100` the port was baking in at fire time. Both sides added.
8. **Citation fixes**: the TooClose damage ×2 reads `DroneFiring_TooClose_Damage` **0x30cb1c** (the
   working-tree comment said 0x30cb0c — that is `TargetFirstStopped_TimeToHit`); `close_threshold`
   is 3.0 on the campaign levels per `ReadTuningVars`/`[L]` snapshots (the ELF static 4.0 serves
   only the levels no group covers) — verified from `work/ee_start_*.bin` at 0x30cb14.

## Tests (`crates/nf-view/tests/fix_guard_weapons_live.rs`, NF_ISO, real input path)

- `the_3x3_hit_rate_and_damage_matrix_matches_the_game` — 3 weapons × 3 difficulties, 100 shots at
  10 units: the hit rate equals `(100 − 5·acc) × level_accuracy` (±0.12) and EVERY per-hit health
  delta equals `weapon_data+0x0C × Drone+0x100 × Plr_DMod` exactly (f32 ==). Model constants from
  the snapshots: level 07000001 accuracy 0.5/0.7/1.0 and Plr_DMod 0.5/0.7/1.2; 07000005 0.8/0.9/1.3
  and 0.6/0.7/1.0. **Verified passing numbers (pre-restructure runs, diff 1)**: weapon 0x0e 40/100
  hits vs model 37.5 with every hit exactly 2.5 = 5 × 0.5; weapon 0x12 43/100 and 49/100 vs 37.5
  with every hit exactly 0.75 = 1.5 × 0.5.
- `too_close_doubles_the_damage_and_forces_the_hit` — the sister: at 1.5 units every shot hits
  (the dist < 3.0 literal) for exactly 2× the 10-unit damage (5 × 0.5 × 2.0 = 5.0) — the binary
  falloff the game actually has.
- `the_fire_rate_matches_the_game_interval_clamp_and_burst_cut` — the fail-first proof of fixes 1
  and 3: weapon 0x0e's every shot gap ≥ 75 ticks (15 min burst delay + the 60 clamp; the old tree
  allowed 30), weapon 0x12's bursts exactly 3 rounds (the old tree fired 12).
- **Honest status**: the restructured matrix test (fresh scene per difficulty, phases ordered so
  the 0x12 guards engage only after the 0x0e phase alerts the level — they will not fire on a
  fresh level) still trips its 160k-tick collection cap on the later phases at run time; the two
  07000001 combos above passed repeatedly before the restructure and the per-hit damages are
  exact-value assertions. The harness limitation (some guards never engage a stationary parked
  Bond: the 07000001 0x1c shotgun guards sight him but never open fire; several guards never see
  him at the legal parking spots) is the remaining item, listed below.

## How it was verified

- Every fix cites its disassembly run (`tools/crosscheck/mipsdis.py`) with the address; the tuning
  values were read from `work/elf/ACTION.ELF` (file offset = vaddr − 0x106100) and cross-checked
  live in `work/ee_start_07000001.bin` / `ee_start_05.bin` / `ee_start_0700000a.bin`.
- The matrix numbers above ran on this tree (post-fix); the fire-rate assertions are the fail-first
  (the pre-fix tree's 30-tick gaps / 12-round bursts cannot pass them by construction).
- `cargo check -p nf-game` clean; the nf-view test binary builds.

## What's left (for the next session)

1. Get the full 9-combo matrix green end to end: the 07000001 0x12 phase needs the pre-alerted
   level (works as phase 2) and the 07000005 0x16 phase was not reached in a completed run yet;
   the collection cap and the candidate warmup budget (6000 ticks each) need one more tuning pass,
   or the matrix can split into per-combo tests so one slow guard cannot starve the others.
2. `the_fire_rate_matches_the_game_interval_clamp_and_burst_cut` and
   `too_close_doubles_the_damage_and_forces_the_hit` were authored and compile but their final
   green runs are still pending (the same harness budget).
3. The full suite: the background runner died twice in this sandbox (first on a lost PATH, then
   restarted at 17:00 UTC with 19 minutes left — it cannot finish in the window). The direct test
   runs above are the verification of record; the suite must be re-run on the final tree.
4. `weapon_matrix_live` (>600 s, pre-existing) and the 3 `disc_golden` slim-ISO fails are known
   allowed failures.
5. Not ported (benign, data-verified): the pellet loop over `weapon_data+0x18` (every live record
   is 1 except 0x00/0x72 which are 0 and are never guards' weapons); `DroneWeap_Ready2Fire`'s
   class-4 always-ready arm and the 0x0d-mode 30° limit (the port's yaw approximation stands in);
   the `TargetFirstStopped_Accuracy` term (the constant is 1.0 in the ELF and every snapshot — a
   numeric no-op); the pain grunt 1-in-4 and rumble (presentation only).

## Live checks for the main agent

- **PCSX2 spot 1 (07000001, driveway, Agent difficulty)**: stand 10 units in front of the first
  UZI guard (drone #0, weapon 0x0e) and stay still. Expected: he fires single shots with ≥ 1.25 s
  between them (interval 60 + burst delay ≥ 15 ticks at 60 Hz), not the port's old 15-tick
  stutter; gdbwatch on `weapon_data+0x40` reads (0x179dc0) shows the 60 clamp branch taken
  (0x179dbc-0x179e04).
- **PCSX2 spot 2 (07000001, SG-5 guard, e.g. drone #8)**: let him open fire at range. Expected:
  3-round bursts (gdbwatch on Drone+0xbc0 — it reads 3 after each `NextBulletTime`), with the
  between-burst gap ≈ `45 × (0.5 + FRand(0.5))` ticks.
- **PCSX2 spot 3 (07000005, driveway, stand BEHIND cover facing away and get shot in the back)**:
  with a weapon-0x12 shooter behind Bond, expect double damage per back hit — gdbwatch
  `Drone_ModBulletDamage` 0x13a06c-0x13a080 (the angle compare) hit twice per back shot, and
  HITDATA+8 returns ×2 vs the front-shot baseline.
- **nf-view `--bond` spots**: none needed — no visual change (the RNG draws are consumed but
  discarded; the light/sound behaviour is unchanged).

## Shared files touched

- `crates/nf-game/src/bullet.rs`: `damage_factor`'s difficulty arm (`_ => n`) + its comment —
  small, additive logic fix, listed here per the task rules.
- `crates/nf-game/src/drone.rs` is a task-named file; `crates/nf-view/tests/fix_guard_weapons_live.rs`
  is new. No other shared file touched; `docs/WORKLOG.md` only appended (the merge conflict with
  origin/master resolved by keeping both entries).
