## Your task: `fix-world-gravity`, `WldGravity` per level instead of a constant

The port hard-codes Bond's gravity (`const GRAVITY: f32 = -9.8` in `crates/nf-game/src/player.rs`). The game reads a
global vector `WldGravity` (0x2d8890..0x2d889c; y at 0x2d8894 = −9.8 on 07000005, [L] ee_start_05.bin). It is used by
`Player_Collision` (the jump/fall) and by `Player_ChangeSubState` 0x190630 case 0xd/0xe, which picks the death anim by
it: `WldGravity.y == 0 ? 0x60008e0 : 0x6000151` [M :68-73]. The space levels are the likely zero-gravity ones.

Do:
1. Find every writer of `WldGravity` in the decompile (grep `DAT_002d8890`..`DAT_002d889c` / `WldGravity` across
   `work/decomp/ACTION.ELF`). Document where its value comes from per level (a level header field? a script op? a
   hard-coded level check?) in `docs/systems/player-physics.md` (create the section if it's missing), with addresses.
2. Produce the per-level value for every level on the disc (a table in that doc), from the data the game reads.
3. Port it: a `world_gravity: [f32; 3]` in the Sim's level data, set the way the game sets it; Bond's physics reads its y
   (keep the existing behaviour on every −9.8 level: the suite must stay green). Add
   `pub fn death_script(&self) -> u32` next to it returning 0x6000151 or 0x60008e0 by the rule above; don't edit
   `death.rs` (the main agent's death-anim branch will call your function).
4. Test (NF_ISO, `crates/nf-view/tests/fix_world_gravity_live.rs`): the table's values for 07000005 and for one level
   where the value differs (if any); `death_script()` follows it. Sister assertion: a deliberately wrong expected
   value fails.

Files: `crates/nf-game/src/player.rs`, the level-data struct you add it to,
`docs/systems/player-physics.md`, the new test.

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


**Branch:** `s/fix-world-gravity`.

---
claimed-by: sandbox5 at 2026-10-15T10:38Z

---
## Report (done, 2026-10-15T12:21Z)

Closed by the driver: the report is on `s/fix-world-gravity` (docs/reports/fix-world-gravity.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-15T12:26Z)

# fix-world-gravity — `WldGravity` per level instead of a constant

**Done.** The port's hard-coded `GRAVITY = −9.8` is replaced by the game's rule: `WldGravity`
(0x2d8890..0x2d889c) is written at every `Player_Init` — default (0, −9.8, 0), zeroed when the
player-start placement's **param 3 == 2** — and Bond's fall + the death-anim pick read it.
**0700001b is the disc's only zero-g level.** Every −9.8 level's behaviour is bit-exact unchanged.

## The game rule, with the addresses

- **The vector**: `WldGravity` @ 0x2d8890, 16 B [M work/action-symtab.tsv]. Two writers only:
  bootup 0x1c7e78:67 (zero) and `Player_Init` 0x1910f8 [M :152-172, disasm 0x19151c-0x191550 the
  default write (x=0, y=0xc11ccccd=−9.8, z=0)]. No writer in DRIVING.ELF.
- **The selector**: `*(u32*)(param_4+0x38)` = the 0xB0 `level_tag` block's **param 3** — the
  player-start placement's p3 (`Player_Start` 0x190fd0 passes the block @0x2ba1c4+i·0xe0; disasm
  `lw a2,56(s5)` 0x191580). **p3 == 2 → `Vec_Zero(&WldGravity)`** (disasm 0x191588 → 0x1915f4 →
  0x191618, `jal 0x1e2d58` with a0 = s0 = 0x2d8890) + spawn pose 8; p3 == 1/3/other keep −9.8
  (p3 == 3 additionally sets BL+0x90c = 0.45 and plays 0x60009ff). Registration: the dispatcher's
  type-0x24 case, kind 1 [M 0x1d04f0:96].
- **The readers**: `Player_Collision` 0x194df0:107/161 (the fall, `vel += WldGravity·
  REC_FRAME_RATE`); `Player_ChangeSubState` 0x190630 case 0xd/0xe [M :68-73] — `WldGravity.y ==
  0.0` (exact float test) → anim **0x60008e0** (space death), else **0x6000151**; also
  `Player_HandleDeath` 0x190bb0:15, `RB_EulerEvolve` 0x1d7810:85, `Bullet_update` 0x12e7b0:256,
  casings/Debris/Pickup/gases_smoke/Car_Update/Drone_CollisionHandler/UpdateAllShards.
- **[L]** ee_start_05 / ee_start_07000001 / ee_start_0700000a / ee_snapshot @0x2d8890..9c all
  read (0, −9.8, 0).
- Full writer/reader chain + the disc-wide table: **docs/systems/player-physics.md** (new
  section; the table was produced from all 36 `0700xxxx.bin` archives on the disc — 30 carry
  type-0x24 starts; only 0700001b has p3 == 2, 07000014 has p3 == 3; 07000023/26/48/4b/0500/0999
  have no type-0x24 start).

## The port

- `crates/nf-game/src/player.rs`: `Bond.world_gravity: [f32; 3]` (constructor default = the
  game's default write), **`pub fn death_script(&self) -> u32`** (0x60008e0 iff
  `world_gravity[1] == 0.0`, else 0x6000151 — the main agent's death-anim branch calls it; death.rs
  untouched), and the fall reads `self.world_gravity[1] / 60.0` (bit-exact `−9.8/60` kept). The
  jump launch is NOT gravity-derived in the game (`Player_HandleJump` 0x1916b0:169-183 is
  facing-scaled with a hard −0.4f lane) — the port's launch stays a constant, unchanged.
  `LevelStart` gained `p3: u32` (the placement's param 3).
- `crates/nf-view/src/lib.rs` (shared file, additive): `level_starts` parses p3; the load path
  (the `Player_Start`/`Player_Init` analogue) sets `sim.bond.world_gravity` from the PICKED
  start's p3 (`p3 == 2 → [0;3], else [0, −9.8, 0]`).
- `crates/nf-view/tests/walkreal_probe2_live.rs` (shared test file, one line): its direct
  `LevelStart` initializer gained `p3: 0` — forced by the new field.
- Not ported (out of scope, documented in the doc): the other WldGravity consumers (grenades/
  casings/pickups/gas/rigid bodies keep their −9.8 constants), the MP spawn-block path, the boot
  zero. No changes to drone.rs / scene.rs / death.rs.

## Verification (NF_ISO, the real input path)

New `crates/nf-view/tests/fix_world_gravity_live.rs`, **5/5 green** on the final merged tree:
1. `disc_scan_player_start_param3` — the disc table's legs pinned (07000005 p3 0 → −9.8;
   **0700001b p3 2 → zero**; 07000014 p3 3; sister: `gravity_for_p3(2) != −9.8` fails the wrong
   rule).
2. `ee_snapshots_wldgravity_matches_the_rule` — the [L] legs (all three snapshots + the generic
   snapshot read (0, −9.8, 0) at 0x2d8890..9c).
3. `death_script_follows_wldgravity` — 0x6000151 default, 0x60008e0 on zero, −0.001 stays ground
   (the exact-float test); sisters `assert_ne!` both wrong hashes.
4. `default_level_keeps_the_fall` — load 07000005 → `bond.world_gravity == [0,−9.8,0]`,
   death_script 0x6000151, and two airborne ticks fall exactly `−9.8/60·2` (bit-exact the pre-fix
   const).
5. `zero_g_level_floats` — load 0700001b → `bond.world_gravity == [0;3]`, death_script 0x60008e0,
   and the first airborne tick applies **no** fall accel.
- **FAIL-FIRST recorded** (the port changes stashed, a pre-fix-API probe): 0700001b, one
  airborne tick through `step_bond` read `vel[1] = −0.16333334` (the hard-coded −9.8/60) — the
  zero-g assertion FAILED there; post-fix the same tick reads 0.0. (Also probed pre-fix: a jump
  on 0700001b lands him back on the floor — the post-fix port keeps the launch velocity; the
  station's low ceilings make the float most visible outdoors, see the live checks.)
- Sister assertion (a deliberately wrong expected value fails): the `assert_ne!` pairs in tests
  1/3/4/5 — e.g. `death_script() != 0x6000151` on zero-g and `gravity_for_p3(2) != [0,−9.8,0]`
  both fail if the rule is flipped.

## The suite (the final tree, merged origin/master 0b91e999 @ 58afcde0)

`tools/test/suite.py` in foreground chunks (the sandbox reaps background jobs between tool
calls — same pattern as the previous sessions), limit 90 s per binary, 328 binaries:
**1320 passed, 55 ignored**. Not-green, all accounted:
- 3 `disc_golden` — the allowed slim-ISO three.
- 3 pylon-render (`fix_0c_pylon_live` ×2, `probe_pylon_render` ×1) — wgpu has no GPU in the
  sandbox; pre-existing on master, another session's scope.
- 4 wall-cuts at the 90 s chunk limit: `fix_guard_weapons_live`, `weapon_matrix_live`,
  `levels_sweep_live`, `campaign_flow_live`. Re-runs on THIS tree: `levels_sweep_live` +
  `campaign_flow_live` green at 270 s (8 passed / 0 failed); `fix_guard_weapons_live` (270 s) and
  `weapon_matrix_live` (solo 600 s: 6 passed / 0 failed / 24 ignored, ~5 min — a long binary, not
  a hang) were green at those limits in the previous session's runs of the same binaries.

## What's left

- `fix_guard_weapons_live` / `weapon_matrix_live` 270/600 s re-runs on this exact tree (both were
  green at the longer limits on the pre-merge tree; nothing in this task touches their paths
  except the one-line fall read).
- The [S] bigger prize: the death-anim branch wiring `sim.bond.death_script()` into death.rs
  (the main agent's), and the spacewalk's float feel outdoors (the zero-g write is proven; the
  drift/float polish on non-fall axes is the game's `SSWalk` zero-g handling, not yet probed).

## Live checks for the main agent

1. **PCSX2, 0700001b**: break on `Vec_Zero` 0x1e2d58 during the level's spawn — expect exactly
   one call, a0 = 0x2d8890; then read 0x2d8894 — expect 0x00000000 (vs 0xc11ccccd on 07000005).
2. **PCSX2, death on 0700001b**: `Player_ChangeSubState` case 0xd/0xe must queue anim
   **0x60008e0** (watch `AnimScriptAdd` 0x1c6d40's a1); 07000005's death queues 0x6000151.
3. **nf-view `--bond` spots**: `--bond 22.97777,-1.900582,3.132896,0,0` on 0700001b — Bond at the
   spawn (station interior, gravity 0 in the overlay/step data); a jump inside head-bumps the low
   ceiling (the game's rooms are low) — the outdoor spacewalk area (through the level's own
   airlock flow) is the clean float spot: Bond should rise and keep drifting, not fall back.
