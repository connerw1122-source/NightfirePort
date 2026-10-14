# fix-guard-weapons2 — report

Branch `s/fix-guard-weapons2` = `origin/s/fix-guard-weapons` (60b5b8e) + `origin/master` (ed718f8) merged at
5186f76 (WORKLOG conflict resolved append-only, both sides kept). Task 103f2: the branch fails six tests on
current master; find what the merge changes in the guard fire path / damage scale, fix it, keep the rest green.

## What the merge actually changed (the finding)

**Nothing in the guard fire path.** Since the merge-base 8295049 master changed only `drone.rs` (+19: the
fix-weapon-fx-levels grenade trail block, additive) and `gas.rs` (the trail spawns). The merged tree is byte-for-byte
the branch's fire code plus that block — `git diff origin/s/fix-guard-weapons HEAD` shows exactly the +19. The six
failures decompose into three unrelated causes:

1. **`fix_progress_live::the_difficulty_scales_a_guards_damage_to_bond_by_the_decompile_factor` — a real
   branch bug, fixed.** The branch had moved the boot default of `Drones::difficulty` to 1, citing
   [L ee_start_*.bin @ GameState+0x28 = 1]. Disassembly (subagent-verified, tools/crosscheck/mipsdis.py):
   Ghidra's field names are **DECIMAL** — `GameState._40_4_` is byte offset 40 = **+0x28 = 0x2a3790**, and
   `bootup` 0x1c7e78:25 stores 2 there (`sw t0,40(s1)` @0x1c7f88; GameState base 0x2a3768). So the game's boot
   default difficulty IS 2 on the same field the branch correctly identified as the model difficulty
   (Player_HandlePain 0x19040c and DroneWeap_DoBulletAccuracy 0x179848 both read +0x28; C_SBNFDFCTY_Handler
   writes it @0x216dc4 — the branch was right about the field, wrong about the boot value). The snapshots' 1 is
   session state: PTP restore (Boot_LoadPTPData 0x1c8560:36) and the mission-start force
   (`Menu_RunMiniMission` 0x1ffb90:22 writes 1); the direct-level load replays it (scene.rs `difficulty = 1`).
   Fix: `Drones::new` boots at 2 again, with the corrected citations (drone.rs). The branch's tests set the
   difficulty explicitly and are unaffected.

2. **`render2_live::dropped_guns_dim_to_the_cel_ambient` and `fix_objects_live::a_death_draws_its_drop...` —
   harness calibration vs the corrected RNG stream.** The branch's fire-model fixes added the game's own
   per-shot draws (FireWeapon 0x179234's muzzle-light orientation FRand ×3 @0x179268/88/ac — applied to the
   muzzle-flash OBJECT after Light_Create, which itself only draws its lifetime `Rand(param_11)` @0x1cc50c —
   and the `Rand(500)` shot pitch @0x17949c; NextBulletTime's burst-sizing draws @0x179eb8-ef8). With guards
   consuming a different stream, Bond's kill lands a few ticks differently and the corpse — the drop's spawn
   point — can end inside `Pickup_Update`'s touch radius (bounds.radius + 1.1 [M 0x18e650's sphere test]) of
   where the harness parked Bond: the game then auto-takes the drop (walk-over pickup), which is CORRECT
   behaviour. Evidence: fix_objects' death-dump printed the drop `gone true` at (−5.42, 6.72, −1.97) with Bond
   parked ~0.5 away. Fix: both tests step Bond clear of the corpse before the fall (test-only, intent
   unchanged). Both green.

