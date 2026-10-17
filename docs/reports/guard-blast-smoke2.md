# guard-blast-smoke2 — finish guard-blast-smoke (the merge hold)

Task 761, branch `s/guard-blast-smoke2`. `s/guard-blast-smoke` (727) was held at the 2026-10-16
merge: on master it failed `nf-game guard_drops_flashbang_live
::frag_at_the_same_spot_kills_the_trio` (a frag 1.9 u from placements 3/4/302 no longer killed
them) and `nf-view fix_actors_live
::killing_a_protected_guest_fails_the_mission_but_scaring_him_does_not` (the guest died but ch
0x60 never raised, the assert at line 80). Both now pass on `s/guard-blast-smoke2` = origin/master
d628f01f (the `briefing_plays_live.rs` check passed) merged with origin/s/guard-blast-smoke
13ca8758.

## The ground truth the hold turned on (the msg-9 survey)

The held branch routed explosion damage through msg 9 (`Drone_ExplosiveHit` 0x13aab8 →
`SendMsgSelf(9, impact, 0, 0)`) and dropped msg 9 for any state its `impact_args` table did not
list, reusing the msg-8 (bullet) pairs as the msg-9 gate. I surveyed all 250 `NDrone2_StateFuncs`
entries (the table at ELF 0x29b300 [L], read from `work/elf/ACTION.ELF`) and each DSTATE's msg
switch. The chain that decides everything:

- `Drone_ExplosiveHit` 0x13aab8 applies **no damage itself** — the only work is
  `drone+0x2b4 = impact+0x20` (the attacker) plus `SendMsgSelf(9, …)` [M].
- `Drone_SM_RouteMsgDCV` 0x171df8 calls the state fn; on a 0 return it re-calls the SAME fn
  pointer (`SMI+0x28` = `NDrone2_ProcessStateMachine` 0x175778, set per drone at init
  [M 0x175838: `+0x130`]) with the **state arg 0** [M-dis 0x171ec8-0x171ef0].
- State 0's entry is `NDrone2_DSTATE_Global` 0x15b830 [L the table], whose switch handles only
  msgs 0/1/10/0x1d — **msg 9 is dropped on both floors**.

So a state takes frag damage iff its own DSTATE has a case 9. The survey's case-9 bodies for the
states in play: Idle 0x04/Alert 0x05/Patrol 0x07/AlertToPosition 0x62/SearchArea 0x65/
HeardNoiseAlert 0xa5/RunForCover 0x8c/CivilianPatrol 0x14/CivilianGuard 0x30 →
`HandleImpact(msg, 0x56, 0)`; Civilian 0x11/PartyGirl 0x2f/TruckDriverIdle 0x34 →
`HandleImpact(msg, 0x12, …)`; the stun/KO families → `SetState(0x44)` (including Knocked_Out
0x43 — an `if (msg != 9) …` chain, not a switch case [M 0x172b48]); Surrender_Anim 0x3f and
Surrendered 0x40 → `SetState(0x44)` [M 0x1626a0/0x162950]. `WaitSwitch` 0x15b960 handles only
msgs 0/1/3/0xe — **no case 8 or 9**.

## Fix 1 — the frag-trio test (the test was wrong; fixed with the evidence)

The as-loaded trio sits in WaitSwitch after `DSTATE_Global`'s ENTER `+0x5a2` pick, and
`switch_channels[0]` is 0 [L the EE snapshot at 0x26fc90, the ELF statics agree] — the game's own
yard guards only leave WaitSwitch through the post-NIS `EnableAll` msg 0xe broadcast
[M 0x1383e8:27] or their activation channel. A frag at the as-loaded trio therefore kills nobody
in the game: msg 9 drops at WaitSwitch and again at the GLOBAL fallback, and `ExplosiveHit`
applies no damage outside msg 9. The held branch's GLOBAL damage arm (impact_args' `GLOBAL`
entry) kills them anyway — a port deviation for the pre-init frame and the unmodeled states, now
documented in the test comment.

Per the task rule the TEST was wrong against the decompile, so:

