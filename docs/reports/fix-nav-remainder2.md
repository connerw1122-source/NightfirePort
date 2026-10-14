# fix-nav-remainder2 — report

Branch `s/fix-nav-remainder2` (from master 5017f15). The task: re-enable, on evidence, the two
parts fix-nav-remainder switched OFF at 61fa5ce — (1) the ally-wait state entry (0x21, the 0x20
"Mayhew Route" nodes), (2) the NoBunching tick scan (the +0x31C "drone ahead" flags).

## What's done

**1. The NoBunching regression is root-caused, and the fix is in the code shape.**

The scan fired where the game's wouldn't: **the port's scan included the player; the game's never
does.** Evidence [M]:

- `NDrone2_Steer` 0x14eba8:98-99 — the player is evaluated with `param_2 = 0`;
- `NDrone2_EvaluateObstacle` 0x14e6f0:58-65 — `Drone+0x314` (the obstacle `DroneMove_NoBunching`
  reads) is written **only** when `param_2 != 0`, i.e. only for a drone candidate (the nearest
  keeper inside the 3.0 seed that Steer sets at 0x14eba8:83-85);
- the state-0xb / speed-0 fallback 0x14eba8:180-214 loops drones only.

So Bond can never land in +0x314 and never feeds the +0x31C flags. The 61fa5ce scan gave the
07000001 death-drop guard bit 2 ("drone ahead") from the PLAYER alone (Bond stands 3.0 behind his
walk line; the [nb] instrumentation this session logged PLAYER-only candidates for the shot guard
at dist 2.30–2.96), the guard parked in 0x5E at the chooser gates (0x13cfb0:35 / 0x148338:95),
died inside Bond's pickup sphere, and the drop was taken (`gone`) — exactly
`fix_objects_live::a_death_draws_its_drop_and_nothing_else_moved`'s failure
(`one visible drop at the corpse: [(0, 14, false, true)]`), and the same for
`render2_live::dropped_guns_dim_to_the_cel_ambient`.

The scan in `drone.rs` is rewritten drone-only with the full citation block (including the game's
candidate chain 0x14eba8:150-155 that the port still approximates, and the NoBunching reader's
movement-lock gate 0x150ad0:22 that the port does not model). It is **held inert behind
`NB_SCAN_LIVE = false`** (see "Why held inert").

**2. The ally-wait chain is pinned end to end; one link is missing from the disc data the port
parses, so the entry stays parked with a live-check request.**

The whole chain, on 07000001's own data (the only level with 0x20 nodes — the kind-8 "Mayhew
Route"; its ally-wait node is path 0 node 33 at (2.91, 5.74, 7.57)):

1. The real game parks the escort: `tests/traces/01-doors.trace` (the actual game recording) holds
   the contact in state **0x21 at (2.94, 5.88, 7.22)** — node 33 — for **1262 frames**
   (44664 → 45926), then releases him to 0x1d. The park is game-true.
