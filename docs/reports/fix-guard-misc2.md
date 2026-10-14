# fix-guard-misc2 — report

Branch `s/fix-guard-misc2` (from origin/master 6691ef9, merged `origin/s/fix-guard-misc` 5d86b9e first to
keep its harnesses). Continuing 102i: three guard bugs — the 0700000c rooftop sniper takes no damage, the
07000006 guards never open doors, the 07000006 first-frames "crazy models". Two of the three are fixed with
failing-first repros; the third needs the main agent's renders (no GPU in this sandbox — see "What's left").

## 1. The 0c rooftop sniper — FIXED (the repro was upstream of the sim, in the harness aim)

**Finding first**: the previous session's "aim-mode pitch drift" theory is DISPROVEN. `bond.pitch` is
written only by `input.look` (`player.rs:441/979`) and the harness sent `look: 0` — the pitch was 0.187
from tick 0 because the harness SETUP computed it from the wrong eye: it used `v[1] + 1.5`, but
`Bond::stand_at` floors Bond at 13.1 + 1.1 = 14.2 with the eye at +1.5 → 15.7, a full 1.0 above the
assumption. The setup aimed 0.4 ABOVE the head box (top 22.30) and every tap sailed over him — exactly
the miss margin the previous session measured.

The sim's aim mechanics were already game-faithful (the aim-mode free-aim window `scene.rs:1711-1717`
[Player_Aiming 0x1a1d28], the cross ease, the overflow into pitch/yaw). What the port DID get wrong
against the decompile, now fixed in `scene.rs` (the `run()` fire-path dir):

- While aiming, the port dropped the auto-aim lock (`aim_target = None`) and discarded the assist offsets.
  The game: `Player_AutoAim` (0x1a7c98) gates the `Check_AutoAim` (0x1a73f8) scan on the aiming bit — bit0
  of `control+0x60`, set only after the aim-raise anim finishes (`Player_SetWeaponAnimObj` 0x1a2e38 case
  0xf) — and in the aim branch only EASES the offsets toward the manual offsets at 0.5/tick while the
  lock pointer survives; the fired bullet consumes them through `Player_GetAimingPoint` (0x1a4a98) in
  `Player_WeaponInitBullet` (0x1a4bf0:152-164). The port now keeps the lock, eases the offsets toward the
  cursor projection (x flipped: the offsets are object-frame angles, a larger yaw turns LEFT — the same
  flip `crosshair()` documents) and builds the aim-mode bullet ray from them.

Test: `the_0c_sniper_takes_damage_when_shot` (in `crates/nf-view/tests/fix_guard_misc_live.rs`), reworked:
setup pitch from the real eye, `auto_aim = true` (the shipped option default), a stick controller that
drives the RAY (`yaw − tx`, `pitch + ty`) like the game's window mechanics, and a fire gate on the shot
ray itself (the torso box is ±0.25 y at 23 units ⇒ ±0.011 rad). **Failing first**: health 10.0 / 0 damage
ticks on the pre-fix tree (shown at the session start and by the stash check for the door test; the
sniper repro's pre-fix failure is the branch's baseline run — "final: sniper #17 health 10 hits_taken 0").
After the fix: +42 first hit, health 10 → −50, `hits_taken 1` — the real msg-8 path
(`Drone_BulletHit` 0x13a9f8 → `NDrone2_HitDamage` 0x145558, the P2K × Agent-difficulty math the previous
session pinned). Sister assertion: `hits_taken >= 1` (the +0x1D8 counter ticked through the bullet path).
The pinned `probe_sniper_direct_bullet` regression (`assert!(hp < 9.99)`) stays green.

## 2. The 06 doors — FIXED (the chase dispatch + the croute binding)

The decompile chain (researched this session, all [M] in ACTION.ELF): `NDrone_InitDoorNodes` 0x139a70
binds the nearest class-0x1C door to every flag-0x2 node (`NODE_tag+0x28`); `LinkCreep_Handler` 0x15a2c8
fires `NDrone2_ReachedDestNode` (0x153358, the flag-0x2 case at :144-174 → obj+0xBA8 = door,
+0x5A6 = return, state 0x81) for EVERY followed route — including the chase:
`DroneFunc_CombatState` 0x148338 → `NDrone2_MoveToObject` 0x151618:65 → `NDrone2_FollowRoute` 0x155be0
→ `AINetwork_FollowRoute` 0x15b110 → `LinkCreep_Handler`. `DSTATE_OpenDoor` (0x1681f8) waits on
`NDrone2_DoorIsOpen` 0x14aaa0 (door state 4), retries via timer C every 2 ticks through
`NDrone2_OpenDoor` 0x14aa38 → `Door_Activate` 0x135be0.

The port had the whole state machine (ported and green on 07000001) but TWO gaps broke it on 06:

