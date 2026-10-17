# level-09-complete — 07000009 "Night of the Agent" (Tower A) start to finish

Task 709b: play the level from its start through the real input path, complete EVERY objective,
and exit the way the game does. Branch `s/level-09-complete`.

## What's done

`crates/nf-view/tests/walkreal_07000009_live.rs` rewritten (3 tests, all live through
`nf_view::load` + `Sim::step_bond` — the real input path, no teleports after the spawn):

1. **`the_elevator_activation_writer_pinned`** — the activation mechanism's shape [M
   0x1a5170:121-122 + 0x1d8d78:108-110; D placement #70 params]: SP #70 "SCRIPTPLAYER_Elevators"
   (mask 4, out ch 57 = 0x39) sits at its placement origin, its mode-64 preload creates the panel
   entity 0x020005EA at load (the E target), and the LIFTDOORS_*_ENDLEVELDOORS SimpleScript pair
   waits on ch 0x39 at the shaft mouth (mask 1 — no E arm [M SS_Activate 0x1a9ea8]).
2. **`walk_tower_a_start_to_finish`** — the full walk, 4260 ticks, 0 stuck, 0 falls:
   - the car-park start #68 (3.59, −377.08, 36.76 [D]); the arrival plate #248 (TOUCH_ONCE, out
     ch 0x51) fires at the spawn [D box 2.62..4.71, −377.08..−375.69, 35.83..37.99];
   - the mission rows after the first monitor ticks: obj 0x4000016 (flags 2) sits fail-pending
     (2), obj 0x4000017 announced (3), obj 0x4000018 enable-gated (1) [M mission.rs init
     0x182270:98-105 + the 0x1827d8 machine; D mission/c07000009.rs];
   - up the stairs and across the lobby on the guards' own AI graph (lenient nav — see Blockers);
   - **the REAL E-press on the elevator panel** (aim loop at the panel's world centre [D gfx
     bounds −18.38,−368.53,24.83..−17.78,−367.92,25.20], E pulses — the elevator-buttons
     mechanism, no channel stand-in) → ch 0x39 → the AND mux #274 → ch 0x46 [D] → **objective
     0x4000017 completes (state 4)** [M 0x1827d8:33-51]; SP #70 plays its 25-tick FX (entity B
     0x020005EB at t=16 [D script stream 0]) and deletes (stay 1) [M 0x1dbda8:142-159, 190-192];
   - the LIFTDOORS SS pose open and disarm to state 0 [M SS_Update 0x1a9948];
   - into the cab → TOUCH_ONCE #67 → ch 0x32 → **LOAD_LEVEL #63 fires to 0700000a** [D];
   - the exit mission state: obj 0x4000016 still 2 (never failed — no alarm channel ever fired:
     0x61/0x7C/0x7D off all walk), obj 0x4000017 = 4, obj 0x4000018 = 3 (its enable ch 0x46 fired
     at the panel [M 0x1827d8:55-67]).
