# fix-casings-16 — the casings rule holds; 16's arrival door was the "couldn't continue"

Branch `s/fix-casings-16` (from origin/master 6cbe522). Fix commit d50cb05.

## What the playtest symptoms were

1. "I see bullet casings flying when I walk backwards" (07000016).
2. "I saw a mission complete thing pop up for some reason in this mission, but I still couldn't
   continue."

## Bug 1 — the casings: no spawn-without-fire exists; the rule is pinned

Reproduced headlessly through the real path (`nf_view::load` + `Sim::step_bond`): 07000016 and
07000005, backwards / forward / idle for 240–900 ticks with no fire → **0 casings, 0 bullets, 0
lights** every run (scratch probes; the count also never grew on the warm-arrival shape). The code
side agrees: the port's only two eject sites are `fire_player` (gated on the shot tick,
`Player_WeaponInitBullet` 0x1a4bf0:306 calls `Bullet_init_casing` 0x12fde0) and the guards'
`handle_firing` (a bullet returned = a shot, `DroneWeap_FireWeapon` 0x178f68:264) — both fire-gated,
matching the game. The user's sighting is consistent with the pool's own 240-tick life
[Bullet_casing_update 0x12faa0]: the shells from their own earlier burst are still airborne during
the backpedal — no new spawns. Test `casings16_live::walking_backwards_never_ejects`: 180 backward
ticks assert 0 casings/0 bullets, then the sister — a 12-tick burst ejects (premise), and 180 more
backward ticks never grow the pool beyond the burst's own shells.

## Bug 2 — the "mission complete thing" is the game's own arrival pair; the REAL bug was the door

- At 16's load the port queues exactly **"OBJECTIVE COMPLETE"** (label 0x2000053 — the game's own
  complete notice [M `Mission_MonitorObjectives` 0x1827d8 case 3]) for the **carried 07000015
  objective** "Find the exit to Omega Sector" — its channel 0x8e is written by the spawn plate #252
  that Bond loads INSIDE (4.14–6.21, −0.19..1.88, −67.76..−63.58; probe: `touched_by(bond) = true`
  from tick 0, ch 0x8e = 1) — and then the enabled announce **"Reach Drake's launch silo"** (0x8e is
  its enable channel). Both are the game's own behavior at arrival [D docs/levels/07000016.md §2:
  "fires as Bond arrives"]. No spurious ch 100 anywhere (900-tick watch: only 0x8e rose).
- **The real "couldn't continue"**: the arrival walkway's first door (glist 0x803f00a0 — kind 2
  slide, straddle, ch_open 0, ch_lock 0, flags 0x3 → **closed at create** [M `Door_Create` 0x136100:
  the initial state is 4 only when doorinfo+0x4c|+0x50 carries bit 2]) blocked the only path. Nothing
  in the game's own doors opens a closed idle one: `Door_Update` 0x1363c8 skips every channel arm for
  a ch_open==0 door, and `Door_ReactToHit` 0x135980 only reverses MOVING leaves (kind 0 needs
  `state & 0x18`, kind 2 needs `state & 0x10`). The game opens it through the collide-contact path
  the port has not rebuilt (U-041). The port's previous stand-in (the straddle walk-in) ran ONLY on
  the E path with a fixed 1.4² radius — Bond blocks at ~2.0 from the door origin, outside it.

### The fix

`crates/nf-game/src/door.rs` — new `Doors::straddle_walkin`: the same activation the E arm performs,
run **every tick** over `Door_Update`'s own actor list (Bond + live drones, r 0.4), with the game's
own proximity metric — `Vec_SqDist3D(door+0x80, actor+0x80) < (door+0x8C + actor+0x8C)² · 2`
[M 0x1363c8 @0x136678-0x136684, the very scan `Door_Update` runs for the auto-close] — so the open
lands exactly where the game's contact would. The lock guards hold (`activate(from_player)` rejects
a locked door with the 320 thud; the `state & 0xc` skip leaves a moving leaf alone; swing doors
(kind 0) stay E/ReactToHit-only). `crates/nf-game/src/drone.rs` — one additive call block, once per
tick, its sounds merged into the door-sound dispatch. Measured on 16: Bond's blocked pose (dist²
2.16) is inside the metric's threshold (5.71 = (1.29+0.4)²·2); the door goes 2 → 8 (opening) → 4
(OPEN) with no use key and the leaf completes its travel (trans y −0.10 → 2.18, its 2 keys [D]).

## Verification

- `casings16_live` 2/2 (both new tests).
- Blast radius: nf-game `--lib` 143/0 (the door unit tests included); `fix_weapon_effects_live` 6/6
  (the pre-existing backwards-walk test included); `walkreal_07000015_live` 1 ignored (the
  pre-existing walkreal-c blocker, unchanged).
- Full suite: this sandbox kills background jobs between tool calls, so the four-crate suite ran as
  synchronous chunks in the previous session's window (951/4/31 on that tree). This session's window
  closed before a fresh full pass on the final tree; the chunks above cover every crate the fix
  touches (nf-game lib + the door/walk/weapon live binaries). The next merge's full-suite run owns
  the rest.
- Files beyond the task's named three (the bug lives there, flagged per the shared-file rule):
  `crates/nf-game/src/door.rs` (the new fn), `crates/nf-game/src/drone.rs` (one additive call).
  `casing.rs`/`gun.rs`/`mission.rs` needed NO changes — the casings and mission layers are correct.

## Left / open

1. **Bond's forward move collapses at the now-open threshold** (z −73.58): with the leaf fully
   clear (its world box y 2.18..4.43) and `capsule_push` returning free at his pose, the walk input
   (local [0, 0.1] alive) still only crawls ~0.0002/tick instead of ~0.096 — a movement-path bug of
   its own (vel 0, lockout 0, grounded). Not root-caused in this window; the passage completion is
   the next session's first item. The door fix stands on its own (the game's door behavior is now
   right; walking into any straddle door opens it).
2. `walkreal_07000015_live`'s ignored walk (the pre-existing walkreal-c blocker) is unrelated but
   likely the same movement-path family as (1).

## Live checks for the main agent

PCSX2 (07000016):
1. Arrival: the two banners ("OBJECTIVE COMPLETE" then "Reach Drake's launch silo") — confirm the
   game shows the same pair at the same moment.
2. The first door past the arrival elevator (before the y 6.7 catwalk): walk at it with NO action
   press — expect it to slide open on contact. In nf-view the same now happens.
3. Walk-through: whether Bond passes the open threshold in the real game (the port's remaining move
   collapse, open item 1, needs the game's behavior as the reference — step into the open doorway
   and watch for the same crawl).
nf-view: `--bond 5.17,0.9,-65.3,3.09,0` walking forward — the door opens with no E (rendered logic
path, no GPU needed for the state asserts above).
