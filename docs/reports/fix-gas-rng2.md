# fix-gas-rng2 — landing 110o: the collide-pass draw order + the puff's real lifetime

Branch `s/fix-gas-rng2` (master a2958c03 + 110o `s/fix-gas-rng` 119e3019 merged at 9bb2d029).
Files: `crates/nf-game/src/drone.rs`, `crates/nf-view/src/scene.rs`,
`crates/nf-view/tests/models_misc_live.rs`, this report, WORKLOG.

## What the game does (the entries this implements)

The frame is two passes over one newest-first object list: the update pass
(`control_movement_object_handler` 0x1336f8 calls `control_funcs[class].update` head→tail), then
the collide pass (`Collide_Update` 0x1ea398 at 0x1336f8:238 calls `control_funcs[class].collide`).
New objects insert at the list HEAD (`control_create_object` 0x1344e0 → `control_add_object_to_list`
0x134698 after the 0x2705a0 anchor), so bullets and puffs update/collide before the older player and
drones. Snapshot check [L ee_start_05.bin, the list at 0x2705b4]: a class-2 drone at index 36, the
class-3 player at 286, four more class-2 drones at 338–364.

Per shot therefore: the bullet's spread draws run in the update pass (Player_Update 0x193600:411-413 →
WeaponFiring 0x1a5460:690 / WeaponRecoil 0x1a3de8:59 → SetWeaponAnimObj 0x1a2e38:419/519 →
WeaponInitBullet 0x1a4bf0, which latches `+0x95b` :188/:203); the muzzle light + smoke (6 draws) run at
the same tick's collide pass (Player_CollisionHandler 0x1949a8:189-191 → Player_MuzzleFlash 0x1a4298:
Light_Create, the 2π draw 0x1a43ec, `Gas_Init` 0x1aacc8's 5); the bullet impacts run in the collide
pass too (Bullet_CollisionHandler 0x12ce80 → Effect_Create/Effect_Bullet/Effect_RicochetProb), ahead
of the player's MuzzleFlash (the bullet heads the list, the player sits deeper).

The puff's lifetime is the decompile-dropped stack word `lhu 0xc0(sp)` 0x1aad24 = 0x80 = 128 ticks;
`gases_smoke` 0x1ab2d0's tail decrements it once per tick (`−FRAME_RATE_MUL`) and kills the object
(`obj+0xFE |= 1`) at ≤ 0.

## The fixes

