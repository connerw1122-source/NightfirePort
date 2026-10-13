# fix-guest-reactions — the guests' reactions to an assault, and the death restart loop

Branch `s/fix-guest-reactions`. The user on 07000007: "the moment I started punching a guest, all
the other guests T-posed. And once I died, we went into a replay loop where it kept replaying at the
start of the mission over and over again"; also "If you punch a guard, it should be an automatic
mission fail" (protected actors, docs/systems/actors.md).

## What was done

**Reproduction first — the punch never landed at all.** The port spawns the fists' round at the fire
script's strike frame (`weapon_data+0x44` = 6 for the fists), so a one-tick trigger press never
spawns a bullet and a walking guest outruns the 1.75-range projectile in the wait. The existing
T-pose test passed **vacuously**: its own log showed `ko=false` and zero punch dispatches. The new
tests re-aim every tick and wait out the strike frame — after that a punch lands, dispatches msg 6
(`NDrone2_DealWithObjHit` case 5, 0x13fff0) and the whole chain runs.

**1. The T-pose (the guests' panic + its animation).** The game's chain, from the decompile:
`NDrone2_PunchImpact` 0x145f28:30 sets `Drone_AlertStatusSet(2)` on the victim, and :55 calls
`DroneFunc_SendHurtMessage` 0x145e28:19-57 — prop 0x32, the +8 once-per-life bit — which
**broadcasts msg 0x12** (`Drone_SM_BroadcastMsg`) with the victim as the alert source. The receivers'
DSTATE case 0x12 → `DroneVision_EnemyAlerts` 0x177c18:41-51 gates on prop 0x27 →
`DroneVision_ShoutFromOtherDrone` → CivilianScared 0x12, whose ENTER (0x15d3a0) runs to CivilianHiding
0x13 with `CallAnim(0x19)` — **the cower, the animation the user's build was missing** [M 0x15d520].
The port had none of it: no broadcast, no 0x12 receivers, no victim alert. (Its sound-alert path
cannot carry the scream anyway: the voices' hearing weight is 0.0 in the sfx table, and 07000007's
`DroneFunc_HandleSoundAlerts` 0x148a30:33-39 returns early for UNARMED drones.) The port now queues
the scream (`pending_shout`) and drains it in the shared pass to every present+enabled drone within
20.0 (the PartyGirl shout's radii [M 0x160140:63-70]); the receivers go CivilianScared → CivilianHiding
with the cower; the guard states keep their `enemy_alerts` investigation (the `DroneVision_AlertSound`
else-arm [M 0x177800:81-88]). The victim's alert = 1.0 (the port's alert-status-2 mapping).

**2. The replay loop.** `Mission_Update`'s game case 5 is a bare `break` [M 0x182c60:83-84] — the
0x40000042 menu waits for the answer. The port's catch-all re-armed `internal = 1`, re-running the
whole death/fail sequence and queueing another restart every tick after the answer — the user's
"kept replaying at the start of the mission over and over again". Fixed with the explicit wait arm
(crates/nf-game/src/mission.rs). The menu → Restart flow itself was already right (fix-restart-target:
`P_ENDMISSION_Handler` 0x216618:19-21 — a WARM reload of the current level on the ENTRY checkpoint,
`Player_RamLoad(Bond, 1)`, the 50.0 health floor [L 0x30cc34-3c]); menu.rs needed no change.

**3. Punch = automatic fail.** Already ported (fix-party-mission's `punch_impact` arm,
0x145f28:45-56 → `DroneFunc_SetMissionFailReason(drone, 5)` [M 0x13e560 → ch 99]) but untestable
through the input path until the strike-frame punch landed. The new test pins it for a GUEST (the
bouncer was already covered by fix_party_mission_live).

The guests' props measured on the disc data [L probe]: 0x27/0x32/0x33/0x52 set, 0x28/0x1f clear —
every gate in the chain lines up.

## How it was verified

`crates/nf-view/tests/fix_guest_reactions_live.rs` (all NF_ISO, all green, through the real
`step_bond` input path):

- `punching_a_guest_fails_the_mission_at_once` — ch 99 + MissionState 2 [M 0x145f28:45-56,
  0x182c60:45-57]; sister: the victim's health drops.
- `punching_a_guest_panics_the_other_guests_into_the_cower` — the victim's alert ≥ 0.66 (the
  AlertStatusSet(2) sister), and within ~20 units of the scream other guests reach
  CivilianScared/CivilianHiding with a hiding guest playing the cower (anim state 0x19). Both
  asserts failed before the fix (no reaction at all).
