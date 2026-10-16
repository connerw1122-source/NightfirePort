# guard-drops-flashbang — report

Task 681, branch `s/guard-drops-flashbang`. Playtest 1011-1655 (07000005): "all enemies are
dropping multiple objects" and "the flashbang item is killing enemies even if I dont throw it
directly at them".

## What the game does (the facts the work cites)

**Drops on death.** `DroneWeap_DropWeapon` (0x17a2a0) drops the held gun once: `Pickup_CreateSimple`
of `weapon_data[w].base` with `clip_size · (0.25 or 0.5 + FRand(0.5))` rounds (the +0.5 band for
clip < 10), then clears the weapon byte, so every later call in the death/knockout/stun flow is a
no-op. The whole drop block is gated on `weapon_data[w]+0x80 != 0` — the fists record (weapon 1)
carries model 0 [L record 1], so a disarmed guard drops nothing. Weapon 0x1A additionally drops the
hidden 0x1B ammo pile. The captain grenade drop is `DroneFunc_OnInitDeath` 0x145bd0:113-145, gated
on `Drone+0x19` — set by `NDrone2_DefaultInit` 0x14b300:495-531 only for behaviour prop 9 on
difficulty 1 (5 on 2, 8 on 3) with kind ∈ {1,2,3,6,7}.

**The stun grenade (0x35).** The disc record is radius 10 / damage 0 with `+0x70 = 0x2280` [L
ee_start_05.bin + USATxt.dat label 'Stun Grenade']. Its fuse death takes `Bullet_DoTrails`
0x12d890's `& 0x2000` flash arm (decomp lines 287-383): `Gas_Init` + `Camera_Shake(2.0)` +
`Sound_Play3D(0x16)` + `Light_Create`, then Bond's whiteout from a 30-unit sphere, and for drones
`Drone_MessageObjVicinity(20.0, 0x18, …)` 0x12e0c0. **No `Explode_Create` and no damage anywhere on
this path** — a flashbang cannot kill, direct or far. `Drone_MessageObjVicinity` 0x13a438 delivers
msg 0x18 to every class-2 object within `Vec_Dist3D ≤ 20.0` that passes `NDrone2_CanSeeObject`
0x1785e0 (a head→blast line-of-sight check, 0x176f18) — no enabled/active gate. `DroneFunc_HandleImpact`
case 0x18 (0x1469b8:250-313) sends prop-0x1C guards into the flinch anims and everyone else into
StunGrenadeImpact 0x4D, whose case 1 (0x172bb0:15-18) drops the held gun and calls
`DroneWeap_ChangeWeapon(1)`. A WaitSwitch (not-yet-triggered) drone's own handler drops every
message except 0/1/3/0xe [M 0x15b960 `return 0`], but the state machine's unhandled→GLOBAL fallback
still lands case 0x18 — the game stuns the un-triggered yard guards too.

## What the port did and what I changed (`crates/nf-game/src/drone.rs`)

The playtest's flashbang-kill is already gone on master (fix-grenade-types/fix-explosions landed
after Oct 11): the player-thrown 0x35 takes the flash branch with damage 0 — no kill is possible.
The remaining gaps vs the decompile, now fixed:

1. **The flash rode a hand-rolled loop instead of the state machine.** The old code skipped
   disabled guards (the game delivers msg 0x18 to them through the GLOBAL fallback), ignored the
   per-guard line of sight (the game needs `CanSeeObject` — the old code stunned through walls),
   and applied the stun directly. Now: `msg::FLASH (0x18)` is a real message (intercepted in
   `handle()` like msg 8/6/7), delivered by the Sim's flash branch under the game's rule — class 2,
   within 20.0, head→blast LOS via `CollisionWorld::line_of_sight` (the port's stand-in for
   `CanSeeObjectFrom`; the AI-network same-cel fast path is not modelled) — and handled by
   `on_flash` (prop 0x1C immune, else StunGrenadeImpact), so WaitSwitch guards reach it exactly as
   in the game.
2. **The weapon drop moved into the StunGrenadeImpact ENTER** (its own case 1 [M 0x172bb0:15-18]):
   `if weapon != 4 { drop_weapon(d, rng); weapon = 1; }` before the alert/anim draws, in the game's
   order. This also fixes the stun-dart path sharing the state (the old inline drop only covered
   the flash branch) and removes the old RNG-swap hack.
3. **The flashed waiter re-enables** (`self.enabled = true` in `on_flash`). [S] — see Live checks.