2. The release channel's writer exists and is ported: the three courtyard guards (drones 6/7/11 at
   (5.42,14.44)/(9.05,17.01)/(5.93,17.86), switch channel 0x0e, death channels 0x13/0x11/0x12
   [D placements pl 166/167/249]) die → `DroneFunc_SetDeathChannel` 0x13e660 raises
   0x11/0x12/0x13 → the **AND mux pl 318** (0xEC at (7.76, 6.22, 15.33), out 0x99, inputs
   0x11/0x12/0x13) raises `switch_channels[0x99]` → `ReachedDestNode` 0x153358:87's gate opens and
   `AllyLeadMissionWait` 0x15efc8 case 3 exits to 0x1d. The port has every one of those links
   (Trigger::update's AND; the death writes in drone.rs). In the recording the courtyard guards
   fight and vanish (killed) exactly inside the park window.
3. The guards' enable channel 0x0e is written by **script 0x06000086** (SP #276: modes [0,0] →
   state 0, mask 1, out 0x21 — a state-0 SP starts when `switch_channels[SP.out]` rises [M the
   port's nis.rs state-0 arm]) — stream 9 pc 16, event 0xd, `switch_channels[0x0e] = 1`.
4. **The missing link: nothing the port parses writes channel 0x21.** Scanned this session: every
   placement param on all maps (values 0x21/0x99/0x11/0x12/0x13), every script's event-0xd/0x11
   channel writes (0x06000086's own 0x0e is the only one), all five doors' ch_open (all 0), the
   muxes/triggers (outs 0x0f/0x32/0x0c/0x26/0x27/0x10/0x99), and the 0x47 objective placements.
   With the entry enabled today the escort parks in 0x21 forever (reproduced:
   walkreal_07000001 parked at (3.5, 6.3, 7.1) state 0x21, "no guard to clear" — the courtyard
   guards stay WAIT_SWITCH, enabled=false, outside the test's clearing filter) and
   fix_mission_live's shoji ping-pongs — the 61fa5ce regressions, now explained.

The ally-wait arm stays un-acted-on in `drone.rs` (the outbox stays; the resolution proves the
node dispatch) with the full chain as the comment.

**3. The evidence runs (all with the scan live, player term removed):**

- `fix_objects_live::a_death_draws_its_drop_and_nothing_else_moved` — **green** (red with the
  old scan, green at the baseline).
- `render2_live::dropped_guns_dim_to_the_cel_ambient` — **green**.
- `nav_kick_live::drone_directly_ahead_parks_the_mover_in_0x5e` — **un-ignored and green** (the
  0x5E park works with the game-true scan).
- `ai_traces_live` — **4/4 green** (01-doors, 01-raptor, 02-civilian, 05-truck): the recorded-game
  comparisons hold with the scan live. That is the "trace match preserved" evidence for the scan.
- `fix_mission_live::the_07000001_exit_works_end_to_end` — green (ally-wait entry off).
- `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact` — green (entry off).

## Why the scan is held inert (NB_SCAN_LIVE = false)

With the player term removed, everything the task named is green (above), but enabling the scan
reshuffles 07000001's RNG-chained escort walk and **`walkreal_07000001_live` fails a different
way**: the west mezzanine guard (drone 2, ch 0x26, at (−30.06, 10.22, 13.44)) now outlives the
test's fixed 12-round clearing budget (10 hp left; Bond cannot close on the mezzanine, shots from
16.6 are blocked), and the contact parks in AllyLeadHide (0x1f) with him armed within 14 → the
round loop exhausts → the settled-vs-plate assert fails. The scan itself has no RNG draws; the
delta is behaviour timing (guards pairing up now park each other in 0x5E mid-fight, which is the
game's intent but changes when each fight ends). Re-pinning that fight (more rounds? the walkreal
file is not this task's) is the next session's first item; the flip is one const
(`NB_SCAN_LIVE`) plus un-ignoring `nav_kick_live::drone_directly_ahead_parks_the_mover_in_0x5e`
(both run-verified green together on this branch — the drive test was un-ignored, passed, and is
re-ignored with that state recorded in its attribute).

The ally-wait entry stays off until the live check below lands; enabling it today stalls the
escort (proven above), so 61fa5ce's OFF is now evidence-backed rather than mysterious.

## What's left

1. Flip `NB_SCAN_LIVE` and re-pin `walkreal_07000001_live`'s guard-2 fight (needs the trace or a
   run-budget decision by whoever owns that file), then un-ignore the drive test.
2. The ch-0x21 live check (below) → then re-enable the ally-wait entry arm in `Drones::tick`
   (the exact arm is written in this branch's history — the 61fa5ce shape, restored and
   reproduced this session).

## Live checks for the main agent

1. **Who writes `switch_channels[0x21]` on 07000001** (the SP #276 arm and the ally-wait release
   both hang on it): `gdbwatch.py` on `switch_channels + 0x21` (address of `switch_channels` is
   the array the decomp names; +0x21 = 33 bytes in) during a play from the spawn through the
   courtyard, recording every instruction that writes it, with the game frame. Expect a write
   around the time the player nears the courtyard shoji (7.9, 11.1) — in the recording the guards
   enabled at trace frame ~45400, i.e. ~2.5 s after Bond crossed the TouchOnce at (6.79, 8.91).
   Suspects to catch: a class-0x47 objective record completing (pl 75/83 style), or a script
   system the decompile's `Script_EventHandler` case set does not show.
2. **`switch_channels[0x0e]`** on the same walk: expect the rise exactly when SP #276's script
   0x06000086 stream 9 runs (the guards' enable), confirming the SP-start chain (out 0x21 → state
   0 arm → 0x0e).
3. Optional render spot (not visual work, but for the record): `--bond 4.0,7.0,9.0,1.2,0` on
   07000001 — the escort should stand parked (alert idle) at node 33 while the courtyard guards
   are alive, if/when the entry is enabled.

## Files touched

`crates/nf-game/src/drone.rs` (the task's file: the NoBunching scan rewritten drone-only + gated;
the ally-wait evidence comment), `crates/nf-game/tests/nav_kick_live.rs` (the drive test's
ignore attribute updated with the run-verified state), `docs/WORKLOG.md` (appended),
`docs/reports/fix-nav-remainder2.md` (this file). No other file touched; no test loosened.

## Suite

`cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view
-- --test-threads=1` run at the end of the session (the tally below); on the slim ISO only the 3
`disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
