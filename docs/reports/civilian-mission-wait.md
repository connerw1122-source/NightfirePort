# civilian-mission-wait — CivilianMissionWait (state 0x17)

Branch `s/civilian-mission-wait` (from origin/master f09f3c6c; master had not moved when this was
written, so the branch tree is the final tree). Task 735, the uncited-gameplay audit's runner-up:
mission civilians must pause at wait-channel route nodes (CivilianMission msg 0x22 [M 0x15d860
case 0x22]; `CivilianMissionWait` 0x15dbb0) — the port had no msg 0x22, so they walked straight
through.

## What's done

All in `crates/nf-game/src/drone.rs`:

- **msg::WAIT_PAUSE (0x22)** — the mission-civilian wait request, with the sender evidence in its
  doc comment (below).
- **CivilianMission (0x16) case 0x22** [M 0x15d860:130-136]: the wait request parks the walker in
  `CivilianMissionWait` (0x17), except the voice-0x13 skins stay (the same carve-out the state's
  ENTER applies for KikoMission).
- **CivilianMissionWait (0x17)** — `NDrone2_StateFuncs[0x17] → 0x15dbb0` [L 0x29b300 entry 0x17]:
  - ENTER [M 0x15dbb0:20-35]: the vision flag (`+0x4F8 |= 0x10000`; the `0x2000` "may notice" bit
    has no port model, matching the sibling ENTERs), stand idle at alert < 0.66 else the
    look-around 0x21, **timer C armed at now + FRAME_RATE_INT·2 = 2 s** (`+0xCC4` [M :29]), and the
    `+0xCD8` wait latch cleared to 0xff.
  - UPDATE (case 3) [M :39-67]: no armed person near → BondTalk and hold the wait (the BondTalk
    call itself is unported, as fix-drone4 documented for the whole civilian family); an armed
    person → the own-sighting flag, then the voice-0x10 team-1 Attack (0x56, with `+0x44 = 1`) or
    CivilianScared (0x12). The prop-0x33 armed-intruder alert chain (:45-62) is unmodelled, the
    same deviation the port carries for CivilianMission's UPDATE.
  - TIMER_C (case 0xc) [M :87-111]: `NDrone2_PreDroneControl` 0x148db0:97-111 sends msg 0xc to
    self when the armed `+0xCC0/+0xCC4` clock expires; the state re-arms the 2 s wait while Bond
    is inside the 0.525 rad view cone AND within 3.0 (2D) — the same conjunction
    `NDrone2_BondWithinAngle` 0x154f98 plus the local `Vec_Dist2D < 3.0` check models — else
    resumes CivilianMission (0x16). The port reuses its `bond_band(3.0, 0.525)` helper (the
    escort's angle+radius band), which computes exactly that conjunction.
  - msg 0x22 while waiting re-arms the clock [M :136-141].
  - sound alarms (0x11-0x16/0x1e) → `DroneVision_EnemyAlerts`, forced attack (0x1f) → Attack, and
    the impact family (case 6/8 → `HandleImpact(0x56, 0)`, the explosive-class 7/9/0x17-0x19 →
    `HandleImpact(0x56, 1)`): the bullet/punch resumes ride the port's `impact_args` /
    `on_punch` tables (the new state joins the CIVILIAN_MISSION row — resume ATTACK, allow false
    [M 0x15dbb0 case 8/6]); msg 0x21 never reaches the state's switch because the port's
    pre-dispatch already gives it the ATTACK resume the case asks for [M :132-135 →
    `ConsiderExplosive(…, 0x56)`]; the shout-scare civilian list gained 0x17 (the game's Wait
    handles 0x11/0x12 through EnemyAlerts exactly like 0x16 does, and the port's civilian
    shortcut is the scared state).

## Where msg 0x22 comes from (the honest limit)

The decompile holds **no literal sender** for DSTATE msg 0x22. Checked 2026-10-16, by
disassembly (the Debian objdump has no MIPS backend, so capstone over the ELF):

- all 25 call sites of `Drone_SM_SendMsgSelf` 0x175a38 / `Drone_SM_SendMsg` 0x13a630, with the
  a0 (msg id) setup at each — none passes 0x22;
- all callers of `Drone_Message` 0x13a678 and `Drone_SM_BroadcastMsg` 0x175aa8 — 0x1a/0x1f/0x12/
  0x11, never 0x22;
- every `li reg, 0x22` in .text mapped to its enclosing function — the DSTATE-region hits are
  `DroneAnim_CallAnim` ANIM ids (Idle/Alert's look-around 0x22), not messages;
- tail-call (`j`) variants of the send helpers: none.

So the id reaches the switch data-driven. The level-side data points the same way: 07000005's
drone placements carry **wait channel 0x22** (placement param 3 → `Drone+0x134`) and trigger 305
/ mux 25 raise channel 0x22 [D docs/systems/triggers.md] — the same number the DSTATE switch
sees. The tests drive the id through the port's real message entry (`Drone::send`), which is the
path any data-driven sender feeds; the state machine's behaviour is what the port pins.

## Tests

`crates/nf-view/tests/civilian_mission_wait_live.rs` (opt-in NF_ISO, level 07000007 — the party's
dtype-10 guests carry prop 0x1D with six kind-8 mission paths under their placements [D, WORKLOG
fix-drone4]; state/msg ids written raw so the file compiles against the pre-fix master):

1. `a_mission_guest_pauses_on_the_wait_message_and_resumes` — a walking guest (0x16, walk danim,
   held route) receives msg 0x22 through `Drone::send` → 0x17, stand idle, timer armed; she
   freezes (0.3 units over 100 ticks while walking covers ≥ 0.6); the expired clock with Bond far
   resumes 0x16 and she walks again. **Fail-first verified**: with the pre-change `drone.rs`
   (git checkout of 89d76c7b^) both tests fail at the 0x17 state assert.
2. `the_wait_holds_while_bond_stands_close_ahead` (sister) — Bond parked 2.0 units ahead of her
   facing keeps the wait alive past the first 2 s clock (the case-0xc re-arm), and a second
   wait request re-arms it again; Bond leaving ends the wait and the route resumes.

Results on the branch: 2 passed. Pre-change: 0 passed / 2 failed.

## What was run

- `cargo test --offline --release -p nf-view --test civilian_mission_wait_live -- --test-threads=1`
  — 2 passed (after the change), 2 failed (before it).
- `fix_drone4_live` (the party-guest walking gate, re-run directly on the final tree): 2 passed —
  the msg-0x22 arm does not disturb the normal CivilianMission walk.
- The full fast runner (`tools/test/suite.py`, NF_ISO on the slim on-foot image) on the final
  tree: build ok, 451 test binaries, then the full run finished (48 min).

## Suite (final tree)

- Total line: **1945 passed, 6 failed, 72 ignored (48 min)** — the 6 are the allowed set: the 3
  slim-ISO `disc_golden` tests and the 3 pre-existing pylon probes
  (`probe_pylon_render::probe_render_pylon_only`,
  `fix_0c_pylon_live::the_0c_pylon_fills_the_view_from_the_closeup_spot`,
  `fix_0c_pylon_live::the_0c_pylon_is_on_screen_at_the_user_spot`), the same set the previous
  sessions' final runs list. `civilian_mission_wait_live` passed inside the suite.

## What's left / notes

- The msg-0x22 sender is data-driven in the game and not identified to a specific table (see
  above). If a later task finds it (script/placement side), wiring it end-to-end is a small
  follow-up: the DSTATE side is complete.
- The port's msg 0x22 request is delivered by level scripts/placements on the levels where the
  game sends it; nothing in the port sends it yet, so a mission civilian waits only when
  something asks. The player-visible symptom ("they walk straight through") is fixed at the
  state-machine level; whether the shipped levels' civilians actually pause depends on that
  sender wiring.
- BondTalk (the wait state's no-armed-person bark) stays unported with the rest of the civilian
  talk family (the audit's separate talk-trio item).

## Live checks for the main agent

1. **The msg-0x22 sender**: on 07000007 (or any level with prop-0x1D kind-8 walkers), set a
   gdbwatch (write, 4 bytes) on a mission civilian's `Drone+0xCC4` (the timer-C deadline) and
   play until a guest pauses; then watch the msg id the SM delivers — if `DSTATE_CivilianMission`
   0x15d860's switch sees 0x22 in a real session, note the caller on the stack. Expected: the
   write lands right after a route node; the value is GameState + 120.
2. **The pause length**: fieldprobe `Drone+0xCC0/+0xCC4` on a waiting civilian: expected
   +0xCC0 = 1, +0xCC4 ≈ start + 120, refreshed every 2 s while Bond stands in front, and the
   state back to 0x16 (read Drone+0x5A0-ish / the SM id) within ~2 s of him leaving.

## Usage

not visible to me
