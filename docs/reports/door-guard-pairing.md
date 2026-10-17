# door-guard-pairing — `SetDronePartner` 0x141360 + `NDrone2_DoorGuardHandler` 0x1416d8

Task 737, branch `s/door-guard-pairing` (from `origin/master` f09f3c6c — no merge needed, master
did not move). The uncited-audit runner-up: CivilianDoorGuard 0x31 existed as a const with no
arms (WORKLOG 3969); the pairing and the door-guard handler were absent.

## What's done

**The disc census [D].** 07000007 is the ONLY story level that places door guards: 6 guards whose
mode resolves to 0x16 and whose start is CivilianDoorGuard 0x31 (`DroneTypeSettings` row 22: start
0x31 [L the table dump]). Placements 404/417 (94.8/93.7, −48.0) and 441/442 (95.7, −55.4/−56.7)
are enabled and in 0x31 from level start; placements 448/449 wait disabled on switch channel 21.
Every other story level (01–0d, 11–16, 1b) has none — the census runs through the port's own
loader (`level_drones` + `start_state`), asserted by
`door_guards_pair_into_mutual_nearest_pairs`' roster premise and the level sweep in the test
harness.

**The pairing (`SetDronePartner` 0x141360).** At the 0x31 ENTER the game calls
`SetDronePartner(0x16, dc)` [M 0x160694]. Ported as a `Drones::door_guard_pass` pre-pass (the
game pairs inside the ENTER; the port's `handle` lacks the list access — one tick after the ENTER
[*S deviation, noted in the code]):
- candidates: the same mode byte 0x16, enabled (`+0x4F8 & 0x600 == 0 && & 0x100 != 0`), health
  > 0, class != 0x11, not deleted, partnerless, and the **RAW** `|yaw − yaw| < 2.0` [M
  0x141360:90 — the unwrapped ABS: the two 07000007 pairs face ~4.7 rad apart, so they never
  cross-pair; a wrapped angle difference would chain them];
- the nearest by `Vec_SqDist2D`; the pair is mutual (`+0x2BC` both sides) [M 0x141360:96, 113];
- the sign of `Vec_AngleDifference(my_yaw, atan2(dir))` picks who owns the evaluation planes: the
  diff < 0 side gets `+0x2CC = 0xff` (the mirror) and the planes are built on the PARTNER through
  −dir; else the PARTNER gets 0xff and the planes are built on the caller through +dir [M
  0x141360:123-174];
- the three `plane_equ_tag`s (`+0x2E0/0x2F0/0x300`): `plane_from_two_vectors_and_point`
  0x1e4498's cross(v0, (0,0,1)) through the anchor plus the derived 0x2F0/0x300 formulas ported
  verbatim [M 0x141360:134-173]. For a z-axis pair (441/442) the cross with (0,0,1) is the game's
  own near-zero normal — the port reproduces it (the test pins the formulas exactly, both pairs).

**The zone read (`DoorGuardHandler` 0x1416d8).** Runs in the same pre-pass (the game calls it
from the 0x31 UPDATE [M 0x160814]): no partner or no opponent → 0; the `+0x2CC == 0xff` holder
returns the partner's value [M 0x1416d8:19-21]; the planes' owner gates on the opponent (the
game's +0x170; the port's stand-in is the guard currently seeing Bond — `los` [S]) and the RAW
yaw gate, then the plane bands: s1 < 0 → 0; s2 < 0 → 0; s0 < −6 → 0; < 0 → **3**; < 2 → **2**;
< 6 → **1**; else 0 [M 0x1416d8:55-73]. The pairs' normals come out ±Y (horizontal pair axes),
so s0 compares the standing player's height against the anchor's — a player seen at the anchor's
height is zone 2, one seen below is zone 3.

**The CivilianDoorGuard 0x31 arms** (`handle_door_guard`, from 0x160648):
- ENTER: the pairing request, `+0x4F8 |= 0x10000 | 0x2000`, the return state 0x56, anim 0x24 with
  `+0xCD8 = 0x24`, `+0xCDC = 4`, timer C at frame + Rand(25)·FRAME_RATE_INT with the payload
  `+0xCC8 = 1` [M case 1]; EXIT sets `+0x44 = 1` [M case 2].
- UPDATE [M case 3]: `NDrone2_NearArmedPerson` 0x1787c8 first — an armed opponent (the port's
  gate: `los && bond_armed`, the armed set `bond_weapon` 2..0x47 / 0x6a..0x71 [M 8004's port
  arm]) turns the guard hostile through the prop-0x33 shout + Attack, and the zones never play;
  an unarmed opponent gets the zone dispatch: zone 1 → face + the idle-anim replay on the step
  change; zone 2 → the 0x5f challenge anim (variant 1) once + `ChooseSFX(0x4f, 0x50)` (the planes'
  owner only) + face; zone 3 → `ChooseSFX(0x8f, 0x92)` + `SetMissionFailReason(0xc)` + Attack;
  zone 0 → the idle replay. `+0xCDC` is the step latch (the ENTER value 4).