- `frag_at_the_same_spot_kills_the_trio` now primes the trio into Idle (the pod-pick test's
  priming pattern) — Idle's case 9 forwards to `HandleImpact(0x56, 0)` [M 0x15bf18 case 8/9
  fall-through] — and accepts the death selector's own outputs: kind 0x0d/0x1b/0x1c/0x1d → 0x44,
  everything else → 0x45 DeathByExplosion [M 0x1465e4-0x14697c]. The drops assertions are
  unchanged (the dead guards drop their SG5s, one gun per dead guard; pl 303 stays in WaitSwitch
  and survives — the game's own geometry now matches the assertion).
- New sister `frag_at_the_waitswitch_trio_takes_nothing`: the same frag at the AS-LOADED trio
  must not kill, damage or stagger — health stays 10.0, no drops. This is the assertion that
  fails if the GLOBAL damage arm ever swallows the drop.

No measured test was loosened: the kill/score assertions keep their numbers; the state set grew
by the death selector's own 0x45 with the address.

## Fix 2 — the protected guest's ch-0x60 raise (a tick-ordering bug in the merge)

The guest (dtype 10, team 3, props 0x53/0x54) kills correctly through the msg-9 path: PartyGirl
0x2f's case 9 → `HandleImpact(0x12, …)` → `ExplosiveImpact` → health ≤ 0 → the death selector →
0x44 (his kind rides the port's documented 0x44 stand-in for the unported 0x92/0xBF kinds). But
the probe showed `death_pending` set while ch 0x60 stayed off: the raise is NOT the drone's own
death channel (the guest's `+0x13c` is 0) — it is `DroneFunc_OnInitDeath` 0x145bd0 →
`SetMissionFailReason` 0x13e560, whose reasons 1/3/6/9 write **`switch_channels[0x60] = 1`
inside the ENTER**, the same instant as the corpse [M 0x13e560 case 1/3].

The port defers that verdict to a per-drone drain inside the control pass — and `spawn_blast`
runs at the tick's tail, after every drone's drain has gone by, so a blast kill's verdict landed
a tick late (master's old direct-kill arm hid this by deferring the whole transition into the
victim's own next tick). The fix: the drain pass (`raise` + `fail_reason` → channels/label) now
also runs after `spawn_blast` at the tick's tail, so the game's same-instant write lands in the
same tick. `killing_a_protected_guest_fails_the_mission_but_scaring_him_does_not` is green with
its ch-0x60 raise [M 0x145bd0 reason 3], FailLabel 0x0400_0034 and the mission fail intact.

## Also

- `blast_smoke_live` updated to master's APIs (`cover::build(&Level)`, `Data.ai_points`) — the
  held branch predated them and did not compile on the merge.
- The merge itself: `drone.rs` kept BOTH master's door-guard handler and the held branch's
  blast-smoke handler (dispatch + fn bodies), and took the held branch's msg-9 blast arm; the
  WORKLOG kept both sides append-only.

## How it was verified

Foreground, before the suite: `guard_drops_flashbang_live` 5/5 (incl. the new sister),
`fix_actors_live` 4/4, `blast_smoke_live` 36/36, `astronaut_guards_live` 4/4, `ai_live` 4/4.

Full suite on the final tree (the fast runner `tools/test/suite.py`, 497 binaries; background
jobs die between tool calls in this sandbox, so it ran as 7 foreground `--only` chunks):

**2073 passed, 6 failed, 74 ignored.** The 6 failures are the allowed set, pre-existing on
master: 3 `disc_golden` (the slim ISO) + 3 no-GPU pylon probes (`probe_pylon_render`,
`fix_0c_pylon_live` ×2 — the documented inherited set in the merged task reports). No
FAILED/TIMEOUT beyond those; total wall time ≈ 38 min across the chunks.

## What's left / known deviations

- **The GLOBAL damage arm for msg 9 is a documented deviation, not removed.** The game drops
  msg 9 at state 0 and at every state without a case 9; the port's `impact_args` GLOBAL entry
  applies the damage through `explosive_impact` for the pre-init frame and the unmodeled states
  (the seated-target rationale). The new sister pins the drop where it is observable (WaitSwitch,
  a modeled state); state 0 lasts one frame at load in the game and is not directly pinnable
  without a save-state-free live boot. The sister fails if the arm ever swallows the WaitSwitch
  drop.
- **The flashbang's WaitSwitch stun is the same-shaped deviation.** The game's msg 0x18 chain
  (WaitSwitch drops → GLOBAL drops) cannot stun a WaitSwitch guard, but the port's `on_flash`
  stuns every state and `flashbang_stuns_three_guards_and_never_kills` (green, pre-existing)
  pins that behavior. Left untouched — changing it would break a test the decompile contradicts,
  which is this task's call to make only where the task points; flagged here for the driver.
- A full per-state msg-9 map (separate from the msg-8 `impact_args` pairs) would replace the
  reuse outright; the survey in this report is the starting point. The pairs spot-checked match
  (Idle/Alert/Patrol → (0x56, 0); Civilian/PartyGirl → (0x12, ·); the stun/KO/surrender families
  → their own arms).

## Live checks for the main agent

Nothing is blocking. Two observations would settle the flagged deviations if convenient:

1. FRESH boot into 07000005, walk to the yard, throw a stun grenade at the three pre-activation
   yard guards: if the real game stuns them (the port's behavior), the WaitSwitch msg-0x18 drop
   reading needs a second look (some other delivery path); if it does not, the port's on_flash
   needs the state gate. Address of interest: the msg-0x18 send in `Bullet_DoTrails` 0x12d890's
   flash arm → `Drone_MessageObjVicinity` 0x13a438 (read/write on `switch_channels` + the drones'
   +0xF4).
2. FRESH boot, frag at the same yard spot BEFORE the guards activate: expect no damage numbers,
   no deaths, no drops (the port now matches; a damage number would falsify the msg-9-drop chain
   and point at a delivery path the survey missed).

## Usage

not visible to me (no token/context figures are exposed in this environment)