The drop side needed no code change: the port's `drop_weapon`/`drop_grenades`/captain gate already
matched the decompile (the tests below now lock it).

## Verification (all through the real paths: `fire_player`, the Sim's grenade list, `tick`)

`crates/nf-game/tests/guard_drops_flashbang_live.rs`, 4 tests, all passing:

- `five_guards_drop_exactly_the_game_drops` — 5 guards of different loadouts (placements
  3/5/7/9/66, skins 0x2e/0x2f/0x30, SG5 0x16 and sniper 0x24), each killed by a real shot: exactly
  ONE pickup (the held gun's base id, kind WEAPON), rounds in the game's bands ([7..=22] for clip
  30, [2..=7] for clip 5), never a grenade. Sister assertions: nothing drops before the kill; no
  guard on 07000005 has captain props 9/5 (the grenade drop can't fire at difficulties 1-2).
- `flashbang_stuns_three_guards_and_never_kills` — a player flashbang (built exactly as the view's
  throw path: record 0x35, damage 0, radius 10, fuse 210, owner = the player) at the yard trio
  (pl 3/4/302, all disabled WaitSwitch guards — the game stuns them through the GLOBAL fallback):
  zero health loss at every tick, no corpse, the stun family entered, the held gun dropped and the
  fists equipped, no explosion record; one drop per flashed guard (the yard's pl 303 is in radius
  too — 4 flashed, 4 gun drops). Control guard pl 271 (far): untouched.
- `frag_at_the_same_spot_kills_the_trio` — the frag (0x34, 8/100 [L]) at the same spot kills the
  trio (health 0, death states) — the flashbang's harmlessness is the flash branch, not a broken
  blast path; one gun per dead guard, no grenade pickups.
- `a_flashed_guard_killed_after_drops_nothing_more` — the flashed yard killed DOWN by a frag while
  every guard holds the fists: the deaths add NO pickups (record 1's model 0 gate). (A guard who
  recovers first draws the sidearm [M 0x163350 case 1 `ChangeWeapon(6)`] and drops that single
  object when killed — observed (6,5) in this harness during development; the game's own one
  object per death.)

Regression runs on the final tree (branch head bfadf3e5; `origin/master` is the merge base — no
merge needed, already up to date):

- `nf-game` full: **446 passed, 0 failed** (50 test binaries, `--test-threads=1`).
- `nf-view` grenade/explosion suites (`fix_grenade_types_live`, `explode_live`, `oddjob_hat_live`,
  `fix_weapons_all_live`, `fix_death_retry_boost_live`): **15 passed, 0 failed**.
- `tools/test/suite.py` on the pre-fix tree: **1779 passed, 6 failed, 72 ignored (42 min)** — the
  6 failures are the 3 `disc_golden` tests (allowed on the slim ISO) + 3 pylon render tests
  (`probe_pylon_render`, `fix_0c_pylon_live` ×2) that need the main agent's GPU render check.
- A fresh `suite.py` run on the final tree was started at 10:50 UTC (`suite_final.out`); its total
  line is appended below if it finished inside the window.

## What's left / findings for later

- The 07000005 "multiple objects" report could not be reproduced on master with the real paths —
  every death produces exactly one object, and the two remaining sources of extra objects are the
  game's own (captain grenade drops on other levels; the recovered guard's sidearm drop). If the
  playtest meant something else, a fresh playtest with this branch would pin it.
- `Drone_MessageObjVicinity` also serves class 0x11 objects — the port has no class-0x11 list yet.
- The port's guard-side flash LOS uses `CollisionWorld::line_of_sight` only (no AI-network ray);
  documented deviation, matches the port's other vision stand-ins.

## Live checks for the main agent

1. **The flashed WaitSwitch guard's re-enable** [S]: throw a flashbang next to a not-yet-triggered
   guard (07000005's yard pair works — they wait on channels). Watch `Drone+0x4F8` and
   `obj+0xF0` (gdbwatch): if the game clears the `obj+0xF0 & 0x30` bits on the msg-0x18 path
   (predicted: yes — the guard becomes visible/drawable during the stun), the port's
   `enabled = true` in `on_flash` is right; if the bits stay set, the stunned waiter should stay
   hidden and the line should be removed.
2. **The stun recovery state** [M-adjacent]: after the 3600-tick stun of a disarmed guard,
   confirm the game runs `ChangeWeapon(6)` (the sidearm appears in hand) — StunGrenadeRecover
   0x163350 case 1's non-civilian arm — and that killing him then drops the sidearm (weapon 6,
   `clip_size(6)=16` band [4..=12]).
