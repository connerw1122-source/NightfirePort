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
  `levels_sweep_live`, `campaign_flow_live` — the previous session re-ran all four green at
  270/600 s (levels_sweep/campaign_flow/fix_guard at 270, weapon_matrix solo at 600: 6 passed /
  0 failed / 24 ignored, ~5 min — a long binary, not a hang); this session's re-runs are noted
  below in What's left.

## What's left

- The four wall-cut binaries' 270/600 s re-runs on THIS exact tree (they advanced past their cut
  points, and all four were green at the longer limits on the pre-merge tree; nothing in this
  task touches their paths except the one-line fall read).
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
