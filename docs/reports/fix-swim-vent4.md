# fix-swim-vent4 — both reds green on the merged branch

Branch `s/fix-swim-vent4` = `s/fix-swim-vent3` (ca559c3) merged with `origin/master` (merge commit
b9db8b5; WORKLOG and scene.rs conflicts resolved keep-both). Two commits landed on top: `2652850`
(the 0700000c route) and `91e0964` (the escort RUN fix). The report file is `docs/reports/
fix-swim-vent4.md` (this file); the WORKLOG entry is appended at the end of `docs/WORKLOG.md`.

## Done 1 — `fix_mission_live::the_07000001_exit_works_end_to_end` (the 07000001 exit): GREEN

**The ping-pong the task describes does not exist on the merged tree.** With a temporary env-gated
route trace on Mayhew (skin 0x0500_004F, since removed): across the whole escort walk the route
index advances monotonically (0 regressions in 3761 logged ticks), the state never leaves
AllyLead 0x1d, and no OpenDoor/turn cycle appears. The creep advance vs `next_node` order, the
arrival radius and the door-4 re-entry all behave as the decompile says — I re-verified the port's
`follow()` order against the game's own chain before touching anything:
`LinkCreep_Handler` 0x15a2c8 (special-node first → the blocked test skipped while +0xBA8 stands →
increment), `LinkCreep_Increment` 0x159f28 (advance only under the lookahead and 0.4/clear-0.35),
`LinkCreep_ForNodes` 0x159810 (the special flag on the segment-start node, steps = |d|·2.5),
`SetupNextNode` 0x159668 — the port matches all four.

