# fix-objects2 — report

Branch `s/fix-objects2` (from master 9aac84e). Task: 07000008 (the gondola level) — the invisible wall after the
opening cut-scene, and the safe + gondola objects (the laser-watch chain, the ride as a script-driven moving
platform).

## The invisible wall: found, reproduced, fixed

**What it is.** Not a door or a cut-scene removal — the `BlockingPoly_PlayerMissile` family: collision-only
objects (glist flags 0x1000, skipped by the draw pass) whose triangles form a **player-missile cage** over the
map. On 07000008: a lid at y 52.8 across the whole sky, a floor at y −40, a wall plane at x 135.3 (z −100.1..−116)
and two walls over the station walkway (z −100.1 × x 135.3–144 × y 46.2–52.8, and x 144) [D — all triangles
dumped in the probe]. The walkway from the shutters down to the gondola crosses the x 135.3 plane — that is the
wall the user hit, several times.

**Why the game doesn't block the player.** `Collide_Pick` (0x1eb320, read in the disassembly at 0x1eb468-0x1eb49c)
walks the collision tree per query and skips any node whose cel flags word (cel+0x94 = the glist flags) has bit
0x1000 **when the query's HITTEST+0x98 carries bit 0x20** — the actor-movement queries. Ray/sight callers (the
searchlight's flags 0x18, for example) don't set it, so the cage stops rays. The level is playable across the
wall in the game, which pins the actor query to the skipping side. The port's static world took every triangle
unconditionally.

**The fix** (`crates/nf-game/src/world.rs`): `build` splits 0x1000-flagged instances into a separate `missile`
geometry set — solid to `segment`/`segment_surface` (bullets, sight, the floor rays), invisible to the actor
capsule (`capsule_push` via `near_actor`). The dynamic set is untouched (no door on this level carries 0x1000).

**Reproduce-first evidence**: `crates/nf-view/tests/fix_objects2_live.rs` — `the_missile_cage_still_stops_bullets_
but_never_the_capsule` asserts the cage lid stops an upward bullet at y 52.8, the x 135.3 plane stops a
crossing segment, and the capsule at the same spot feels nothing. **Fail-first shown**: with the world.rs change
stashed, that capsule assertion panics (the wall pushed the capsule +x). The walk test
(`bond_walks_down_the_station_stairs_past_the_former_invisible_wall`) drives the real input path — spawn → the
intro plate (the 3881-tick cut-scene plays) → the shutters (E-opened) → the walkway → down the stairs past the
cage plane — green post-fix; the route and every blocked position were first mapped with `probe_objects2.rs`
(kept in the repo).

## The safe and the gondola: the whole chain, green end-to-end

