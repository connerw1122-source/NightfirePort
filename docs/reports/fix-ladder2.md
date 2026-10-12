# fix-ladder2 — the plain-language report

Branch: `s/fix-ladder2` (from master 923595b). Everything below went through the "reproduce first,
then fix from the decompile" path, with the evidence tags in `docs/WORKLOG.md`.

## Done

1. **The ladders are drawn.** Root cause: a ladder placement (type 0x22) is a *dynamic* object in the
   game and never carries the 0x8000 "create a cel" flag, so the port's default scene build (which
   instances only 0x8000 placements) dropped every ladder. The game draws each ladder exactly once
   through its generic dynamic-object path, as a static mesh at the placement transform, with the
   placement's own object as the mesh and no ambient dim. The port now does exactly that (a dedicated
   sweep, counted in `Scene::ladders_drawn`). Verified: 07000002 draws exactly 4 ladders (the disc's
   placement count), 07000005 exactly 1 — the exact counts also prove there is no double draw.

2. **The camera moves while on a ladder.** `Player_Climb`'s look handling is ported: the turn input
   moves the yaw inside the eased ±90° window towards the ladder (hard snap beyond ±90°, then 20% per
   tick back towards the ladder facing — the ease pauses while aiming), and the pitch is free, clamped
   to ±90° like on foot. Verified headless through the real input path: turning and looking during a
   live climb both move the camera; a unit test pins the window and the clamp.

3. **The weapon comes back after a ladder, and empty hands can't fire.** The game saves the current
   weapon on the grab (`Player_WeaponNone`), switches to hands immediately (no holster animation), and
   restores it on the dismount through the anim-carry + re-select chain. The port now does all three.
   The "fires once" bug: the hands record has no fire scripts at all, and the game spawns the round
   from the fire-script path — the port now blocks the trigger for scriptless weapons. Fists keep
   theirs, so punching still works. Verified: grab → hands with the P2K saved, firing at hands spawns
   no bullet, dismount → the P2K is re-drawn.

4. **Crouch-walking no longer sounds like rapid footsteps on metal.** The 0.4125 stride + fixed
   0x286 sound belong to the *ladder climb* (the rung sound), not crouch — the port had attached them
   to crouching. Crouch steps now use the normal 1.65 stride with the floor's surface sound, as the
   game does.

## Verified and how

- New tests `crates/nf-view/tests/fix_ladder2_live.rs` (run with `NF_ISO`, through the real viewer
  load + input path) and a `player.rs` unit test; the full four-crate suite was run before pushing
  (only the 3 allowed `disc_golden` failures on the slim ISO, if any).
- Every fix cites its decompile function and address in the WORKLOG entry and in code comments.

## Left (researched, not ported)

- **Swimming from a bank**: repro recorded (an `#[ignore]`d test asserts today's broken behaviour).
  The garden water's collision floor sits 3 mm *below* the water surface, so Bond stands on the water.
  The port's player collision is double-sided while the game's triangles are one-sided by their stored
  plane — the likely cause of "walks on the surface, can't get in". Fixing it needs the winding
  convention of the collision data verified first; I ran out of time before that.
- **The vent / grate**: researched, not ported. There is no grate class — the crawlable vent is a
  *creepwall* (placement 0x3F, class 0x2B: use → crawl substate 7 with its music event and channel
  writes), and the grate panel is an ordinary door. Porting it is a full session of its own; the full
  function chain is in the WORKLOG.
- **Level-select weapon fallback**: the game's rules are documented in the WORKLOG (`Player_EquipWeapon`
  refuses weapons without a first-person skin, `Player_CheckWeaponsLoaded` zeroes the group, fists
  fallback) — not ported, the dev-tool give path is outside this task's files.
- **Searchlight alarm music on 07000006**: the whole chain is identified (spotted → channel →
  `DroneFunc_CheckAlarmRaised` → `Music_Event(2,1)` → the per-level music updater) and the port has
  all but the last step (a small change in `music.rs`, a shared file — left for its owner session).
- **C2-style drones (drawn, never attack, un-hittable), the 07000005 patrol torso twist, the grenade
  impact/explosion and flashbang-stun work**: not started (time).
- **07000007 guests not walking**: already fixed on master by fix-drone4 (CivilianMission + the route
  chain) — verified nothing to do.
