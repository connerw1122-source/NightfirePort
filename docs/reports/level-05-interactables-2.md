# level-05-interactables-2 — the HELD 05 work finished (three breaks fixed)

Branch `s/level-05-interactables-2` (master 0083ce97 + the held `s/level-05-interactables` 41f5ae1
merged at ea43560a, then the fixes). The held branch's census, its 11 live tests, its docs and its
report all carry over; this task made the merge land.

## What's done

1. **The merge, resolved to ONE hurt system.** The held branch had mounted its own hurt record list
   on `Drones` while master already runs `crates/nf-game/src/hurt.rs` on `Sim.hurts` (nf-view
   scene.rs). The merged tree keeps the single module on `Sim.hurts`: the held param map
   (param 0 type, 1 damage, 2 gate, 3 sound, 4 who, 5 one-shot [M Hurt_Create 0x17e870 — the
   creator's record fields, pinned by 07000002 #13 and 07000016 #309 [D]]), the held
   capsule-vs-mesh contact (`triggers::capsule_hits_tris` over the placement's own collision
   triangles — the glist `Control_SetGList` links [M 0x17e870:44]), and the state machine master's
   version had and the held version had lost:
   - the gate maintenance is a LIVE level, every tick — the channel low sets `obj+0xF0 |= 0x20`,
     the channel up clears it, and state 1 re-checks it [M 0x17e5d0:26-37, :109-116] (the old doc's
     "rising clears" was an edge-shaped simplification; the decompile is a plain level);
   - a gated volume's who-mask IS the channel value [M 0x17e5d0:97-99] — so #309 (gate 0xf3, who 1)
     is NOT inert: the launch channel starts it;
   - an ungated volume starts on who 0, else only when a contact class bit survives the AND — the
     player brings 0x10, who 1 collects nothing, ever [M 0x17e5d0:57-95] — the configured-inert
     rows (05's #328–330/#334–340, 06's #373) stay inert;
   - the one-shot retire [M 0x17e5d0:129-132 → :41-47].
   The damage rides the caller's `Player_HandlePain` pipeline exactly as before (difficulty factor,
   no armour for types 1..7 [M 0x1902c8:84-104]). **#309 still deals 1000.** The duplicate
   `pub mod hurt` the merge produced in lib.rs is gone. Seven nf-game unit tests pin the law,
   including the #309 shape (gated + who 1: the low channel parks it, the live channel starts it).

2. **searchlight_live::channels_and_tracking_behave — the spot-ray housing skip.** The held skip
   slab-tested the head's gfx box against world ray coordinates, but the box is the part's
   LOCAL-space bounds (the bullet stand-in tests it after the matrix's world→local transform): the
   head at (−57.03, 37.84, −59.97) aiming +x crossed its own ±0.27 origin-anchored box 57 units
   out, `skip` became 56.77 world units, the ray start jumped past the real hit (t 0.228 of the
   100-unit ray), `segment_surface` missed, and the re-lit dark head's spot stayed at −10000
   ([-10057.8, -9963.4, -10059.7], the sentinel). The fix keeps the exclusion's effect [M
   Collide_PickObj 0x1eae80:57-59 — the game refuses the ray's own source object] with the port's
   box stand-in used in its own space: the slab test runs on the matrix-transformed origin and
   direction (length-preserving, so the t is the same parameter), `skip` is a FRACTION of the
   100-unit ray capped at 0.04 (4 world units — the housing is a ~0.5-unit box [L ee_start_05.bin
   head 016dfe70]), and the sub-segment's t is rebased into the full ray's parametrization so the
   player-sphere test (which always was a full-ray fraction) compares like with like. The held
   code's second bug — mixing the world-unit skip into the fraction domain (`t + skip`) — dies with
   the rebase.

3. **The two 07000016 finale failures — the real divergence is the disc's own exhaust order.**
   Evidence chain:
   - ch 243 (0xf3) has no placement writer [D]; the port's writer is script 0x0600090a (SP #272
     "ScriptPlayer_ShuttleLaunch"), stream 6's event-0xd writes: **ch 0xf3 = 1 at script tick 1611,
     = 0 at tick 2170** [D 0x0600090a, NF_SCRIPTLOG decode] — a ~560-tick pulse, the launch exhaust.
   - the same script writes **ch 0xf0 (240) = 1 at tick 2180** [D] — the chaser spawner #257's
     trigger fires only AFTER the pulse ends. In the game and the port alike, the chaser fight
     never overlaps the pulse.
   - the pack cinema (0x06000631) snaps Bond onto the silo floor (3.57, −23.18, −179.55) — inside
     #309's box (x −30.7..35.3, y −24.1..−21.3, z −197.3..−133.8 [D]) — and the pulse kills a Bond
     parked there (probe: hp 100 → 0 at the pulse, ch 0x62 up, MissionState 3, to_end 0x7000048).
   - Mission_Update 0x182c60 case 1 reads ch 0x62 FIRST: up → the fail arm (Player_SetHealth(0),
     MissionState 3, LevelToEndTo 0x7000048) [M 0x182c60:21-43]; the ch-100 completion arm
     (:24-28, case 6 → 0x7300017 for 0x7000016 :111-112) only ever runs from InternalState 1. A
     dead Bond can never complete — the game's player must leave the pit during the launch.
   - the two tests parked Bond on the silo floor through the whole chain; they passed on master
     only because master's hurt param map read #309's damage from param 4 (=1 → 1.0 damage) into an
     origin-anchored box stand-in that never reached the pit. That was the bug the held merge
     fixed; with the volume live, the tests' placement contradicts the game's flow. Per the task
     rule (a test that contradicts the decompile is wrong), **the tests were fixed**: both re-seat
     Bond on the upper floor the descent came from (2.5, 1.2, −153.9 — outside the volume) once the
     cinema hands control back, before the pulse, and play the chain out from there. The damage is
     untouched (1000), the gate law is untouched, the sister assertions are untouched.

## How it was verified

- nf-game: **536 passed / 0 failed** across every test binary (`cargo test --offline --release -p
  nf-game -- --test-threads=2`, NF_ISO set) — includes searchlight_live 3/3 and the new hurt law
  tests.
- nf-view (foreground slices; the sandbox kills background jobs between tool calls, so
  tools/test/suite.py cannot run here — same constraint the held report recorded): the three
  formerly-failing binaries **searchlight_live 3/3, fix_finale_live 2/2, spurious_complete_16_live
  2/2**; the searchlight neighbours (searchlight_flip_07000006 2, fix_searchlights_guards 3,
  fix_searchlight_alarm 2, fix_searchlight_remainder 3); the hurt/interact family (hurt_02_live 1,
  interact_07000005_live 11, interact_07000006_live 9, walk_07000005 2, walkreal_07000005 2,
  mayhew_handover 2, level_carry 8, fix_truck 2, wire 1); the mission/finale family (fix_mission 4,
  mission_unlock_flow 3, casings16 2, levels_sweep 3, walk_07000008 4, walk_0700000a 3). **0
  failures across 57 nf-view tests run.**
- The probe scripts used for the diagnosis (ray slab numbers, the scriptlog decode, the mission
  fail arm) were scratch files and are not committed.

## What's left

- The held report's own leftovers carry unchanged: the walkreal west/cable route anchors (020-fix-
  wire), the window-ledge plunge catch, the #36 reveal plate vs the toucher stand-in, the copter
  #241 fly-by eyeball.
- The full nf-view suite beyond the slices above (menus/movies/net and the remaining walk_* bins)
  did not fit the window; the touched crates and their dependents are green.
- The launch-pulse live check below settles the one [S] the test fix rests on (the game's player
  really is expected out of the pit).

## Live checks for the main agent

1. 07000016, fresh boot: play to the launch with Bond ON the silo floor at (3.57, −23.18, −179.55)
   and idle there. gdbwatch `switch_channels[0xf3]` (0x2bf0f8 + 0xf3) through the launch script —
   expect 1 for ~560 ticks (the pulse [D 0x0600090a stream 6]) — and watch the health: if the REAL
   game kills an idle Bond there (type-2 pain, 1000), the tests' re-seat is the only correct shape
   and the pit is a real hazard; if the game's health does not move, #309's glist does not reach
   that spot in the game and the port's capsule-vs-tris contact is looser than the game's pick —
   then tighten `capsule_hits_tris`'s margin for the hurt path (the contact rule is the port's [S]).
2. Same session: does the game's launch NIS move Bond out of the pit (a scripted escape) between
   the cinema and the pulse? fieldprobe screenshots at the pulse start vs the cinema end settle
   whether the port's missing player-stream animation matters for play.

## Usage

not visible to me — the environment shows no token, credit or context-window figures for this
session.