3. **`the_alarm_switch_fails_objective_1`** — the fail arm (objective 1's sister): E on an alarm
   switch (ch 124 = 0x7C [D placements #78/251/…]) → the OR mux #278 puts ch 0x61 on [D] → the
   fail objective fires → ch 99 + the row's state 5 ("Alarm Raised") [M 0x1827d8:128-131].

## The objective list and how each completes

| # | Label | Completes on | Writer on this level | Evidence |
|---|---|---|---|---|
| 0x4000016 | "Don't let the security guards raise the alarm" | INVERTED (flags 2): fails when ch 0x61 fires | the 11 alarm switches (7× ch 0x7C, 4× ch 0x7D [D placements]) → OR mux #278 → ch 0x61; the walk never touches them | [D mux #278 + switch placements; M Switch_Activate 0x1b02a8, 0x1827d8:68-77] |
| 0x4000017 | "Get to the lobby and activate the main elevator system" | ch 0x46 | the REAL E-press on SP #70's panel entity → ch 0x39 → AND mux #274 → ch 0x46 | [M 0x1a5170:121-122, 0x1d8d78:108-110, 0x1dbda8:221-229; D mux #274] |
| 0x4000018 | "Take main elevator to office floor" | ch 0x47 on 0700000a | enable ch 0x46 — announced (3) here when the elevator activated; completes on the next level | [M 0x1827d8:55-67; D record enable 0x46] |

Mission completion rides both rows live [M 0x1827d8:33-51, 119-123] — the walk's exit state
proves it (no alarm + ch 0x46 held by the AND mux's per-tick recompute).

## Blockers found

**None in the port code** — the level chain (triggers, the panel SP activation, the SS lift
doors, the exit LOAD_LEVEL, the mission rows) all worked; the walk needed no source fix this
session. What the walk hit, all route-level:

- The lobby's roaming guards and the raised desk platform body-block direct steers in a
  guard-state-dependent way ([S] walk observation: the same leg passes in one run and wedges in
  another) — the route uses the guards' own nav graph (`walk_nav_to`, lenient skip-and-re-aim).
- The security door E works (verified OPEN in-run: state 0x4, pos01 1.0) but the north-wall
  furniture belt (a console at ≈(−23.5, 38.2)) climbs under the use-drift and wedges the player
  1.3 above the floor — left out of the route, documented in the test ([S] walk observation).

## Data fix

The level doc's census line "Monitor ×1 | 0x30" was **wrong**: 07000009 carries ZERO 0x30
placements in all 40 of its maps [D probe of every map's flags&0xff], and the game creates
monitors only from parsemap case 0x30 [M 0x1d04f0:175-177 → Monitor_Create 0x1b08b0]. The
security-room monitor WALL is baked art with no object — there is nothing to E. The doc line and
the test now pin the absence.

## What the walk still cannot do (with the exact reason)

- **The laser toggle switch #71** (SCRIPTPLAYER_Lasers, ch 0x37): its placement transform is the
  ORIGIN — its world position is scripted (animation 0x06000627) — and the port's switch use
  targets placement positions, so the real E cannot reach it. Needs the switch's live scripted
  position fed to the use pick (the s/laser-scripts work, held for the main agent's render check).
- **The SS lift-door open ANIMATION** is draw-side (the state machine, the channel gate and the
  posed collision are ported and asserted; the main agent renders the art).
- The security-room interior traversal (the furniture wedge above) — optional, no objective or
  channel inside [D].

## How it was verified

- `walkreal_07000009_live` 3/3 and `elevator_buttons_09_live` 3/3 on the final tree (twice for
  the walk binary through its route iterations).
- The four-crate fast-runner suite was launched on this tree (build ok, 519 binaries); the run
  was in flight at report time with only the 3 allowed slim-ISO `disc_golden` failures
  (`docs/tools/test-suite.md`) — see the WORKLOG entry for the final count; the branch-touching
  binaries ran green directly.
- `nf-game --lib` 276/276 (the channel-persistence work merged on master, re-verified).

## Left for the next message

- The full-suite total line for the final tree (the run was in flight; the branch-touching
  binaries ran green directly — see above).
- The laser toggle's scripted-position use target (with s/laser-scripts, render check pending).
- Optional depth: the guards' alert→alarm-switch flow (the port's Switch_Activate has only the
  Bond caller — a guard can never raise the alarm in the port; the game's guards can — a
  fidelity gap worth a future task if the user wants guard-raised alarms modelled).

## Live checks for the main agent

None required — every pinned claim is [M] decompile + [D] disc data, checked headlessly. Render
checks (draw-side, not mine): the LIFTDOORS SS open animation frames, the laser grid art states.

## Usage

- Token/credit/context figures for this session: not visible to me. (No usage, cost, or
  context-window numbers are shown anywhere in this environment; nothing to copy.)
