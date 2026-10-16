## Your task: `difficulty-effects`, Operative / Agent / 00 Agent

Audit everything the selected difficulty changes in the game (`GameState+0x40` from the difficulty table 0x2df4c0
+0x0c; find every read): enemy damage/accuracy/reaction/health, Bond's damage taken, armour, ammo, hints on/off, the
score's difficulty bonus, objectives per difficulty, continue health boosts (`ContinueHealthBoost*`). List each with
its address and the port's status; implement the missing ones in the sim. Tests per effect (numbers only) with
sister assertions.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/difficulty-effects`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox5 at 2026-10-16T00:03Z

---
## Report (done, 2026-10-16T01:41Z)


---
## Report from the worker (done, 2026-10-16T01:41Z)

# difficulty-effects — report

Branch `s/difficulty-effects`. Task: audit everything the selected difficulty
(Operative / Agent / 00 Agent) changes, list each with its address and the
port's status, implement the missing ones in the sim, tests per effect
(numbers only) with sister assertions.

## The one address fact everything hangs on

`GameState+0x40` in the task brief is a Ghidra **DECIMAL** print: hex **+0x28**
= EE **0x2a3790** (`GameState` global 0x2a3768, size 88 — a hex reading would
put `_76_4_` out of bounds; verified against the EE snapshots: `[0x2a3790]=1`
in all four). `GameState+0x40` HEX (0x2a37a8) is a different field entirely —
the one-frame skip-`Game_Draw` byte written only by `GameFlow_PushState`
0x1c9f38 / `GameFlow_PopState` 0x1c9eb0. The port's `Progress::difficulty`
already used the correct address. Three docs cited the wrong address — fixed
in this branch (`docs/systems/progress.md:26,117`, `docs/systems/mission.md:206`,
`docs/reports/fix-death-retry-boost.md:11`).

## The audit (every effect, its game address, the port's status)

The difficulty value comes from the 3×0x18 wheel table at 0x2df4c0 (+0x0c =
value 1/2/3 = Operative/Agent/00 Agent; +0x10 = the unlock flag), written by
`C_SBNFDFCTY_Handler` 0x216ca8:15. Writers: bootup 0x1c7e78:25 (default **2**),
P_MPCONFIRM 0x20ec00:150 (MP = 1), Menu_RunMiniMission 0x1ffb90:22 (1),
Boot_LoadPTPData 0x1c8560. Every read found by decomp grep of
`GameState._40_4_` ∪ a full-ELF scan of 0x2a3790:

| Effect | Game | Port status |
|---|---|---|
| Damage Bond takes ×0.6/0.7/1.0 (d4 ×2) | `Player_HandlePain` 0x1902c8 @0x19040c | was implemented + tested (`bullet.rs` damage_factor) |
| Damage enemies take ×1.5/1.0/0.8 | `NDrone2_HitDamage` 0x145558 @0x145838 | was implemented + tested (`damage.rs`) |
| Enemy hit chance ×0.8/0.9/1.3 | `DroneWeap_DoBulletAccuracy` 0x179740 @0x179848 | was implemented + tested (`level_accuracy`) |
| Bond auto-aim cone ×1.9/1.0/0.0 | `Check_AutoAim` 0x1a73f8 @0x1a761c | was implemented (G-006); the `Player_InitWeapon` 0x1a7ef8 @0x1a7f64 distance caps are write-only globals — still deliberately not threaded |
| Enemy "captain" elites (props 9/5/8; ×2 health/damage, accuracy ÷2) | `NDrone2_DefaultInit` 0x14b300 | was implemented (`captain_setup`) |
| Enemy reaction time | `DroneFunc_ReactionTime` 0x14ac60 — **no difficulty read** | correctly neutral; no hook added |
| Extra enemies per difficulty | `Drone_Create` 0x136be0:49 — `level_tag+0x34` (param 2) `<= difficulty` | **MISSING → implemented** (below) |
| Hints on/off per difficulty | `Hint_Create` 0x1b3248:12-25 — stage gate + the `DAT_002a37cc` master switch (bootup sets 1; not difficulty) | **MISSING → implemented** (below) |
| Door locks auto-unlock on Operative | `Lock_Create` 0x1ae1f8:67 → `Lock_Unlock` 0x1b0650:28-31 (state +0xf4=1) | **MISSING → implemented** (below) |
| Copter HP ×1.0/1.5/2.0/4.0 ×299.0 | `Copter_Create` 0x17d340:76-94 | **MISSING → implemented** (below) |
| Difficulty-tagged sound triggers | `SoundTrigger_Create` 0x1b2628 | was implemented (`sound.rs` record+0x44) |
| Retry health floor (`ContinueHealthBoostEasy/Medium/Hard` 0x30cc34/38/3c, all 50.0) | `Player_RamLoad` 0x1957a8:21-34 | was implemented + tested (`continue_health_boost`, `RamSave::continue_boost`) |
| Score difficulty bonus ×0.0/1.0/2.0 @0x2bc910 | `PlrStat_GetScore` 0x1984b0:294-300 | was implemented + tested (`ScoringTableDifMul`) |
| Objectives per difficulty | `Mission_Init` 0x182270 — **no read** | correctly neutral (differences ride level tags) |
| Armour / ammo amounts | `Pickup_Handler` 0x18e650 — **no read** | correctly neutral |
| Front-end pick → sim | `C_SBNFDFCTY` 0x216ca8:15 | **MISSING → implemented** (below) |

## What I implemented (the missing ones)

1. **Front-end → sim wiring** — `menu.rs` Difficulty-page Confirm records the
   1-based pick (`requested_difficulty`, the game writes GameState at the
   confirm [M 0x216ca8:15]); `App::selected_difficulty` (default 2 = the boot
   default [M 0x1c7e78:25]) adopts it in `menu_after`; every front-end load
   goes through the new `load_with_carry_difficulty` /
   `load_with_carry_weapons_difficulty` into `Scene::build(..., difficulty)`,
   which sets the difficulty BEFORE `attach_level` (all create-time gates
   read it there) and into the `Data` the roster is created from. The plain
   `load*` wrappers keep difficulty 1 — the documented replay parity for
   direct loads.
2. **Drone spawn gate** — `level_drones` skips a 0x0F placement when
   `param(2) > difficulty` (u32, pre-create, no RNG draws — the game's exact
   gate) [M 0x136be0:49].
3. **Lock Operative auto-unlock** — `Lock::create` records param 6
   (level_tag+0x44) and `unlocked = param6 != 0 && difficulty == 1`; an
   unlocked lock takes no burn [M 0x1ae1f8:67-69, 0x1b0650:28-31]. The
   module doc's "a loaded save" misreading of the gate is fixed.
4. **Hint stage gate** — `Hint.stage` (param 2) + `hint_stage_gates`
   (verbatim decomp condition: refuse `stage != 0 && stage < difficulty`,
   then 5/6/7 = exactly d1/d2/d3; so 1..4 are difficulty CEILINGS — stage 1
   shows on Operative only), applied in `hints(level, difficulty)` [M
   0x1b3248:12-25].
5. **Copter HP** — `Copter.hp = copter_hp_mult(d) × 299.0` (1.0/1.5/2.0/4.0)
   [M 0x17d340:76-94].
6. **Doc errata** — the drone gate is the min-difficulty tag, not a "spawn
   frame" (`docs/fields/level-tag.md:34`); the three 0x2a37a8 address fixes
   above.

## Tests (numbers only, each with a sister assertion)

- `crates/nf-game/tests/difficulty_gates.rs` (no disc): lock param-6 gate
  flips exactly at 1→2; copter HP 299.0/448.5/598.0/1196.0 (+ the d0
  fall-through); the full hint-stage table row by row.
- `crates/nf-game/tests/difficulty_spawn_gate_live.rs` (ISO): **07000011**
  (found by scanning every level's main map — it tags 13 placements param2=2
  and 4 param2=3): rosters **20 / 33 / 37** at d1/2/3 — each difficulty adds
  exactly its tagged placements. Sister: the pre-task code gives 37/37/37.
- `crates/nf-view/tests/difficulty_wiring_live.rs` (ISO): wheel Confirm →
  `requested_difficulty = Some(1/2/3)`; `load_with_carry_difficulty` →
  `sim.drones.difficulty` + `progress.difficulty` mirror + rosters 20/37
  end to end. Sister: the pre-task build hardcoded 1.
- Regression fixed on the way: `enemies_07000012_live.rs` indexed
  `drones.list[20]` — a param2=2 reinforcement that Operative legitimately
  never spawns (the gate working as the game does, roster 16 not 32); the
  walkthrough now loads at 00 Agent via `load_with_carry_difficulty` where
  the full roster exists. `gadget.rs` / `sound_coverage_live.rs` gained the
  two new `Lock` fields (both `false`).

## What I ran (before the final push)

`cargo test --offline --release -j2 -- --test-threads=1` per crate:
- **nf-game: all green** (212 unit + every test binary, incl. the three new
  difficulty ones — zero failures).
- **nf-view: all green** after the 07000012 fix (116 passed).
- **nf-formats: only the 3 `disc_golden` failures** — exactly the slim-ISO
  allowance.
- **nf-audio: green** (from the earlier full run).
- The fast runner (`tools/test/suite.py`) cannot survive as a background job
  in this sandbox (both background attempts died between tool calls with
  0-byte logs); the per-crate foreground run above covers the same crates.

## What's left

- The difficulty→DRIVING.ELF path rides PTPDATA+0x984 (Boot_GetPTPData
  0x1c8198:14-22, forced 1 for the mini-mission); the DRIVING-side read
  (main 0x174bf0's 1→0/2→1/3,4→2 mapping, WORKLOG:13227) needs a DRIVING
  decomp pass — out of this task's sim scope.
- `Player_InitWeapon`'s per-difficulty `MAX_AUTO_AIM_DIST_*` stay unthreaded
  (write-only globals — unchanged, documented decision).
- The suite's disc_golden trio: slim-ISO allowance, unchanged.

## Live checks for the main agent

1. Difficulty table poke: with a mission loaded, poke `[0x2a3790] = 1` vs `3`
   (fieldprobe) before a restart — expect 07000012-like levels to show the
   extra Agent/00 patrols (roster 16 vs 32) and the Operative run to show
   fewer guards; screenshot from the same bond spot both ways.
2. `Lock_Create` param-6 tag: none of the shipped levels' 0x2E placements
   were scanned for param 6 ≠ 0 — if any exists, expect it open on
   Operative only; worth a `--bond` look only if the scan finds one.
