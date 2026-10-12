# fix-actors — the non-guard actors

Branch `s/fix-actors`. The brief: inventory every non-guard actor type, port the allies/civilians/VIPs/bosses
and the spawners that bring them, with tests through the real input paths.

## Done

1. **The inventory** — `docs/systems/actors.md` (new): `DroneTypeSettings` (0x29aee8) and the state-function
   table (0x29b300) decoded from the EE snapshot [L], the `GetDTYPENEW` (0x14c938) special-type cases [M],
   the mission-fail rules, the 593-drone sweep of all 24 story levels aggregated by dtype/mode/team/skin/level
   (the party girls, the escort VIPs Mayhew/Zoe/Drake, the tower staff, the 06 truck driver, Rook, the
   assassin, the per-level guard rosters), the spawner parameter contract, and the port status with the
   stand-ins listed.
2. **The protected-actor mission-fail rules** (`ally.rs`, new; hooks in `drone.rs`): `SetMissionFailReason`
   0x13e560 and `OnInitDeath` 0x145bd0's fail half — the 85 drones carrying props 0x53/0x54 (every party
   guest, the office/hotel staff, the VIPs, the 06 driver) now fail the mission when they die, "Killed
   Civilians" (team 2/3) or "Cover Lost" (team 1), through the real death path with the killer identity
   (Drone+0x2B4) deciding reason 1 vs 3. Verified: killing a 07000007 party guest raises ch 0x60, sets the
   FailLabel and MissionState 2; the sisters (a near shot, an unprotected guard) leave the mission open.
3. **The ally states** (`drone.rs` arms, the scan in the tick): AllyLeadPlayerInWay 0x15ea28, AllyLeadHide
   0x15ec08 (cower, the 14-unit clear band, the 2-tick resume), AllyLeadMissionWait 0x15efc8,
   AllyLeadBondCombat 0x15f120, AllyFollow 0x24–0x27, AllyGoToGoalPosition 0x15f938. Reading the real
   `AllyLead` update (0x15e698 case 3) showed the armed-drone check gates the whole route update — the old
   port walked through fire. Verified: the escort cowers when a hostile fights within 10 and resumes the lead
   once the 14-unit band clears (the shared 07000001 escort test now resolves the fight the player would).
4. **The spawner arm fix** (`spawners.rs`): `DroneSpawner_Create` 0x1378d0 puts param 7 at +0x14 — the ARM
   channel (`Control` 0x137e68 latches `ch[p7] → armed`); the port read it as a kill channel and removed the
   spawner when it fired — 07000011's stairwell spawner died instead of arriving (the C2-class "the guards
   never arrive"). The stop channel now un-triggers/pauses/deletes exactly as the game does. Verified: the
   stairwell touch (#285 → ch 0x1D) brings the wave in.
5. **The trigger contact volume** (`triggers.rs`, one function): 07000011's arm touch #183 sits at the
   standing floor — the port's hip-high capsule could never reach it; the player's trigger contact now covers
   the body (the game's contact sphere reaches the feet).
6. **The boss**: the assassin (07000004) pinned — dtype 15, the ATTACK start, death channel 0x54 → "Defeat
   the assassin" completes on the kill; the sister (a miss) leaves it open. Rook (07000014) was already
   driven end-to-end by the map-tower2 walkthrough.
7. **The civilians**: the party states (PartyGirl 0x2F for the girls, Civilian 0x11 for the wanderers — the
   port already matched the disc), the scared radius (20.0, 10.0 on 07000007) and the scared chain were
   already ported; the C3 "guests can be shot" is the game's own rule (`NDrone2_DealWithObjHit` 0x13fff0
   case 5 has no team gate for the player) and now carries its mission-fail consequence.

## How it was verified

`crates/nf-view/tests/fix_actors_live.rs` (4 tests, each driven through `nf_view::load` + `Sim::step_bond`
with real fire/grenade/walk inputs, each with a sister assertion) + the whole existing suite. Full suite:
80 test binaries green; only the 3 allowed `disc_golden` slim-ISO failures.

## Shared-file edits (all additive, listed)

- `drone.rs`: the `hit_by_player`/`fail_reason`/`armed_peer_dist`/`wait_channel*`/`route_blocked` fields, the
  death hook + the fail drain, the armed-peer snapshot, the new state arms, the AILOG trace in the Lead arm.
- `triggers.rs`: `Toucher::player`'s volume (one function, cited).
- `tests/fix_mission_live.rs`: the escort test resolves the fight (the hide rule is the game's own).
- `spawners.rs`/`ally.rs`: the task's own files.

## What's left (documented in docs/systems/actors.md §7)

The straight-line stand-ins for `DroneMove_ObjectMissionPathDistance` 0x153f50 in the Lead waits;
LeadMissionWait has no route-node channel source; the AllyFollow band is straight-line; the LeadBondCombat
cover-run (FindRunToPoint) is not ported. Not ported (no disc pressure on the completion paths): the Hostage
chain 0x08–0x0F beyond the idle, UnderCover 0x93–0x9C, Abseil 0x8E–0x92, Astronaut 0xBD–0xC1, the Ninja melee
chain 0xA6–0xB3, KikoMission 0x19/0x1A, ElevatorJumper 0x8D, FailMission 0xB5, DeleteMe 0xB4; the spawner's
mode-3 wave-delay byte and the power-station spawn SFX branch (G-025's remainder).
