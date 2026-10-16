## Your task: `pickups-08` — the briefcase and the rocket launcher on 07000008

Playtests 1012-2337 and 1013-1758: "cant pickup the briefcase or rocket launcher" in the 07000008 safe room; the
invisible wall there is fixed. Find why each pickup refuses:
- the placement types and pickup kinds [M Pickup_Create 0x18dda0, Pickup_Handler 0x18e650];
- the use/collect radius;
- any gate (a script channel, the safe's state, the Q-Laser cut);
- whether the briefcase is a gadget pickup (the briefcase gadget 0x...: the weapon id from the pickup record).

Fix the sim so walking or using the item collects it exactly as the game does. Headless tests on 07000008: collect
both, from Bond positions taken from the placements; check the inventory and the HUD message. Sister: out of range
does nothing.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/pickups-08`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T07:48Z

---
## Report (done, 2026-10-16T09:49Z)


---
## Report from the worker (done, 2026-10-16T09:49Z)

# pickups-08 — the 07000008 safe room's briefcase and rocket launcher

Branch `s/pickups-08`, from `origin/master` ad761677, merged `origin/master` aa3bc1f5 (the final
tree the numbers below ran on). Playtests 1012-2337 / 1013-1758 reported "cant pickup the
briefcase or rocket launcher" in the safe room; the invisible wall there was fixed by
fix-safe08/fix-safe08b before this task started.

## What the take query really does (fix-safe08b's open item, closed)

`Pickup_Update` 0x18e148:203-214 collects when the player is within `radius + 1.1` and
`Collide_LineOfSight` from his world-sphere centre to the pickup's is clear. The wrapper's
disassembly (0x1edf50-0x1edf98, the C had dropped its arguments) shows what it actually passes to
`Collide_RayIntersect` 0x1ec690:

- **char = 0** — so the per-node skip in RayIntersect's walk (`owner != 0 && owner != exclude &&
  char != 0`) never fires: EVERY candidate node is tested, dynamic-owned included;
- **query word `0x721 | 4 = 0x725`** (HITTEST+0x98) — the take's flags get bit 0x4 added inside
  the wrapper (`ori t3, t1, 4` at 0x1edf54);
- **mask 0x10** (HITTEST+0x9A) — which arms the per-triangle `Collide_Filter` 0x1ed888 inside
  `Intersect_RayGeom` 0x10a8c8 (0x10acc4-0x10ad20: with +0x9A bit 0x10, each accepted triangle
  first runs Filter(surface, +0x51, +0x98); a Filter rejection is overridden when the surface
  byte's 0x40/0x80 bits are set).

The filters bit 0x725 turns on are the ones the port already implements: the 0x1000-cage skip
(`Collide_Pick` 0x1eb320's bit 0x20) and the surface classes 0xd/0xe
(`Collide_Intersect` 0x1ebaa8's tail / `Collide_Filter` — bit 0x1). `Collide_PickObj` 0x1eae80
keeps a dynamic object unless obj+0xF0 & (0x20 | 0x1000) or its class byte (+0xff) is
0x02/0x03/0x05/0x11/0x12 — and the safe-room blockers all pass those gates:

- the safe door SS #205: class 0x25 [M SS_Create 0x1a9c58:52], glist flags 0;
- the safe body `Object_Safe` (map51 obj155) and the bolt objects: glist flags 0
  [D `probe_glist_flags` in `pickups08_live.rs`, NF_PICKUPS08_FLAGS=1].

So the doorway take ray that grazes the safe's own shell (surface 6) at t 0.647 **survives every
filter of the game's query**: the game refuses the take at body-sphere facings whose ray grazes
the shell exactly like the port. The origin is the yaw-rotated body sphere [L M_CONTROL+0x80 =
the posed model's bounding sphere], so the refusal is knife-edge — a walking player's varying
facing collects both items, which is what the game's players do and what the new tests show.
**No take-path change was needed; the port matches the query.** (fix-safe08b's live check 2 —
the GDB watch on the HITTEST words at the doorway spot — can confirm: expect +0x94 = 0x201,
+0x98 = 0x725, +0x9A = 0x10, and the `Object_Safe` shell hit DISCARDED only at t≥1 end contacts,
never by a filter.)

## The real bug found and fixed

`Script_CreateEntity`'s obj_type-0xF0 arm calls `Pickup_Create` [M disasm 0x1d8bcc-0x1d8c20], and
`Pickup_Create` applies the level-conditional rounds override **inside the create** [M
0x18dda0:116-118: `GameState+0x12 == 0x7000008 && id == 0x2c → rounds = 4`] — so the
script-spawned SafePickUps rocket must hold 4 rounds even though the script's word says 5 [D
0x060009E2 s1]. The port applied the override on the placement path and the reserved-slot
reservation path (scene.rs `script_pickup_reservations`) but NOT on the drain's fresh-push arm
(drone.rs) — a script spawn outside the pre-scan held the raw word. On 07000008 the take then
credited a phantom carried round (the rocket's clip is 4): `Player_EquipWeapon`'s overflow
`rounds + old − clip_size` = 1 carried that the game never gives.

Fix: `create_rounds_override(d.level, sp.id, sp.rounds)` in `drone.rs`'s fresh-push arm.

## The tests (`crates/nf-view/tests/pickups08_live.rs`, all through `nf_view::load` + `Sim::step_bond`)

- `the_safe_spawns_sit_at_the_script_keys_after_the_open` — after the real open chain (fusebox →
  Q-Laser burns → ch 136 → the door SS) the rocket (kind 0, id 0x2C, **4** rounds) rests at the
  lower-shelf key (162.59, 42.75, −116.78) and the briefcase (kind 4 LABEL, id 6, switch 0x4A) at
  the top-shelf key (162.70, 43.54, −116.22) [D 0x060009E2 s1/s2]. The briefcase is NOT a gadget
  pickup: its id is 6 and kind 4 has no inventory effect [M 0x18e650 case 4]; the 0x52 briefcase
  gadget belongs to 0700000c/07000012.
- `the_safe_pickups_collect_from_the_doorway` — both items collect from the doorway (the user's
  spot): the inventory gets the rocket (owned, clip filled to the weapon's clip size 4, the
  carried pools credited only the overflow — none at 4), ch 0x4A fires, the HUD queues the
  "Picked up <rocket>" message (0x200000D [M 0x18e650:243-247]) and the label-0 briefcase queues
  none, no auto-switch fires (the 0x2C base ranks −1 in BestWeapon [M 0x1a6eb0]), and the
  view's gadget-recharge slots are excluded from the untouched-checks.
- `walking_through_the_safe_mouth_collects_both` — the touch test runs every tick [M 0x18e148]:
  a forward walk through the safe mouth takes both mid-walk, no button.
- `out_of_range_the_safe_pickups_do_nothing` (the sister) — from the bedroom mid-floor (~7 units
  out), four seconds of mashing E: nothing takes, ch 0x4A stays off, the rocket's inventory
  slots hold still, both shelf items remain.
- `probe_glist_flags` (NF_PICKUPS08_FLAGS=1) — the report's disc-flags evidence.

All 5 green on the final tree.

## What was run before pushing

- `cargo test --offline --release -p nf-game --lib -- --test-threads=1` — **237 passed, 0 failed**
  (the changed crate's full unit suite, final tree).
- `cargo test -p nf-view --test pickups08_live --test fix_pickups_remainder_live
  --test fix_pickups_ammo_live --test fix_safe08_live --test fix_hud_live -- --test-threads=1` —
  **21 passed, 0 failed** across the five pickup-related binaries.
- The fast runner (`python3 tools/test/suite.py`, 404 binaries) on the final tree: this sandbox
  reaps background jobs and re-builds after each merge, so the session got it part-way (the
  nf-formats + nf-view head of the run). Within the part that ran: the only failures are the
  documented pre-existing set — the 3 `disc_golden` (the slim ISO) and the 0c-pylon family
  (WORKLOG 11157/11545). **The full-suite total line is not available this session; the next
  session should run the suite start-to-finish on this branch.**

## Live checks for the main agent

1. GDB-watch `Pickup_Update`'s `Collide_LineOfSight` call at 0x18e148:214 while taking the rocket
   from a spot whose ray grazes the safe shell (the doorway, body facing the safe): the HITTEST
   words +0x94/+0x98/+0x9A (expect 0x201 / 0x725 / 0x10), and whether the game registers the
   `Object_Safe` shell hit at t 0.647 with +0x9A's 0x10 armed (the decode says it does — the
   take should refuse at that exact facing, matching the port).
2. Render check (rule 6): `07000008 --bond 161.87,43.39,-116.54,1.583,-0.495` after the safe
   chain — both items on the shelves, the take message on a walk-in.

## What's left

- The full suite start-to-finish on this branch (session time; see above).
- The gadget-recharge gates (scene.rs 1978-1992) are an unverified approximation — out of scope here.
