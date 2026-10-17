# tase-scream-broadcast — the tase scream alerts nearby guards

Task 736 (the uncited-gameplay audit's runner-up), branch `s/tase-scream-broadcast`
(from `origin/master` f09f3c6c).

## What's done

**The tase ENTER now screams, exactly as the game's case-1 tail does.** `NDrone2_DSTATE_Taser`
0x162b98's case-1 tail [M, disassembly checked at 0x162d0c-0x162d40] gates the scream on three
things and then broadcasts:

1. `behaviour_util_getProperty(0x32, drone+0x4d8)` — the scream behaviour property must be set
   (`jal 0x1c7d98, lw $a1,1240($v0)`); zero → return, no scream.
2. `+0x4F8 & 8` clear — the once-per-life bit, THE SAME bit `DroneFunc_SendHurtMessage` 0x145e28
   checks (:11-13) and sets (:32) — the hurt scream and the tase scream share it.
3. `lbu $v1,197($a0)` = Drone+0xC5, the active type id [M docs/fields/drone.md "+0xC5"], must not
   be 2 (the port's `dtype`).

Then `+0x4FC |= 2` and `NDrone2_DroneAlertToPosition(0, 0, 0, obj, 0x12, obj+0x30, obj+0x20)`
[0x162da8-ish call] — msg 0x12 (SCREAM) with the victim's position (obj+0x30) as the alert source;
the function fills the global alert record (0x2990a0) and broadcasts through
`Drone_SM_BroadcastMsg(0x12, record, 0x1e, …)` [M 0x14a498].

The port's `handle_taser` TASER-ENTER arm (crates/nf-game/src/drone.rs) gained the arm after the
fall-wait setup (the game's order): `if self.prop(d, 0x32) && !self.screamed && self.dtype != 2`
→ `self.screamed = true; self.pending_shout = Some((msg::SCREAM, self.pos))`. The broadcast rides
the existing `pending_shout` queue — the shared pass drains it to every present, enabled drone
within 20 u [M the hurt scream's broadcast shape; `Drone_SM_BroadcastMsg` 0x14a498 tail] — the
same mechanism the punch/bullet scream (`send_hurt_scream`, DroneFunc_SendHurtMessage 0x145e28)
already uses. The alert record's cel word (obj+0x20) has no port-side consumer; the scream stays
local like the hurt path's [*S*, the existing documented shape].

Before this change the port screamed only on bullets and punches (`send_hurt_scream`'s callers) —
the uncited-audit runner-up's symptom.

## How it was verified

- **Fail-first shown**: with the new arm stashed (`git stash`), `tase_scream_live::
  the_tase_enter_screams_and_only_once` fails exactly at the scream assertion — the guard falls
  into DSTATE_Taser 0x4a under a held Stunner beam (the real input path: `switch_to Some(0x4a)`,
  the press-and-hold fire through `step_bond`) and `screamed` stays false. With the arm restored
  the test passes.
- **The test** (crates/nf-view/tests/tase_scream_live.rs, opt-in `NF_ISO`, level 07000005):
  a prop-0x32, dtype≠2 guard within 20 u of Bond's start (the game's own gates narrow the setup
  [M 0x162d0c-0x162d40]); the held tase fells him [M DroneFunc_HandleImpact 0x1469b8 case 7]; the
  ENTER sets the once bit [M 0x162b98 case-1 tail]. Sister: release — the 30-tick fall wait
  expires into STUNNED 0x4b [M 0x162b98 case 0xc; the held beam re-arms +30 every tick via case 7,
  so the wait only runs released] — then hold again: STUNNED's case 7 re-falls him [M 0x162eb0
  case 7 → 0x4a], the TASER ENTER re-runs, and the once bit survives (no reset, no second queue —
  the hurt-scream share [M 0x145e28:11-13/32 + 0x162b98 both gate on +0x4F8 & 8]).
- **Neighbours green**: `fix_stunner_press_live` 3/3 (the press-fire pin, the fall/stay-down pin,
  the P2K-is-not-a-tase sister).
- **Full suite** (`tools/test/suite.py`, NF_ISO set) on the final tree: see the total line below
  (patched in before the finish step).

## What's left / open questions

- **The once-bit's end-to-end alert effect has no listener on 07000005's spawn area**: the three
  prop-0x32 guards sit 38 u (guard 2, (61.3, 27.1, 99.5)) and 50 u (guard 5, (150.2, 16.8, 100.8))
  from the victim — outside the 20 u broadcast radius [M Drone_SM_BroadcastMsg's sweep], so no
  other guard hears the scream in a spawn-area test. The receiver path itself (msg 0x12 → prop 0x27
  → `DroneVision_ShoutFromOtherDrone`/`enemy_alerts` → HEARD_NOISE) is the pre-existing ported
  route the punch scream already exercises. A level where a second guard stands within 20 u of a
  tase-able, prop-0x32 victim would pin the alert hand-off end-to-end.
- **A full recovery + second-tase cycle** (walk away past the AnyoneNear(2.0) band, wait the 1200-
  tick STUNNED timer, recover, re-tase) could not be driven in this sandbox: every exit route from
  the spawn courtyard either crosses a cut-scene trigger — script 0x06000952's stream-4 event 5
  (`Drone_EnableAll(0)` [M Script_EventHandler 0x1d94c0 case 5]) freezes every guard's timers —
  or leaves the victim within the keep-down band. The probe evidence: with the freeze active the
  STUNNED timer never fires (timer_c frozen at its armed value across 2300 ticks); without it the
  re-arm fires on schedule. The shipped sister pins the same once-gate through the re-fall cycle,
  which needs no movement.
- The dtype-2 exclusion (the third gate) has no dtype-2 guard near 07000005's spawn to pin against
  disc data; the gate is [M 0x162d34-0x162d3c] and marked in the code comment.

## Live checks for the main agent

1. **PCSX2, any level with a tase-able guard pair** (07000004's offices?): tase one guard of a
   standing pair with the Stunner and watch the SECOND guard — expected [M 0x162b98 case-1 tail +
   the msg-0x12 receivers 0x177c18:33-51]: he investigates the victim's position (the alert walk/
   run to the scream source). gdbwatch `switch_channels`/the receiver's +0x4F8 during the beat
   would confirm the broadcast reaches him.
2. **The scream property's per-type spread** [D the dronetype tables]: which dtypes carry prop
   0x32 — the port reads the same tables (`PropertyTables::get`), but a live spot-check of one
   screaming and one silent type would confirm the table parse.

## Usage

not visible to me — this session's environment shows no token, credit, context-window or cost
figures to copy (the agent's tool surface exposes none).