The mission record names the objectives ("Retrieve guidance chip from Safe Room" — ch 0x4A; "Escape with Zoe in
gondola" — ch 0x4B [L record 3]), and every writer turned out to be on this level. The chain, as ported:

1. **The safe bolts** are locks (kind 0x2E → `Lock_Create` 0x1ae1f8; out channels = param 5 → **134/135** [D
   #207/#208]). New `crates/nf-game/src/lock.rs` carries the object and the burn hook: `Lock_Update` (0x1ae3d8)
   case 0 reads the activating weapon (0x4E/0x4F the decryptor, the 0x56/0x57 laser band :162) and writes
   `switch_channels[record+8]` on burn-through (:194). **The laser watch itself is not ported** (the gadget port
   map: all of 0x4A..0x5E are out of selection) — the hook is the documented seam: the future
   `Player_LaserPointer` (0x1a65d8) calls `Lock::burn` per tick while its ray hits the lock's world sphere.
2. Burn both bolts → mux #278 (out **136** = 134|135) → the safe door #205 (a SimpleScript on ch 136 — the port's
   `ss.rs` animates it) and SP #286 SafePickUps. The fusebox #219 (kind 0x31, script 0x060008AD) → ch 137 opens
   the GuardRoomDoor bolt (#217) and unlocks Door_Guardroom (#122, lock ch 137) — the port's existing fusebox
   E-cut stand-in drives it (the game's tool is the laser watch on the SmallExplosionBox #234).
3. **The briefcase** #290 (`oBriefcase_closed`, pickup type 4, switch ch **0x4A**): the port's inventory refused
   type 4 (`_ => None`). The game's `Pickup_Handler` case 4 (0x18e650) is the label/item pickup — always consumed
   (label 0 here [D], so no message). Fixed: `kind::LABEL` + `Taken::Label` (both HUD matches updated, including
   fix_hud_live's). Taking it completes objective 1 [M 0x18e650:330-333].
4. Bond on the cable-car floor (Touch 0xEB #236 `SWITCH_CABLECARFLOOR`, out 4, mask 8 = the player) + ch 0x4A →
   mux #235 (out 1 = 0x4A AND 4) → **ch 1**.
5. ch 1 starts SP #295 `ScriptPlayer_CableCar` (mask 1 on ch 1, mode 64 = loaded-armed [M SP_LoadScript 0x1dbc08
   disassembly]) — the ride script **0x0600004C lives in the level bin** (the level doc's "loadable pack" guess
   was wrong), 53 streams / 1420 ticks: `OBJECT_Carriage` (0x0200038a) descends (112.0, 32.4, −126.3) →
   (112.0, 18.0, −179.4) with its doors, glass and the in-car pickups parented; stream 46 writes **ch 0x53 (83)
   at tick 1410**; stream 49 keeps the objects at the ride's end.
6. ch 83 starts SP #296 — 0x06000078, the bottom-of-mountain cinematic (pack 07900008.bin, 342 ticks) — whose
   stream 10 writes **ch 5**.
7. **The helicopter** #45 (`Helicopter-Body`, kind 0x2F → `Copter_Create` 0x17d340): new
   `crates/nf-game/src/copter.rs`. The channel machine is exact [M: the gate record+0x70 = param 6 (0x17d340
   disasm 0x17d49c); the arrival write record+0x72 = param 7 when the move completes (0x17dbb8 cases 4/5/6,
   :249-268); the +0x74/+0x76 countdown pulse :16-22]. Gate = ch 5, out = **ch 0x4B**; the flight (0x06000050,
   299 ticks — its parts assembly 0x06000051) rides the port's script machinery via `Scripts::spawn_once` (the
   game's hidden `SP_Create` inside `Copter_Create`, 0x1dc7e0 = SP_Create). The combat states (missiles, guns,
   Music_Event(2,1)) are not ported — the extraction copter never arms here.
8. ch 0x4B completes objective 2 → mux #276 (out 100 = 75) → **ch 100** → MissionState 6 → the level ends into
   0x0730008 [M Mission_Update 0x182c60:85-116]. `the_gondola_ride_runs_and_the_helicopter_completes_the_mission`
   drives the whole chain headless and asserts every hop.

**The trigger containment fix** (`triggers.rs::touched_by` — a shared file, named in the task as a trigger hook):
the AABB containment contact fired the plunge death NIS for Bond standing on the boarding platform INSIDE plunge
#54's AABB — the volume's mesh is two valley-sized quads the platform never touches [D tris]. That is
fix-level2's recorded walk-in graze (playtest F5). The containment now applies only to volumes whose triangles
are all < 24 units on an edge (the room-exit shells and walk-over plates — 07000006's LoadPoint #746, edges
≤ 10.4); the plunge family's quads (edges 25.8–46.5) fall back to the mesh test. [S — the game's contact is the
mesh; the tri-size gate is the port's approximation.] 07000006's walkthrough re-verified green.

## Shared files touched (minimal, per the task's note)

- `world.rs` — the missile-cage split (the core fix).
- `triggers.rs` — the containment tri-size gate.
- `pickup.rs` — `kind::LABEL` + `Taken::Label`.
- `nis.rs` — `Scripts::spawn_once` (the copter's one-shot player).
- `drone.rs` — `copters`/`locks` fields, their create in `attach_level`, the copter tick before `scripts.tick`.
- `nf-view`: `lib.rs`/`scene.rs` (the two `Taken` matches — one arm each), `tests/fix_hud_live.rs` (the same
  match, one arm).

## What's left (each with its evidence trail)

- **The gondola as a ridable platform**: collision works (the carriage joins `set_dynamic` via the normal
  script-entity path — Bond in its path is pushed/swept, observed), but the **riding inherit does not engage**:
  the probe shows Bond grounded at his teleport height while the floor below moves away (`grounded=1`,
  `floor_below` = the static platform 10.8 below, the grate's shape present and moving in `set_dynamic`). The
  truck ride's inherit ran on ANIM discrete parts; the carriage's parts are script ENTITY parts (one
  `(id, hash, matrix)` per entity). Needs a session comparing the two paths. The level is completable without
  it — the ride is a scripted cut-scene (the game disables input; Bond boards and the script takes over).
- **The laser watch gadget** (0x1a65d8 `Player_LaserPointer`, `Draw_LaserBeam` 0x1a4560) — the object side is
  ready (`lock.rs`'s burn hook, the fusebox, the SmallExplosionBox 0xE1 `Destroy_Create` — not ported); the
  gadget itself is a gadgets task.
- **The copter's combat states** (0x17dbb8 case 2: `Music_Event(2,1)`, `Copter_FireMissile`/`FireGun`, the
  cone-sphere detection) and its exact path-following (`Copter_Move` 0x17d628's framelist advance) — the port
  times the flight by the script length [S], which lands ch 0x4B at the right beat but does not model the
  per-path state machine.
- **The lock/destruction meshes** are not drawn (the 0x2E/0xE1 placements are not in the port's dynamic draw
  list) and the locks' framelist open-animation is not posed — render-side only; the channel logic is exact.
- **The kind-4 pickup's label text** (PICKUPINFO+0x34 → `Txt_BindLabel`) — the port's pickups don't carry the
  label word; the only kind-4 pickup on the disc data so far labels 0 (no message in the game either).

## Verification

- `fix_objects2_live.rs` (5 tests, all green, NF_ISO): the walk past the wall; the cage's bullet/capsule sisters
  (fail-first shown for the capsule); the safe chain; the ride chain end-to-end; the no-briefcase sister.
- Full suite (`NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio
  -p nf-view -- --test-threads=1`): 86 test binaries green; the only failures are the 3 allowed slim-ISO
  `disc_golden` tests.
- The docs updated with the commit: `docs/levels/07000008.md` (the full chain, the missile cage, the plunge
  shells), `docs/WORKLOG.md` (the fix-objects2 entry).