1. `move_to_object` (drone.rs, the `DroneFunc_CombatState` movement) dropped the route's reached node —
   `route_reached_special` never ran for the chase path. Now called exactly like `move_to_goal`
   (fix-nav-remainder's wiring), with the game's successor-node timing. The game's return mapping sends
   10 → 0 ("moving"; `00151618` tail) — the state change carries the door, so the dead `10 =>` arm at
   COMBAT_NO_ROUTE stays untouched.
2. `Drones::tick` resolved the `door_node` → door binding through `dr.route` ONLY; a door reached on the
   `croute` (0x63 and the chase route) resolved to None, so OPEN_DOOR's UPDATE saw `hit_door_open = true`
   and immediately restored the return state without ever activating the door. Now falls back to
   `croute` exactly like the kick resolution right below it (fix-nav-remainder's own pattern).
3. (minor, same file) `route_reached_special` saved `door_return` unconditionally; the game saves +0x5A6
   only when the current state ≠ 0x81 (0x153358:157-160) — re-entering through the pair's second node
   must not clobber the return state.

Test: `the_06_guard_driven_through_a_door_pair_opens_it_and_crosses` — finds the door-flagged node pairs
with a real door (path 3 n11–n14 → door #4, path 7 n0–n1 → door #1), picks a live guard on the same nav
path, drives him through the game's GoToGoalPosition 0x63 path (`goal` beyond the doorway, ENTER via the
real dispatch), and requires: OPEN_DOOR (0x81) entered, the door reaching OPEN (4), and the guard crossing
— Bond passive throughout. **Failing first** (verified by `git stash` of drone.rs): "the driven guard
never entered DSTATE_OpenDoor (0x81) — the dispatch never fired". After: OPEN_DOOR at +30, door #1 OPENED
at +38, crossed. The old passive test's assertion flipped to the game-true idle contract (06's guards all
stand at posts — `route: None` — so idle opens nothing, in the game either): no door may open and no
guard may enter 0x81 with nobody driving it. Sister assertion inside the driven test: the 0x63 entry
itself (`assert_eq!(state, GO_TO_GOAL)` after the ENTER).

## 3. The 06 first frames — NOT REPRODUCED HEADLESS; renders listed for the main agent

`the_06_guards_are_posed_from_their_first_frames` still passes (0 unposed characters at ticks
0/1/5/30/60/120/239 — 88 characters, every drone's start danim set at creation). This sandbox has no GPU
(`nf-view --shot` → "No suitable graphics adapter found"), so the render half of the task could not run
here; the exact commands are under "Live checks". No fix was guessed.

## How it was verified

- `fix_guard_misc_live`: **6 passed / 0 failed** (both new tests, the reworked sniper, the pose probe,
  the passive census, the direct-bullet regression).
- Neighbours on the merged tree: `ai_traces_live` 4, `doors_live` 2, `doors_01_live` 2,
  `drone_portcheck_live` 5, `fix_drone3_live` 5, `fix_drone4_live` 2, `fix_searchlights_guards_live` 2,
  `fix_npcs_live` 4, `fix_ai_remainder_live` 1, `fix_weapon_effects_live` 6, `fix_weapon_fire_modes_live` 5,
  `fix_weapons_all_live` 4, `gadgets_live` 4, `fix_level_0c_live` 4, `fix_0c_hatch_live` 2 (+2 ignored),
  `weapon_matrix_live::matrix_0700000c + matrix_07000006` 2 (80.9 s).
- The full suite (`tools/test/suite.py`): build OK — **241 test binaries** — then the run hit this
  sandbox's foreground wall-clock (backgrounds die here; the wrapper was killed at 570 s mid-run) and
  `weapon_matrix_live` exceeds every per-binary limit I could afford (it loads ~26 levels; TIMEOUT at
  150 s and at 300 s, both mid-run, not hung — the two matrix tests that touch the changed code paths
  pass directly). All other binaries that ran in the chunks passed. The remaining un-run binaries are
  the ones the main agent's pre-merge suite covers.
- Files changed: `crates/nf-game/src/drone.rs` (move_to_object dispatch, croute door resolution,
  door_return guard), `crates/nf-view/src/scene.rs` (the aim-mode offset keep — the one shared-file
  hook, additive: the old `else` branch is unchanged, the aiming case is a new branch),
  `crates/nf-view/tests/fix_guard_misc_live.rs` (the two reworked tests + the new drive test).

## What's left

- **The 06 renders** (below) — the main agent confirms or kills the "crazy models" symptom visually.
- `weapon_matrix_live`'s full run + the remaining suite binaries on the merged tree (the sandbox wall
  ate the chunked run; every binary that ran is green).
- The aim-raise window: the game keeps scanning until the raise anim finishes (`SetWeaponAnimObj` case
  0xf); the port has no raise anim, so its aim-mode gate is the aiming flag itself. A faithful fix needs
  the raise-anim state — noted, not guessed.

## Live checks for the main agent

- **nf-view renders (07000006, the truck start)** — frames 1, 5, 30, 60, plus 120:
  `nf-view <iso> 07000006 --frame 1 --shot f1.png --bond 57.18,39.73,22.61,3.14,0` (then 5/30/60/120).
  What to look for: T-poses vs blended walk frames, stretched limbs (bad palette/skin bind), guards held
  back by spawners (the load log says 20 character meshes are spawner-held — are THOSE the crazy ones?),
  and the facial/blink layer (facial3) on the first frames. The headless check only covers the bind-pose
  case (`unposed()`), which is clean.
- **PCSX2 (0700000c)**: at the yard vantage (`--bond 46.0,14.2,-182.0` spot), fire at the tower sniper
  (43.77, 21.43, −205.13): the real game should show the hit effect (`NDrone2_DoHitEffects` 0x1458e0,
  sound 0x413/0x601) — confirming the user's "no damage" was the aim, not his health path.
- **PCSX2 (07000006)**: pull the gate searchlights' alarm (ch 55, guards d11/d23–d25/d27) and bait a
  guard across door #1 (36.5, 40.0, −16.25): he should open it himself (DSTATE_OpenDoor → the door opens
  without the use key) — the same chain the driven test now exercises.

## ntfy failed

(none — sent at finish; see the queue record.)