1. **The player bullets' impact effects moved to the collide pass** (drone.rs). `update_player_bullets`
   used to call `Effects::bullet_impact` mid-flight (mid-update, before every guard's AI roll). The
   call is now recorded (`pending_world_impacts`, with the bullet index, hit, normal, material, dir,
   flags70, bounces, laser flag) and flushed at the end of `Drones::tick` — after the guard control
   loop, following the guard bullets' own impacts (their loop already sat post-loop, matching the
   game's newest-first collide order). The flush replays `Bullet_CollisionHandler`'s order: the
   effects (their RNG draws) → the explode gate (`+0x70 & {4, 0x100000, 0x200000}`, non-ricochet,
   [M 0x12d068-0x12d188]) → else the ricochet re-aim [M 0x12d704-0x12d7f0]. The dead-bullet `retain`
   moved from `update_player_bullets` to the flush so a deferred ricochet revives its bullet IN PLACE
   (the list order, and with it the per-bullet draw order, is preserved).
2. **The muzzle light + smoke latch** (scene.rs). The fire section no longer draws; it latches
   (muzzle pos, light radius, rgb) — the `+0x95b` equivalent — and the post-`drones.tick` hook
   consumes it: `lights.create` (its draw), the 2π draw, `init_muzzle_smoke` (5). The 0x2000 no-smoke
   gate stays [M 0x1a42e8]. Net stream position: after the guards' update-pass rolls and after the
   impact flush — the game's collide-pass slot.
3. **The muzzle puff's lifetime re-pinned to the game's** (models_misc_live.rs). The old pin (die
   within 20 ticks, `ticks == 7`, "life 8 [S]") predated the ABI finding. Now: the puff is captured
   fresh (the latch consumption runs after the pool tick) and must die after exactly 128 pool ticks
   (`ticks == 128`, guard < 200) — the drawn lifetime IS the field the game sets from its draw, and
   the pool kills on time [M 0x1aad24, 0x1ab2d0 tail]. GREEN.

## Verify

- `models_misc_live::firing_spawns_a_muzzle_smoke_puff_that_fades_and_dies` — FAILED pre-fix (the
  puff outlived its life at the 20-tick guard), GREEN post-fix with the exact 128-tick death.
- nf-game lib 177 passed / 0 failed; `ai_live` 34/35 (only the known cover failure below).
- The held failures reproduced on the branch before the fixes: ai_live `UnderCoverAim lasted 90
  (live 37)` (line 249), models_misc `the puff outlived its life` (line 325), fix_guard_weapons
  `no candidate guard with weapon 0x12 completed the collection`, walkreal contact route stall —
  all four re-run and confirmed before any edit.

## What's left — the two AI failures are NOT fixed by the order moves

`ai_live::guard_takes_cover_like_the_game` still measures 90 (window 32..=40), fix_guard_weapons and
walkreal still fail. Findings from this session narrow the cause:

- The `UnderCoverAim` phase runs BEFORE the guard's first shot (its own smoke draws cannot reach it),
  so the shift rides the draws around Bond's fire and the pre-cover rolls.
- Experiment (reverted, uncommitted): consuming the fire-script op's smoke draws
  (AnimProcessScriptCmds 0x1c1b38:455-458 — the 2π + `Gas_Init`'s 5) stream-only in
  `DroneWeap_FireWeapon`'s port left the cover test at 90 — consistent with the aim phase preceding
  the guard's first shot; it neither confirms nor denies the op for guard shots.
- The nf-game test path has NO muzzle smoke draws for Bond (the latch lives in nf-view's
  `step_bond`), while the game draws 6 per Bond shot at the collide pass. Master matched the live 37
  with NO puff draws at all; 110o added the impact-puff draws and the test went to 90; my collide-pass
  re-positioning did not recover 37. The remaining candidates, in order: (a) the nf-game path's
  missing Bond-side muzzle draws (6/shot at the collide slot), (b) the guard-shot draw set (does the
  guard's shot carry the anim-op smoke — 6 more), (c) a residual update/collide split inside
  `drones.tick` (the grenade/DoTrails section runs after my flush; in the game it is update-pass and
  would precede the collide draws).

**Live checks for the main agent** (PCSX2 + gdbwatch on SEED_X/SEED_Y 0x30d0a0/0x30d0a4,
07000005 courtyard):

1. Bond fires ONE pistol shot at a wall, no guards alerted: count the seed advances across the tick.
   Expected per the decompile: spread draws (Bullet_init) in the update pass + 6 at the collide pass
   (light + 2π + Gas_Init 5) + the impact's own (debris/puffs/ricochet as the surface record says).
   This settles (a): whether the port's headless path must carry Bond's muzzle 6 to match the stream.
2. A guard fires ONE shot at Bond (Bond idle, at range): same count for the guard's shot. Expected
   per the anim-op path: accuracy roll + Bullet_init spread + light + 2π + Gas_Init 5. This settles
   (b): whether `DroneWeap_FireWeapon`'s port must add the op's smoke draws (fix-guard-weapons3's
   sprite-branch finding concerns 0x17925c's THREE draws, not this op's 6).
3. fieldprobe the fresh muzzle puff's data+0x38: expected 0x80 = 128 (already cited; re-confirms the
   life pin this report re-pinned the test to).

nf-view `--bond` spots (render): fire one shot and hold — the muzzle puff should read as a small dust
quad at the muzzle growing 0.05→0.1 over ~2.1 s (128 ticks) while fading; the impact puffs should hug
the wall and rise only slightly.

## Suite status

Not run to completion this session (the sandbox reaps background jobs — nohup builds and runners die
between tool calls; the foreground chunks completed: nf-game lib 177/0, ai_live 34/1, models_misc
life test green, fix_guard_weapons still failing, walkreal not re-run after the fixes — 400 s binary).
The full four-crate run on the merged tree is the next session's first job, with the driver's
expected total on the line.

## Shared files touched (additive, per the merge rules)

- `drone.rs`: the `pending_world_impacts` field + the flush block at the end of `Drones::tick`; the
  world-arm call → record; the retain move. No existing arms renamed; no constants added.
- `scene.rs`: the `muzzle_latch` field (own line in the struct and the constructor), the fire
  section's draw block → latch, the post-`drones.tick` consumption hook.
- `models_misc_live.rs`: the life pin's comments + assertions only.
