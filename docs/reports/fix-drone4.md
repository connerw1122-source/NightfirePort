# fix-drone4 — the party guests never walk (user retest), plus the C2 evidence pass

Branch `s/fix-drone4` (from master 94ca4c3). Playtest reference: docs/playtests/2026-10-11.md.

## Done

**The 07000007 guests now walk.** Two root causes, both fixed:

1. The port never ran **CivilianMission (0x16)** — the state the game actually puts the party guests in.
   Every guest on 07000007 carries placement property 0x1D, and the level's six kind-8 (mission) AI paths
   sit exactly under guest placements (one guest 0.017 units off her path node; two more on path nodes 0).
   The game's CivilianInit (0x172730) and PartyGirlInit (0x1728c0) send a prop-0x1D drone with a mask-8
   route into CivilianMission, whose ENTER starts the walk animation (danim 0x37) and whose UPDATE follows
   the route. The port skipped the mask-8 branch entirely and had no 0x16 handler, so every guest stood in
   PartyGirl/Civilian doing stand-idle — the fix-drone3 reading ("CivilianPatrol walks them") had nothing to
   walk: this level has NO kind-4 patrol paths at all.
2. `Route::for_path` collapsed every open path chain. On a node with fewer than two links it re-appended the
   same node, so a mission path's chain (endpoint node 0, 5 nodes spanning ~18 units) built a one-node route
   that "arrives" where the guest stands. The game's InitAIPath (0x153720) leaves a 1-link first node by its
   single link and stops the chain at the first later node without two links. Ported; guard patrols (the
   kind-4 loops) are unaffected and pinned by a regression test.

Verified with the real viewer path (`nf_view::load` + `Sim::step_bond`), tests in
`crates/nf-view/tests/fix_drone4_live.rs`, both shown failing on the pre-fix code (no route, no 0x16,
0.04 units of movement in 600 ticks) and passing after (state 0x16, walk danim 0x37, > 2 units walked, a
≥4-node route chain). The fix-drone3 suite still passes. A guest also now correctly stops walking and stands
as a Civilian once her mission route ends, matching the game.

Deviations kept honest: the guests walk but don't chat yet — the game's BondTalk (0x1446b0, the head-track +
speech lines when unarmed Bond walks up to a guest) is still unported, and its random draws are therefore not
in the stream either (no determinism risk added).

## Evidence gathered, not fixed

**The C2-class "drawn but frozen/unhittable" guards (07000002 bathhouse, 07000006).** The dump harness
(`crates/nf-view/tests/dump_drone4.rs`) runs the real viewer path and records: at level start the 07000002
ch-21/23 guards are drawn but haven't started their state machines; within two seconds they enter WaitSwitch
(disabled, undrawn) waiting on their trigger plates — consistent with the game's design. The only drawn
drone at the bathhouse in the port is the civilian woman (skin 0x0500004F, team 2, HostageIdle class); the
user's screenshot shows a guard next to her that the port's dump does not contain. Raising channel 21
headless at Bond's spot did not re-draw the guards. The headless `drawn` flag is the port's own visibility
bookkeeping, so this dump cannot yet separate "the game's guard is a different drone" from "the port's
drawn flag misbehaves after a teleport". Next pass: walk Bond over the real plate positions (341/383)
instead of raising the channel directly, and damage-check the woken guard.

## Not done (with reasons)

- The 07000006 searchlight alarm **sound**/music event: the spotted→channel-55→guards-wake path works
  (fix-drone3's test pins it); the klaxon/music half needs its call identified in Searchlight_Update.
- The party level-start weapon-lock timing (the half-second armed frame on a warm load) and the
  bouncer/male-guest permanent aggro after seeing an armed Bond: not started.
- The 07000006 driver hiding on top of the crate stack: not started.
- The 07000005 patrol torso twist: fix-drone3's aim-state gate + ±0.9 clamp are still in and its tests
  pass; no new evidence this session.

## Files

`crates/nf-game/src/drone.rs` (the mission-state block), `crates/nf-game/src/nav.rs` (`Route::for_path` —
shared file, minimal change), new tests `crates/nf-view/tests/{fix_drone4_live,dump_drone4,probe_drone4}.rs`.