3. **The branch's own `fix_guard_weapons_live` tests were never finished** (the branch's own report says so:
   "authored and compile but their final green runs are still pending"). Reproduced on the merged tree:
   fire-rate 4/40 (the guard stalls after ~4 shots — he repositions out of the sight line), TooClose 2.5 vs 5,
   matrix 90/100. Fixes, all test-side (drone.rs/bullet.rs needed NO further change):
   - **Harness**: on a 3000-tick shooting stall, re-park Bond around the SAME guard (up to 4×) and keep
     collecting instead of burning a 160k-tick budget on a dead engagement. Stall-safe by construction (a
     boundary gap ≥ 3000 can neither hide a < 75 gap nor join a burst).
   - **TooClose test rewritten to the game's actual two-band model**: an UZI guard parked 1.5 units away does
     NOT shoot — the AI's `NDrone2_DSTATE_CombatTooClose` 0x173338 backs him out (~8.2 units live), and the
     `FireWeapon` early-return gate `Drone+0x1A0 < weapon_data+0x08` [M 0x178f68:97-102, 0x12a960:44-46] is
     inert for 0x0e (its +0x08 min range is 0.0, read live from the records). The test now pins: hits only
     land outside the band, and EVERY delta equals record × Drone+0x100 × Plr_DMod × (dist < 3.0 ? 2 : 1) —
     all exactly 2.5 = 5 × 0.5. The ×2 arm itself is unreachable headless for this level's weapons (below).
   - **Fire-rate**: the observed 0x0e gaps are min exactly 60 with ~105-120 burst boundaries — the burst
     sizing draw `0x0e..=0x10 → Rand(2)+1` [M 0x179ed4-0x179edc] makes 2-round bursts, so in-burst gaps hit
     the tight clamp at 60; the old `min_gap >= 75` premise was wrong. Now: min == 60 both ways (tight clamp;
     the pre-fix tree's 30-tick gaps still fail by construction). 0x12's runs: no run may exceed 3 (the
     12-round record cut [M 0x179f80-0x17a01c] — fail-first vs the old tree) + 3-round coverage ≥ 70%; the
     rare 1+2 split is a mid-burst sight/aim slip, deterministic in the stream.
   - **Matrix**: 8 of 9 phases green — hit rates within tolerance (40/100 vs 37.5, 58 vs 52.5, 79 vs 75,
     31 vs 37.5, 58 vs 52.5, 65 vs 75, 50 vs 60, 70 vs 67.5) and EVERY per-hit delta exact against the model
     via the health-pool round-trip (`100 − (100 − dmg)`: 1.65 is not binary-exact, the pool double-rounds —
     the game's pool does the same). The 9th phase (07000005 × diff 3 × SG-5 0x16) cannot complete its
     collection: both 0x16 guards either never sight a parked Bond or stall through all 4 re-parks — the
     engagement-budget limitation documented in fix-guard-weapons' own report ("the 07000005 0x16 phase was
     not reached in a completed run yet").

## Verification

- All six named failures reproduced first, then fixed: fix_progress (boot-default assert), render2 + fix_objects
  (drop `gone`), fire-rate 4/40 + TooClose 2.5 + matrix 90/100.
- Green after: `fix_progress_live` 3/3, `fix_objects_live` 6/6, `render2_live` dropped_guns + the file's other
  spots, `fix_guard_weapons_live`: fire-rate ✓ too-close ✓ matrix 8/9 phases ✓.
- Neighbours: `weapon_fx_levels_live`, `fix_weapon_effects` and `walkreal_07000001_live` were started; the fx
  families and walkreal's first test pass, but walkreal's walk phase exceeds the ~6.5-min foreground window in
  this sandbox (background jobs die between tool calls here — same as the previous three sessions), so its
  batch "failure" could not be re-verified to completion in the window.
- **The full `tools/test/suite.py` did NOT fit the window** (same sandbox constraint; the direct runs above are
  the verification of record). Next session: run the suite on this tree, then chase the matrix's 9th phase.

## What's left

1. The matrix's 07000005-diff3-0x16 phase (harness engagement budget — needs the per-combo split the previous
   report suggested, or a guard that engages on that level).
2. The full suite on this tree; the walkreal batch re-check.
3. The ×2 TooClose damage arm has no headless exercise (no 07000001 guard weapon can fire inside 3.0) — covered
   by the live check below.

## Live checks for the main agent

- **PCSX2 (07000001, Agent)**: stand point-blank (1.5 units) in front of the first UZI guard (drone #0, weapon
  0x0e) and stay still. Expected: he does NOT stand and shoot — he backs out to ~8 units before firing
  (gdbwatch on `NDrone2_DSTATE_CombatTooClose` 0x173338 entered within ~120 ticks of sighting).
- **PCSX2 (any level, a pistol/shotgun-class guard — class-4-exempt weapons aside, one whose
  weapon_data+0x08 min range is < 3.0)**: let him fire from inside 3.0 units. Expected: each hit costs double
  (HITDATA+8 ×2 via `Drone_ModBulletDamage` 0x139fac-0x139fc8 comparing Drone+0x1A0 against [0x30cb14] = 3.0).
  This is the arm the headless harness cannot reach with 07000001's guards.
- **nf-view `--bond` spots** (for the merge render): no visual change is expected from this task — the only
  code changes are the difficulty boot default (menu-facing: a fresh boot without a save should show Secret
  Agent preselected on the difficulty page, 2 [M 0x1c7e78:25 @0x1c7f88]) and test files. Spot to confirm:
  boot → difficulty page shows the selection on Secret Agent (2); a direct-level load still plays on Agent (1,
  the replayed session state).

## Shared files touched (beyond the task-named `drone.rs`)

- `crates/nf-view/tests/fix_objects_live.rs`, `crates/nf-view/tests/render2_live.rs`, 
  `crates/nf-view/tests/fix_guard_weapons_live.rs` (this branch's own test file): the harness changes above —
  additive, intent-preserving, each with a comment citing the cause. No production shared file touched
  (`scene.rs`, `player.rs`, `lib.rs`, `bullet.rs` untouched this session; `bullet.rs` keeps the branch's
  `damage_factor` unchanged).
- `docs/WORKLOG.md`: appended only.
