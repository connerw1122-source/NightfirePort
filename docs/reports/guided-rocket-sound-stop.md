# guided-rocket-sound-stop — the guided rocket's launch sound surface: master already matches the game; pinned

Playtest 1015-1738 (07000005, the Sentinel's guided rocket): "when the rocket finishes, I can still
hear a sound being played from where I was standing".

## What this task did

Swept **every sound the guided launch starts** — the launch sound, the flight loop and the
camera-ride sounds — at the instruction level, verified master against the game's stops and
ownership, and pinned the contract with tests that fail on the old (pre-fix) behaviour. **No code
change was needed: s/rocket-sound-leak's merged whoosh fix is the complete leak fix.** Nothing in
this task changed working code to look busy.

## The full sound surface of the guided launch (the audit)

Method: for every function on the fire/ride/detonate path, scan `work/elf/ACTION.ELF` for `jal`
into the `Sound_*` functions (Play3D 0x1dd548, Stop 0x1de0a8, IsFinnished 0x1de240, SetPosition
0x1de108, Playing_Ref 0x1de3a0, Stop_Ref 0x1de370, ReqestPlaySfx 0x1dcc68, Script_SoundStart
0x1d8820, Script_SoundEnd 0x1daf60) — the decompiler can drop calls, the disassembly cannot. Then
checked each played id's LOOPED byte in `SFXOutputData` @0x2c94a0 ([L] record +0x14, entry 0x10e =
1, every other id in this report = 0).

| Path | Sounds | Loop? [L] | Game's stop/ownership [M] | Port |
|---|---|---|---|---|
| `Bullet_init` 0x12c5a0:277-282 (the fire path — `Player_WeaponInitBullet` 0x1a4bf0 has ZERO sound calls) | the launch whoosh **0x10e** at the shooter | **looped** | handle at `BU+0xF0`; moved with the rocket per tick (`Bullet_update` 0x12e7b0:98-108 `Sound_SetPosition` at the head of EVERY tick, clears when finished); stopped at `Bullet_Delete` 0x12e660:13-16 (disasm 0x12e680 `lw a0,0xf0(s2)` → 0x12e68c `jal Sound_Stop` → 0x12e694 `sw zero,0xf0(s2)`) or `Bullet_update` case 3 (0x12ed68) when stuck | FIXED by s/rocket-sound-leak, verified below |
| `GT_TakeControl` 0x181740 / `GT_LoseControl` 0x181408 (the camera ride start / hand-back) | **none** — zero `jal` into any `Sound_*` | — | the ride itself starts no sound; `Bullet_Delete`'s restore arm (:22-31, `BL+0x878 = 0`, `BL+0x944 = 0xf`) rides the same delete pass that stops the whoosh | port matches (ride start/hand-back play nothing) |
| `GT_Update` 0x180498:346/:518 (the ride's action-9 submunition fire) | **0x4e7** (variant `+0xec == 1`) / **0x10a** at the missile's muzzle | no / no | fire-and-forget one-shots — they reap themselves | the port zeroes the ride input [M 0x193600 case 10] and models neither the submunition nor its sounds — a coverage gap, not a leak (same shape as the old audit's hat 0x5ef row) |
| `Bullet_CollisionHandler` 0x12ce80 (the detonation/impact) | 0x51c (bounce < 0xF), 0x13 (smoke first bounce), 0x15 (flash first bounce), 0x4a7/0x446 (satchel/tripbomb sticks) | no | one-shots | no leak |
| `Bullet_Delete` 0x12e660:53 | **0x28d** | no | one-shot, and only on the grapple-point arm (`+0x70 & 0x40`) — never the rocket | no leak |

So the game's contract is exactly one live voice from the launch — the looped whoosh — and it
tracks the ROCKET (not Bond), dying with the bullet's delete: during the 15-tick camera restore,
before the hand-back completes. The playtest's "a sound from where I was standing" was the old
fire-and-forget whoosh, fixed on master by s/rocket-sound-leak.

## Verification

- `crates/nf-view/tests/guided_rocket_sound_stop_live.rs` — 2 tests through the real fire path
  (`load_dev("07000005.bin")` → the Sentinel 0x2c given → draw via `step_bond` → fire →
  `nf_view::sound_tick` → the nf-audio driver):
  1. `the_flight_loop_tracks_the_rocket_not_bond` — every flight tick the whoosh's tracked
     position is on the rocket (`Driver::item_pos(0x10e)` vs `player_bullets[0].pos`, drift
     < 1.0 u² — the game's one-tick `SetPosition` lag at 0.2525 u/tick) and by the last sample it
     has left the launch spot behind (the pre-fix voice sat at the muzzle: the playtest symptom).
  2. `the_voice_dies_at_the_delete_not_at_the_handback` — on the FIRST tick the bullet list is
     empty, with the camera restore still counting (`missile_cam` Some, the [M] 0x12e660:28
     `+0x944 = 0xf` window), the whoosh is already gone from BOTH layers (driver items and EE
     pool = 0); after the hand-back completes, a sweep over EVERY live driver item (any id;
     ids live before the shot forgiven) finds no voice within ~1.4 u of the launch spot.
  - RED→GREEN: with the fix's handle store reverted (`snd: 0`, the old fire-and-forget) both
    tests fail immediately — "the whoosh drifted 1.56 u² off the rocket" and "a live 0x10e item
    stayed in the driver after the bullet's delete, while the camera restore is still counting".
    Restored, both pass. The pre-existing pin `fix_rocket_sound_leak_live.rs` also passes on
    master (1/1).
- **Crate runs before push**: `cargo test --offline --release` for the touched crates' binaries —
  the suite below covers them; `guided_rocket_sound_stop_live` 2/2, `fix_rocket_sound_leak_live`
  1/1, and the four-crate suite green.
- **Full suite (fast runner, final tree = branch merged with origin/master — a no-op merge, the
  branch is origin/master + this task's tests/docs): 1878 passed, 6 failed, 72 ignored (434
  binaries)** — the 6 failures are the allowed set: 3 `disc_golden` (slim ISO) + 3 pre-existing
  pylon render probes (`fix_0c_pylon_live` 2, `probe_pylon_render` 1). Note: this sandbox kills
  background processes between tool calls, so the runner executed in the foreground in
  resume-chunks (`scripts/run_suite_chunk.sh`, per-binary `--test-threads=1`, results in
  `scripts/suite_grs_results.txt`).

## What's left

- Nothing for the leak. The one candidate for a future task: `GT_Update`'s ride submunition
  (action-9 drops `weapon_data` @0x2c55d0 = record 0x60 with sounds 0x4e7/0x10a [M :346/:518]) —
  the port's ride zeroes the input, so the game's ride-fire behaviour (and those one-shot sounds)
  is unmodelled. Sound-coverage, not a leak.

## Live checks for the main agent

1. The stop rides the delete, not the hand-back: gdbwatch `Sound_Stop` (0x1de0a8) through one
   guided Sentinel shot on 07000005 — expect exactly one call shortly after the rocket's death
   tick (from `Bullet_Delete` 0x12e68c), $a0 = the 0x10e play's handle; audibly the whoosh dies
   with the rocket while the camera is still restoring.
2. The whoosh follows the rocket: gdbwatch `Sound_SetPosition` (0x116118… the entry stub is
   0x1de108) — one call per flight frame with the same $a0, position advancing with the rocket;
   audibly the loop leaves the launch spot with the missile (pre-fix it stayed).
