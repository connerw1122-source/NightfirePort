# fix-safe08 — the 07000008 bedroom: the blue block and the safe's pickups

Branch `s/fix-safe08`. Time-boxed session (sandbox expiry); the pickups' root cause is found,
ported and verified to the point the take fires in the harness except for one last LOS graze; the
block's root cause is found and localised, its fix is specified but NOT implemented.

## The safe's pickups — root cause found and ported

The user takes nothing because the port never created the takeable items. On the disc, the items
are spawned by the **SafePickUps script 0x060009E2** (SP #286, mask 1 on ch 136 — it starts when the
safe opens):

- stream s1: `Entity { hash 0x02000F35, obj_type 0xF0, params [0, 44, 5, 0, 319, 0, 0] }` — the
  rocket launcher (kind 0, id 0x2C, 5 rounds), stream keys at **(162.59, 42.75, −116.78)**;
- stream s2: `Entity { hash 0x02000F34, obj_type 0xF0, params [4, 6, 0, 74, 245, 0, 0] }` — the
  briefcase (kind 4 LABEL, switch 74 = 0x4A), keys at **(162.70, 43.54, −116.22)** [D the level-bin
  script, dumped].

`Script_CreateEntity` 0x1d8968's obj_type-0xF0 arm calls `Pickup_Create` — kind/id/rounds/switch
from the entity params [M disasm 0x1d8bcc-0x1d8c20: t0=lhu 0x349974, t1=lhu 0x349978, t2=lw 0x34997c,
t3=lw 0x349980], sound = params[4] (0x349984), **rest = 1** (8(sp)=1 — the pickup rests exactly at
the stream's posed key position), lifetime 0, label 0, glist = `hashtable_hashcode_to_celglist(hash)`.
The disc's parked 0xF0 placements #290/#291 sit at shelf +10 x (172.7) — INSIDE the east wall [D
placement dump; the shelf keys are exactly the parked positions −10 x]. The port created only the
parked pickups (10-11 units from the safe, unreachable) and never ran the script arm.

**Ported** (all cited in `docs/WORKLOG.md`): `Objects::pending_pickups` + the 0xF0 emit in
`nis.rs::entity_start` (create path only — the game's reuse path re-points gfx without a new pickup
[M 0x1d8a68-0x1d8a80]); the drain in `drone.rs` after `scripts.tick` (a reserved slot goes live; a
no-slot spawn pushes a fresh pickup); `scene.rs::script_pickup_reservations` (deterministic pre-scan
of `level.nis` sorted by hashcode, appending inert `gone` slots to BOTH the render and the sim
placed-pickup lists so the renderer's per-index draw survives) + `Drones.script_pickup_slots`.
After the fix the sim holds the shelf pickups at (162.59, 42.75, −116.78) / (162.72, 43.53, −116.23)
— the harness prints them (`[safe08] pickup 16/17`).

**Verified**: `the_briefcase_is_not_takeable_before_the_safe_opens` (the sister) passes — before
ch 136 nothing takes and ch 0x4A stays off. `the_safe_pickups_take_after_the_safe_opens` still
fails on the last take step (below).

## The last take blocker (open, with the evidence)

The take test (`Pickup_Update` 0x18e148:203-214) needs distance < radius+1.1 **and**
`Collide_LineOfSight` (flags **0x721**, bit 0x20 set) from the player's sphere centre to the
pickup's. Measured in the harness:

- The safe door SS #205 glides 420 ticks (8 keys, rate 60 [D]); while near-closed its collision
  blocks the ray (t 0.39-0.52). The game's player waits those seconds; the test now waits to t 6.9.
- Door fully open: the briefcase ray is clear of every static instance; the rocket ray grazes
  `Object_Safe`'s own shell (surface byte 6) at t 0.647 from the doorway.
- The port's flat `segment` ALSO crossed the **0x1000 missile cage** beside the safe (the cage's
  tris reach the bedroom; the game's bit-0x20 queries skip 0x1000 [M Collide_Pick 0x1eb320 disasm
  0x1eb480-0x1eb49c]) — fixed this session: `world.rs::segment_actor` (static minus the cage, plus
  dynamic) used by `Pickup::touches`; `segment` (bullets/sight) unchanged.
