# fix-objects — report

Branch `s/fix-objects` (from master 0a2fcc4; master merged in before push).

## What was done

**The top priority — pickup models drawn from the wrong source.** Found and fixed. There were two
real bugs behind the screenshots, plus a third one the death-drop report pointed at:

1. **Draw slots went stale after the first pickup was taken.** The renderer draws placed pickup slot
   `k` with the mesh it chose at level load, and poses it every tick at the position of pickup `k`
   in the simulation's list. The simulation *removed* taken pickups from that list
   (`retain(!gone)`), so after your first pickup, every later mesh was drawn one slot off — at other
   pickups' positions and at guard drops' positions. That is the gold "007" mesh at the armour spot
   and the wrong model on the floor at a drop. The game never does this: it draws each pickup
   object's own model at its own matrix (`View_DrawObjectGlist` 0x1e6780). Fixed: taken pickups stay
   in place as tombstones; everything that counts or draws filters them.
2. **Some placed pickups were never drawn at all.** The game draws the model pointer it was handed
   at creation; the port re-looked the mesh up by hashcode. The 9mm ammo piles on 07000005 resolve
   to objects whose hashcode is 0xFFFFFFFF — no hash entry — so those pickups existed, were
   takeable, and were invisible. Fixed: the pickup keeps the resolved object and the renderer draws
   exactly that object (this is also the mechanism behind the playtest's O2 "armour model isn't
   drawn").
3. **Extra visible objects at corpses** were the same slot shift (placed meshes hovering at drop
   positions), not extra drops — the drop itself was already correct (one visible gun per death;
   fix-player3's pin re-verified through the real input path).

**The two "swapped" piles on 07000001.** Verified against the disc (all six type-0xF0 placements:
kinds, ids, rounds, meshes) — the port's param reading is correct now, and both ammo piles and both
armour pieces carry their disc kind, id, rounds and their own mesh. Tests pin all four piles, so a
future param mistake fails immediately.

**The 007 bonus doing nothing on pickup.** The game logs the bonus (`PlrStat_LogBondBonus`) and
plays the take sound 0x14C (`Pickup_Handler` overwrites the pickup's sound per kind before playing;
0x18e650:234-236, 319-321). The port consumed the bonus silently. Fixed: the take is logged and
plays the sound; the notice shows in the window title (the game's HUD text path doesn't exist in the
port yet, so the full on-screen message is not done).

## What's verified and how

- Headless tests through the real path (`nf_view::load` + `step_bond`, the same input path the
  viewer uses) in `crates/nf-view/tests/fix_objects_live.rs`. Each failed on master, all pass now:
  - slot identity after a take (master: index out of bounds — the list had shifted);
  - kind↔drawn-model for every placed pickup on 07000001 and 07000005 (master: "kind-1 pickup draws
    `<unresolved 0xffffffff>`");
  - the four 07000001 piles against their disc kinds/ids/rounds/meshes;
  - a guard death driven through firing: dump of every pickup created (kind, id, rounds, hidden,
    gone, from, position) — exactly one visible drop, placed slots unmoved;
  - the afc60ed regression guard you asked for: on 07000001 and 07000005, the dynamic-vertex tail
    pairs 1:1 with `char_verts`, and every placed pickup, door and searchlight slot has its vertices
    (this is the test that would have caught the dropped CharVertex push);
  - the bonus take reports `Bonus` and consumes the pickup.
- All previously passing tests in the touched area re-run green: fix_player2, fix_player3, scenario,
  pickup_live, rng_live, weapon_switch_live.
- Independent audit (a separate agent with only the decompile): 6/6 claims verified against the
  decompiled C (the case-0xF0 argument map at 0x1d04f0:457-464; the glist-pointer draw path
  0x18dda0:134 → 0x1e6780:68-82; `hashtable_hashcode_to_celglist(0xFFFFFFFF)` returning 0;
  kind 3/6 creation behaviour; the take conditions; the handler's per-kind sound and consumption),
  and no remaining consumer of the pickup list that a tombstone could break.

## What's left (2-hour sandbox; nothing was changed for these without a repro first)

- O1 — 07000006 placed pickups missing below the start (needs the spawn-filter investigation).
- O3 — shootable glass/breakables; the 07000002 kitchen pane test case.
- O4 — the wire-cut box interaction.
- F3 — the tuxedo skin-swap in the dressing cut-scene (`nis.rs`).
- The 07000002 next-area door not drawn.
- The gondola level's invisible wall, the rocket-launcher pickup gate, the red-headband guard skin.
- The game's HUD message text (0x200004A et al.) — the port has no HUD text path yet.
- The full NF_ISO suite was not run end-to-end in this sandbox; the touched crates' pickup, player,
  scenario, RNG and weapon tests were (all green).

## Files touched

`crates/nf-game/src/pickup.rs` (obj field, Taken::Bonus, take sound), `crates/nf-game/src/drone.rs`
(tombstones, per-kind take sound — shared file, changes confined to `update_pickups`),
`crates/nf-view/src/scene.rs` (pickup object wiring in the two build paths + draw loop, drop-find
filters — shared file, changes confined to the pickup/attachment code), `crates/nf-view/src/lib.rs`
(one match arm for the new Taken variant), new `crates/nf-view/tests/fix_objects_live.rs`,
`docs/atlas/pickup.md` (port status), `docs/WORKLOG.md`.