- TIMER_C [M case 0xc]: the idle cycle 0x24 → 0x25 → 0x26 → 0x24 by the `+0xCC8` payload, re-armed
  at frame + (Rand(20) + 5)·FRAME_RATE_INT (FRAME_RATE_INT = 30 — the port's DEAD-arm datum).
- The impact arms: 6 → HandleImpact(0x56, 0); 7/9/0x17/0x18/0x19 → (0x56, 1); 8 → (0x56, 0) [M
  cases 6-9, 0x17-0x19]; 0x11/0x12/0x14/0x15/0x16/0x1E → EnemyAlerts; 0x1F → DoForcedAttack;
  0x21 → ConsiderExplosive(0x56) [M cases 0x11-0x21]. The alerts ride the port's existing
  mission-state handling (`CIVILIAN_DOOR_GUARD` was already in those lists).

New drone fields (each on its own line): `door_partner` (+0x2BC), `door_side` (+0x2CC),
`door_planes` (+0x2E0..0x30F), `door_zone`, `door_idle_anim` (+0xCD8's 0x31 use), `door_zone_step`
(+0xCDC), `door_pair_requested`, `timer_c_payload` (+0xCC8).

## How it was verified

`crates/nf-game/tests/door_guard_live.rs` (3 tests, all pass, `--test-threads=1`, NF_ISO):
1. `door_guards_pair_into_mutual_nearest_pairs` — the four active guards reach 0x31, pair into two
   mutual pairs within the yaw gate, exactly one evaluator per pair (the 0xff mirror marker), and
   the owner's three planes match the [M] formulas exactly (both pairs, including 441/442's
   degenerate normal). Fail-first: the const had no arms — `door_partner` never filled.
2. `zone_bands_drive_the_challenge_and_the_mission_fail` — the geometry probe (the test reads the
   stored planes) finds standing spots in front of the guards: the ARMED sister first (an armed
   Bond at the same spot turns the guard hostile, no challenge — the NearArmedPerson gate [M]);
   then unarmed Bond: the challenge band fires (zone 2, `door_zone_step` 2, the 0x5f anim path)
   and never fails the mission; the restricted band (a seen player below the anchor, s0 ∈
   [−6, 0)) raises the fail reason 0xc and the guard goes hostile. Fail-first: no arms → no
   zones, no fail.
3. `the_waiting_pair_activates_through_its_channel` — pl 448/449 sit in WAIT_SWITCH (channel 21
   off, disabled [D the probe]) with no partners; Bond at their feet raises nothing; pulsing ch
   21 activates them into 0x31 and they pair [M WaitSwitch 0x15b960 case 3 → Enable(1) +
   SetState(+0x5A2)]. Sister: the pairing requires the 0x31 ENTER.

Crates run before pushing: `nf-game` (the three tests above; the crate's full run is the suite).
Full suite on the final tree: see the total line below.

## Suite total

(to be filled from the final run)

## What's left

- The zone read's opponent gate is the port's `los` stand-in for the game's +0x170 opponent slot;
  the game's slot is set by the vision system with hysteresis the port does not model — a player
  the guard SEEN but lost may still hold +0x170 in the game (the live check below settles it).
- The `+0x694 = +0xC24` copy in the zone-0 arm (0x160648:99) is not modelled (both are
  combat-bookkeeping slots the port does not carry for these guards).
- The speech lines are queued as `sfx_reqs` with the ChooseSFX ids; the port's audio layer plays
  them without the drone-voice weighting (the established voice stand-in).
- The pairing runs one tick after the ENTER (the pre-pass) instead of inside it — indistinguishable
  from tick 2 on (the positions are static at level start), noted [S].

## Live checks for the main agent

1. The pairing on 07000007 (the customs-house pair 404/417): `gdbwatch` the two `Drone+0x2BC`
   slots after the level boots — expected: each holds the other's obj pointer [M 0x141360:96,113];
   and the planes' owner's `Drone+0x2CC` should stay 0x00 while the mirror's reads 0xff until the
   first zone [M 0x141360:124/152].
2. The unarmed walk-up: walk Bond (fists) into the pair's front at the anchor's height — expected:
   the planes' owner plays anim 0x5f (the challenge) and speaks once; continuing through/past the
   pair at a lower height (the stairs) → the mission-fail reason 0xc ("Cover Lost" label per the
   port's mission_fail_reason map [M 0x13e560 case 12 → the 0x4000033/ch-99 arm]) — confirm the
   fail actually fires on this level's geometry.
3. The armed walk-up: draw any gun within their sight — expected: instant hostility (Attack) with
   the prop-0x33 shout broadcast, no 0x5f [M 0x160648:38-58].

## Usage

not visible to me