**The real defect: the lead WALKED instead of RUNNING.** `NDrone2_DSTATE_AllyLead` 0x15e698 case 3
calls `DroneAnim_CallAnim(0x1e)` = RUN on route codes 0/2 (the decompile's own arms); the port's
`update_mission_route` hardcoded `walk_anim(d)` for every caller, and the walk danim's root delta
carries Mayhew at ~0.02/tick — the trace shows a uniform 0.02/tick for the entire route (the
"stall" was just the walk running out of the test's escort window at the west stairs). ENTER
already started RUN [M 0x15e698 case 1], and the first UPDATE's walk request replaced it.

Fix (`crates/nf-game/src/drone.rs`): `update_mission_route` takes the MOVING anim per caller —
the game picks it in each DSTATE. AllyLead passes `anim_state::RUN` [M 0x15e698 case 3],
CivilianMission keeps its walk [M 0x15d860], KikoMission 0x34 (its ENTER's anim), KikoMissionRun
RUN (its ENTER's 0x1e), the AllyGoToGoal stand-in keeps the walk. Mayhew now covers the route at
~0.04-0.05/tick and walks onto the plate: `at_plate true`, `level_exit Some(0x07000002)`,
walked 172.8 — the test is green with its budgets untouched.

**One test-choreography change (documented in the test, not a re-time/re-seed)**: the test used to
teleport Bond onto the plate when Mayhew came within 4.5 of it (a workaround for the old pacing
stall). With Mayhew running the route correctly, the game's Bond is BEHIND the escort — the test
now joins Bond to the plate only inside the 2.0 latch ball. Reason this matters: the exit touch
volume is z 12.17..18.68 (placement 180 [D probe]) and Mayhew's r-0.4 toucher brushes its z-lo
face from z 11.77 — with Bond pre-placed on the plate, the level end fired ~1.6 units before the
at_plate latch could ever see Mayhew.

## Done 2 — `walkreal_0700000c_live` (the ST-1 plate): GREEN

**The report's geometry model was wrong; the disc evidence re-sited the stand-in.** Probes
(`probe_planes` raw 0x2E blocks, `segment_hit` floor maps):

- The old vent stand-in (13.0, 12.2, −154.3) hangs over the interior pit: the disc has **NO floor
  in y 9..13.5** anywhere at z ≤ −153.3 for x 7.2..13 — Bond fell 5.5 units to the 6.27 interior
  floor at the drop, and every walk after it ran 5 units UNDER the plate (the b2-era red: the
  crouch loop "reached" its target in xz while Bond was in the pit).
- The corridor ledge (11.27-11.32, z −153.14..−152.8) is sealed from the plate pad by the display
  case obj 143: x 4.559..6.962 × z −153.140..−153.044 × y 11.078..13.744, standing on the ledge's
  south edge — the pad (11.84, x 5.0..6.5, z −153.05..−154.95) is an island: pit 6.27 west/east/
  south, the case north. The report's "(targets (x−0.5, −152.95))" approach cannot cross: a
  crouched Bond at z −152.47 sits pressed against the case's north face (−153.044 + capsule 0.55).
- The "south walkway" at 12.26 across the pit gap (z −155.4) is the **pulley car** — dynamic geo,
  no static triangles [D probe_planes box dump empty], with a frame that stops a walk at its north
  edge. The game reaches the display ground through the unported 0xE2 vent/pulley route.

Fix (`crates/nf-view/tests/walkreal_0700000c_live.rs`, the route only): the vent stand-in drops
Bond onto the pad's south end (5.84, 12.93, −154.9) — the documented 0xE2 gap, now on real floor —
and the walk to the plate volume centre (5.84, −154.15), the touch, and the objective chain stay
real (the walk out: 2691 ticks, stuck 0, falls 0). Two sister asserts pin the regression: the old
spot must stay floorless (a `segment_hit` probe in the test) and Bond must land grounded on the
pad. `walkreal_0700000c_live` green end to end (ch 0x1b, objective 1 completes, the REAL camera
photograph ch 0x1c, the yard, the ladder, the gate E).

## Done 3 — the full four-crate suite on the branch (foreground, `--test-threads=1`)

- `nf-game`: lib 128/128; every test binary green (run after the RUN fix landed).
- `nf-formats` + `nf-audio`: **89 passed / 3 failed** — the failures are exactly the allowed
  `disc_golden` trio (`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_
  manifest`, `directory_tree_matches_the_golden_manifest`).
- `nf-view` (all 142 test binaries, two halves): **430 passed / 4 failed**, all four the same
  "E at the gate door loads 07000006" family: `level_carry_live` ×2, `level_end_live` ×1,
  `walk_07000005_live` ×1. **These are master's own reds, inherited by the merge**: I ran
  `level_carry_live` on `origin/master` in this sandbox and both tests fail there identically
  (`E at the door requests 07000006` / `LOAD_LEVEL on E still fires`); the other two share the
  same panic signature [S — same assert text, not individually re-run on master].

One small shared-file change: `crates/nf-formats/tests/pss_movies_live.rs` (fix-movies' file) —
`audio_sample_count_matches_stream_length` panicked on the slim ISO ("expected ~20 audio-bearing
movies, checked 0") instead of skipping; it now skips with a note when the image carries no movie
data (the header comment's own env gate; the retail-image path is unchanged). Without it the
suite policy ("only the 3 disc_golden tests may fail") cannot hold for any branch off this master.

## Files touched

`crates/nf-game/src/drone.rs` (the moving_anim param + the AllyLead RUN — minimal), `crates/
nf-view/tests/fix_mission_live.rs` (the Bond-join choreography only), `crates/nf-view/tests/
walkreal_0700000c_live.rs` (the route only), `crates/nf-formats/tests/pss_movies_live.rs` (the
slim-ISO skip guard, noted above), `docs/WORKLOG.md` (this task's entry), this report. The two
probe examples (`probe_pad.rs`, `probe_scoped.rs`) from the previous session are untouched; my
throwaway `probe_path` example was never committed.

## What's left / notes for the next session

1. The E-at-the-gate-door family (4 nf-view reds above) is a master-side regression from the
   recent merges — worth its own task (fix-input's keyboard regression test passes, so the break
   is elsewhere in the door/use path).
2. The Mayhew walk is now the game's RUN pace, but I did not re-tune anything downstream: the
   escort's OpenDoor choreography, PlayerInWay and the wait bands behave as before (no OpenDoor
   cycle appeared in the trace).
3. The 0700000c vent/pulley (0xE2 SimpleScripts) remains the documented gap; the crawl (creepwall)
   ports only the 05/0d/41 vents [D fix-swim-vent.md].

## Live checks for the main agent

1. 07000001, fieldprobe: Bond on the exit plate (−33.105, ~2.9, 15.43) + Mayhew parked on the west
   stairs' bottom (−33.0, feet 3.54, 11.80) — 0.37 outside the exit touch volume's z lo (12.17,
   placement 180 [D probe]). Does the game's Trigger_Hit 0x1b1fc8 count him (ch 0x10 fires) or
   not? This settles whether the port's r-0.4 drone toucher (a stand-in for the game's contact
   sphere vs the glist bounds) is too generous — the port fires; if the game does not, the drone
   toucher radius needs the game's real contact rule.
2. 07000001, gdbwatch on `NDrone2_DSTATE_AllyLead` 0x15e698 case 3 while the escort walks the
   courtyard: confirm CallAnim(0x1e) fires on the route's moving ticks (the port's fix models
   exactly this — a confirm-only check).
3. 0700000c, fieldprobe: is the display case obj 143 (x 4.56..6.96 on the ledge's south edge)
   really solid in the game — i.e. the game's Bond reaches the plate pad only through the vent/
   pulley? If the game's Bond WALKS from the corridor onto the pad, the port's case collision
   (or a missing opening in it) needs a second look.