- After that the take STILL does not fire: a hit persists at t 0.65-0.88 on the exact rays. The
  game's LOS query-type filter (HITTEST+0x94/0x8 = 0x201/0x721 in `Collide_Intersect` 0x1ebaa8's
  switch, the per-node `+0x3c` word) is not fully decoded — the hit that survives in the port is
  one the game's query evidently ignores.

## The blue block — root cause found, fix specified (not implemented)

The block is **`Room_Outside_3`** (map51 obj16, the castle's east EXTERIOR room, r 267): its mesh
has **20 triangles centred INSIDE the bedroom interior** (x 155.3..163.0, floor to ceiling at the
east wall beside the safe) and its collision has **32 tris inside, 10 in the safe band** [D per-tri
dump, `probe_safe08.rs`]. The exterior wall's geometry protrudes into `Room_Guardhouse_F2`.

The game never shows or collides them from indoors because BOTH systems are cel-scoped:

- draw: `View_AddCels` draws only the `Vision_Portal_Recurse` (0x1e86e8 → 0x1e82e0) cel set from
  the eye's room cel — the exterior cel is not portal-visible from the bedroom;
- collision: `Collide_Pick` (0x1eb320) tests only the objects of the cels `Collide_StraddleCels`
  (0x1eac98) collected by walking portals the query straddles (cel+0x28 lists, `Intersect_Portal`,
  cap 0x7e cels) — a capsule inside the bedroom straddles no portal to the exterior cel.

The port draws with the frustum box test per cel (the portal walk `portal_visible_cels` exists but
is NF_PORTAL-gated since fix-perf-level's 07000006 incident) and collides with ONE flat static
world (`world.rs::build`) — so the exterior stone draws over the room and its collision blocks the
walk to the safe. **Fix (next session)**: scope the static collision by the room/portal graph
(passed from scene.rs's `parse_cel_graph` into `CollisionWorld::build`; per-query straddle walk for
`capsule_push`/the actor paths first), and evaluate un-gating the draw's portal walk on this level
(headless check: the walk from every room must keep ≥ the room itself; the 07000006 regression case
re-checked). Shared files touched this session (small, additive): `world.rs` (`segment_actor`),
`nis.rs` (`Objects::pending_pickups`), `drone.rs` (`script_pickup_slots` + the drain), `scene.rs`
(`script_pickup_reservations`).

## What's left

1. Decode the game's LOS query-type filter (0x721/0x201) enough to name the skipped node class, and
   finish the take (the shelf pickups exist and are positioned correctly).
2. Implement the cel-scoped collision (+ optionally the draw) for the block.
3. Re-run the full suite (this session ran the new tests + probes only; the suite did not fit the
   sandbox window).

## Live checks for the main agent

1. `switch_channels` +0x4A while taking the briefcase in the real game, standing in the safe
   doorway: confirm the take fires from the doorway spot (161.87, 43.39, −116.54) — and record the
   player object's +0x80/+0x30 at that moment.
2. GDB-watch the game's `Collide_LineOfSight` call at 0x18e148:214 during that take: the HITTEST
   words +0x94/+0x98 (expect 0x201/0x721) and whether the safe shelf node (Object_Safe's celglist)
   is skipped — that settles the query-type filter the port is missing.
3. In the bedroom, break on `Collide_StraddleCels` 0x1eac98 and dump the collected cel count/ids
   for a walk segment — confirms the exterior cel is absent for indoor queries (the port's fix
   target).
4. Renders (rule 6): `07000008 --bond 155.19,43.12,-113.38,-3.837,0.010` — the blue block beside
   the safe (the port-only exterior stone); after the fix the same spot should show the bedroom
   wall. `07000008 --bond 161.87,43.39,-116.54,1.583,-0.495` — the briefcase and rocket on the
   safe's shelves once the take lands.

## Verification this session

- `fix_safe08_live.rs::the_briefcase_is_not_takeable_before_the_safe_opens` — green.
- `fix_safe08_live.rs::the_safe_pickups_take_after_the_safe_opens` — red at the last assert (the
  LOS graze above); red for the right reason: the shelf pickups EXIST and are at the game's key
  positions (the harness log is quoted in the WORKLOG).
- The fail-first order was honoured for the spawn side: the pre-fix probe showed no pickup within
  3.7 units of the user's spot (the parked ones at 10-11); post-fix the shelf pickups exist.
- Full suite: NOT run (time); the touched code paths are the new tests + probes, `cargo build`
  clean for nf-game/nf-view.