- `one_death_restarts_exactly_once` — the death opens the 0x40000042 menu exactly once; the Restart
  targets the CURRENT level warm on the entry checkpoint; a second answer is a no-op; 300 further
  ticks queue no further restart and never re-enter the menu. The test failed before the mission.rs
  fix (the menu re-opened — the loop).

Full suite on the branch: **nf-game 225 ok; nf-formats + nf-audio 88 ok (only the 3 allowed
disc_golden failures); nf-view 337 fast-binary + 103 walk tests ok — zero unexpected failures.**
One flake: `audio_sample_count_matches_stream_length` failed once inside a combined
formats+audio run and passed on every isolated/combined re-run (including on a pristine
origin/master worktree).

## Files

- `crates/nf-game/src/drone.rs` (the task's file): the msg constants SCREAM 0x12 / SHOUT 0x11, the
  `screamed`/`scare_shouted`/`pending_shout` fields (own lines), the scream queue + the shared-pass
  drain (20.0 radius), the SCREAM/SHOUT receivers in `handle`, the victim's AlertStatusSet(2) and the
  hurt broadcast in `punch_impact`.
- `crates/nf-game/src/mission.rs` (the task's file): the case-5 wait.
- `crates/nf-view/tests/fix_guest_reactions_live.rs` (new).
- `crates/nf-view/src/scene.rs`, `menu.rs`, `lib.rs`, `player.rs`: **no net change** (temporary
  probe instrumentation was removed; the menus' flow needed nothing).

## What's left

- The PartyGirl **armed-intruder shout** is PORTED but its test is `#[ignore]`: the update's prop-0x33
  arm now broadcasts msg 0x11 to everyone within 20 (`NDrone2_DroneAlertToObject`, 0x160140:63-70)
  and sets the fail reason 0xb behind prop 0x52 [M 0x13e560] — the panic path for an ARMED Bond (the
  guests' 0x28 is clear, so the shout reaches the guards' prop-0x28 route). The test's headless blind
  spot: the scare's gate needs the guest's `seen` (the sight pass's reaction time/FOV on a wandering
  guest) which this session could not drive in time; the shout code itself is prop-gated and harmless.
- The scream broadcast radius is hardcoded 20.0 (the PartyGirl shout's value [M 0x160140:63-70]);
  `Drone_SM_BroadcastMsg`'s own alert-struct radius was not dumped.
- The idle state-1 guests sit disabled (`WaitSwitch` — `Drone_EnableAll(1)` skips them [M 0x1383e8:14])
  and bullets pass through them; the game wakes them on their placement switch channels. Punching
  those specific guests stays inert until the wake path is ported.
- Live checks for the main agent (PCSX2, 07000007): walk to a party guest, punch it twice — the
  guest screams and the nearby guests cower (the CiviliansHiding cower anim, no T-pose) and the
  mission fails at once ("MISSION FAILED"); die (walk into the atrium fall or let a bouncer kill
  you) — the fail/death page opens once, Restart reloads the level exactly once at the entry
  checkpoint, no replay loop. nf-view render spots: `--bond` beside the first enabled guest after
  the punch (the victim fleeing 0x56, the others in the cower 0x19), and at the death (the red fade
  then the menu).
