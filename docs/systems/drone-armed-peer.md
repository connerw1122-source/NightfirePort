# The drone armed-peer rules (NearArmedDrone / NotNearArmedDrone)

The "armed peer" scan behind the escort's cower (AllyLead's LeadHide family) and the hostage
states. The port's snapshot computes `Drone::armed_peer_dist` (drone.rs, the shared pass before
the per-drone walk) from it; every consumer branches on ≤ 10.0 (hide enter), > 14.0 (hide exit)
or ≤ 20.0 (the hostage radii), so the port caps the scan at 20.0 — a peer beyond 20 can never
flip any of those comparisons.

## The game's predicate

`NDrone2_NearArmedDrone(DCVars*, float radius, char flag)` 0x177420 walks the global obj list
and accepts a candidate only when ALL of these hold:

1. **The candidate's actor is a live started drone** [M 0x177420:74-89]: actor+0xe0 → drone data;
   its +0x4F8 flags need `(flags & 0x600) == 0` (not down/stunned), `(flags & 0x100) != 0`
   (started), health (+0xAC) > 0.0; obj+0xFF != 0x11 and obj+0xFE bit 0 clear (not deleted);
   obj+0x4F8 & 0x4200 == 0. The port's `team == 1 && started && !corpse && enabled && health >
   0.0` covers the shipped levels' cases (the 0x600 stun bits the port does not model are the
   known gap, tagged in the report).
2. **The weapon filter** [M 0x177420:89-104, disasm 0x177538-0x177578]: the candidate's weapon
   byte (obj+0xDC → +0x62) must be in **[2..0x46] ∪ [0x6a..0x71]** — the same predicate as
   `NDrone2_ObjectIsArmed` 0x178768 (`Drone::obj_armed`). 0 and 1 (unarmed/holstered-fists),
   0x47 and ≥ 0x72 are not peers. The decompiler C reads signed; the disassembly is `sltiu`
   on `(b − 0x6a) <u 8` ([0x6a..0x71] armed) + `slti (s8)b < 2` ([0..1] out) + the 0x47 branch
   at 0x177564 (out) — the C's `7 < bVar1 - 0x6a` is unsigned.
3. **The class byte** [M 0x177420 disasm 0x177580]: obj+0x44 == 1 (the team-1 guard class).
4. **The radius**: `Vec_Dist3D(drone_obj+0x30, candidate_obj+0x30) <= radius` [M 0x177420:111].
5. **The sight gate** [M 0x177420:113-128]: `NDrone2_GetHeadPos` on the seeker, then
   `DroneVision_CanSeeObjectFrom(drone, candidate, 5, head_pos, drone_cel, 0)` —
   0x176f18 builds the candidate's bone-5 (head) position and runs a head-to-head
   `Collide_LineOfSight(0xa27)` behind an `AINetwork_TestRayCels` AI-cel pretest (the two cels'
   AI ray must be clear first; equal cels skip the pretest) [M 0x176f18:56-105].
   The port's stand-in is the shared head-to-head `world.line_of_sight` (the shape the shout
   pass already uses) — the AI-cel pretest is subsumed by the world segment test.
6. **The flag-1 prop gate** [M 0x177420:57-61]: with `flag != 0`, the SEEKER's own behaviour
   property 0x50 must be set (`behaviour_util_getProperty` 0x1c7d98 on Drone+0x4D8) — the
   callers pass flag=1 at the four AllyLead enter sites [M 0x15e698:30, 0x15eda8:30, 0x15ea28:30,
   0x15f120:26] and flag=0 at the hostage sites [M 0x15ce60:29, 0x15cfe0:24]. The port gates the
   four Lead hide-enters on `self.prop(d, 0x50)`; the hostage sites and the hide exit carry no
   prop gate.

`NDrone2_NotNearArmedDrone(DCVars*, float radius)` 0x177640 (the hide-exit read, radius 14.0
[M 0x15ec08:30]) runs the same 1-5 filters with NO prop gate, plus one hysteresis wrinkle: the
peer latched at Drone+0x2B8 by the last NearArmedDrone hit (0x177420:126 writes the found peer
there) blocks the exit by DISTANCE ALONE while within the radius, seen or not [M 0x177640:98-100].
The port does not model the +0x2B8 latch (each of its snapshots recomputes the min over visible
peers); the walkreal 01 walk does not hit the difference — the balcony guard who parks 13.7 from
the contact is never VISIBLE from the ground, so the sight gate alone clears the exit.

## Why the sight gate matters on 07000001

The meet scene wakes four ch-0xE hall guards; one (skin 0x05000036) ends the walk parked on the
balcony above the west stairwell — head-to-head LOS from the escort's ground position is blocked
(the ray hits the balcony slab). Without the sight gate the port counted him forever: the escort
hiding at the stairwell head could not reach the > 14.0 exit (the guard sits at 3D ≈ 3.9-13.7
through the descent) and never finished his mission route to the LoadPointActivator plate. With
the gate he stops being a peer the moment the line blocks, exactly as the game reads it.

## The port's consumers

- `ALLY_LEAD` / `ALLY_LEAD_WAIT` / `PLAYER_IN_WAY` / `LEAD_BOND_COMBAT` hide-enter at ≤ 10.0,
  gated on prop 0x50 [M the flag-1 call sites above] (drone.rs, the AllyLead family).
- `LEAD_HIDE` exits at > 14.0 with the 2-tick timer C latch [M 0x15ec08 case 3].
- The hostage save states (20.0, flag 0) ride the same snapshot.

## Live checks for the main agent

- [L wanted] 07000001, after the meet scene: watch Drone+0x2B8 (the armed-peer latch) of the
  skin-0x4F contact during the stairwell approach — write watch, expect 0 until a hide-enter,
  then the triggering guard's obj pointer.
- [L wanted] The balcony guard (skin 0x05000036) while the contact is at the stairwell head:
  confirm the game's contact does NOT hide (the sight gate blocks) — poke-bond to the stairwell
  head and screenshot the contact's state over ~5 s.
