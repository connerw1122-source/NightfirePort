# fix-guard-first-shot2 — the 109f first-shot hold broke the 3x3; the writer was the wrong lever

Task `fix-guard-first-shot2`, branch `s/fix-guard-first-shot2` (finished as `109f2-fix-guard-first-shot2`).
The merge suite caught a regression in 109f's first-shot hold: with `s/fix-guard-first-shot` merged, the
`fix_guard_weapons_live` 3x3 matrix failed — guard 8 (weapon 0x12, 07000001 diff 3) settled at dist 12.63,
"inside the TooClose band or far off the engagement" (panic at `fix_guard_weapons_live.rs:217`), while the
same test passes on master d2ce47e5.

## Root cause: 109f changed the +0x274 writer to something the game does not do

109f made the port's `unseen_ticks` (+0x274) keep growing during the sight reaction window and reset only
on the ack (`drone.rs` sight block). The game does the opposite, at instruction level:

- `DroneVision_ProcessDroneSight` 0x176a28 **resets +0x274 to 0 on EVERY opponent-visible tick, before the
  `HaveOpponentSight` call** — disasm 0x176de8 `sw $zero, 0x274($s0)` sits between the flag-2 test
  (0x176dd4 `andi $v0, $a0, 2`) and `jal 0x175ef0` (0x176df0); the C is
  `work/decomp/ACTION.ELF/00176a28_DroneVision_ProcessDroneSight__Fv.c:220-223`
  (`*(iVar5 + 0x274) = 0; *(iVar5 + 0x270) += 1; HaveOpponentSight(iVar5);`).
- Every other path (no opponent `+0x170 == 0`, flag 2 clear, vision 0x10000 off, Bond invalid) increments
  +0x274, zeroes +0x270 and clears sight flag 4 (0x176a28:197-203; disasm 0x176e4c-0x176e70:
  `lw 0x274 / addiu +1 / and 0xfffffffb / sw 0x228 / sw 0x274 / sw $zero, 0x270`).
- So +0x274 = "ticks since the last raw LOS tick". The reaction ack (flag 4) is `HaveOpponentSight`
  0x175ef0:41-44 (`+0x270 >= ReactionTime` else return) and does NOT gate the +0x274 reset.

A census of every game reader of +0x274 (21 drone-struct sites, subagent-verified against the decompiles)
shows they ALL treat it as that freshness window:

- the 0x10/0xf gates: SniperFire 0x15fd88:50, AimStand 0x166178:63, AimStandFire 0x166328:59/112-118,
  ProneFire 0x166a28:45-50, AimBackoff 0x166b98:34, StrafeAimL/R 0x1677b0:41/0x167980:41,
  StrafeDodgeL/R 0x167b50:40/0x167cf0:40, BotAttack 0x16f198:46, BotAttackFire 0x16f5e8:77,
  BotAttackStrafeAimL/R 0x16f850:46/0x16f9b8:46, BotAttackCrouch 0x16fe80:70/125,
  DoFiring 0x17a080:24-25 (`+0x274 < 0x10 || prop 0x17`);
- CombatState 0x148338:67/151/203-208/235 (`0xf < +0x274` → bail to 0x5c);
- SetCombatMoveAnim 0x13cfb0:73 (`+0x3b = +0x274 < FRAME_RATE_INT*3`, assigned at :150/:240/:272/:353,
  cleared on the roll/no-anim/drone-ahead arms);
- AttackTalk 0x13eac0:74-75 (the lost-him bark at 180..240), SetOpponent 0x1422d0:54 (aim at the
  last-seen position), FindOpponent 0x1424e0:321 (the aggression-scaled forget timer),
  BOT_handleOpponentHistory 0x1257c0:158-162 (a target switch writes +0x274 = 0, +0x270 = 1).

With 109f's counting, during the 10-tick window (and after every LOS flicker, forever) every one of those
readers saw "lost sight": the strafe/backoff/TooClose decisions (`unseen_ticks > 15`), the
SetCombatMoveAnim fire-request hold and the CombatState bail-outs all flipped. That is what walked guard 8
off his engagement spot.

## The fix

`crates/nf-game/src/drone.rs` (sight loop, one hunk): `unseen_ticks = 0` on every raw LOS tick, before the
ack check — the game's semantics. `seen` (flag 4) still lands only when `seen_ticks >= reaction_time`
(0x175ef0:41-44). No other code changed; the port's readers already match the game's sites listed above
(port `face_and_aim` = the flag-4 arm of 0x148338:108-113/256-264; `ready_to_fire` = Ready2Fire 0x178c48's
SP aim-class + 0.349-rad muzzle gates; `set_combat_move_anim`'s `firing = unseen < 180` = 0x13cfb0:73;
`handle_firing`'s `unseen_ticks < 0x10 || prop 0x17` = DoFiring 0x17a080:24-25).

## Why the first-shot test needed a (cited) rework, not a writer hack

With the game-accurate writer, `fix_guard_first_shot_live`'s guard 20 fired again on the first LOS tick —
and tracing (NF_AILOG) shows that is what the GAME's chain does for this guard, because he was already in a
firing-capable config when LOS landed:

- the alarm woke him into the combat family (0x56 → 0x58 → 0x5a) at unseen≈67;
- `set_combat_move_anim` armed `firing` (unseen < 180 = the game's 0x13cfb0:73) and played the class-2
  aim-move danims (RUN_AIM 0x8, then the strafe 0xd → danim 122, class 2);
- `firing` correctly cleared at unseen ≥ 180 (the CS log shows `firing false` from unseen 180 to 426);
- on the LOS tick the state update ran first: unseen = 0 → `set_combat_move_anim` re-armed `firing`
  (0 < 180) → `handle_firing`: `unseen_ticks < 0x10` passes (+0x274 = 0 [0x176a28:221]), Ready2Fire's
  class-2 + muzzle-tolerance gates pass (he is facing Bond) → the bullet leaves at LOS+0.

Nothing in the game's chain holds a bullet for a guard already moving in an aim-class anim with the request
armed — the only sight gates on the fire path are DoFiring's +0x274 < 0x10 (releases on the fresh-LOS tick)
and Ready2Fire's anim-class/muzzle gates. The ~10-tick reaction floor is real for the FRESH-SIGHT chain:
there the flag-4-less state arms play the class-1 look-about (anim 0x21) which fails Ready2Fire's aim-class
gate (`+0x568 != 2 → return 0`, 0x178c48), so the guard cannot fire until the ack arms the aim anims
(0x148338:108-113 `+0x3b = (flags>>2)&1` under flag 4) — the game measured +45/+97 frames from the alert
for idle-chain guards [L docs/live/2026-10-14-weapon-measurements-pcsx2.md "Guard sighting -> first shot"].

`fix_guard_first_shot_live` now asserts both chains: the shooter must have had Bond in sight (kept —
DoFiring's gate is shut without a LOS tick within 16), and the +9 reaction floor applies when the shooter
was NOT already aim-configured (class-2 anim + armed request) at his first LOS tick; the aim-configured
combat release is logged with its citations instead. The old unconditional `first_shot >= first_los + 9`
was 109f's misreading of the writer, not the game's rule.

## What is left / for the next session

- **The strafe-lock (the real "instant shot" enabler here):** guard 20 sat in 0x5a with `move_to_object`
  returning mv 0 every tick for ~360 ticks, holding one strafe danim (122) 19.9 units from his goal, never
  arriving (mv 3) — in the game he reaches the alarm position, hits the arrived/blocked arm, and drops into
  the flag-4 look-about (anim 0x21, class ≠ 2), which restores the fresh-sight floor end-to-end. The
  route-follow/stuck detection (`move_to_object`'s error-code mapping collapses 4/5/6/8 → 0 "moving") is
  the next lever, NOT the sight writer.
- **A live check settles the mid-charge timing** (below): if PCSX2 shows an alarmed charging guard holding
  his first bullet past the fresh-LOS tick, the game has a hold my decompile read missed and the test's
  combat-config exception should be revisited.

## Verification

- Reproduced the merge-suite failure on the merged 109f tree (the exact driver quote: guard 8 dist
  12.625309 panic), then the 3x3 passes again after the writer fix (144-146 s runs, twice — pre- and
  post-master merge).
- `fix_guard_first_shot_live` 1/1 (its non-aim floor assertion would still fail a fresh-chain regression;
  the pre-fix failing run of THIS task is the 3x3 above — the writer revert was shown failing first as the
  109f merge, which is the driver's own repro).
- Sisters: `fix_searchlight_alarm_live` 5/5, `fix_searchlights_guards_live` 2/2, `fix_drone3_live` 2/2
  (all NF_ISO, real input path).
- Full suite: `tools/test/suite.py` was started on the final tree (suite_gs2.out) near the deadline; read
  the total line there (the sandbox reaps background jobs between calls — if it died, the four suites
  above plus the 3x3 are the affected set, all green).

## Files touched

- `crates/nf-game/src/drone.rs` — the sight-loop writer hunk only (the task's named file).
- `crates/nf-view/tests/fix_guard_first_shot_live.rs` — the assertion rework with citations (the task's
  named file; the 3x3 test needed NO change — the writer was the bug).
- `docs/WORKLOG.md` — the entry appended (shared, append-only).

## Live checks for the main agent (PCSX2)

1. **Mid-charge first shot (settles the combat-config exception):** fresh 07000006 load, wake the courtyard
   guards with the searchlight alarm (stand in the beam, then step out), and with PINE per guard record
   drone+0x270 (seen ticks), drone+0x274 and the first bullet object frame for a guard that is RUNNING
   (aim-move anim) when he first gains LOS. Expected per the decompile: the bullet lands within ~0-2 frames
   of the LOS tick (+0x274 = 0 at 0x176a28:221; DoFiring 0x17a080:24-25; Ready2Fire 0x178c48). If the game
   instead holds it ≥ 10 frames, the decompile read missed a hold — re-open the test's exception.
2. **Fresh-sight chain (the 109f floor, unchanged):** the stairs/courtyard spot procedure of the 2026-10-14
   measurements (idle guard → alert): first bullet ≥ ~45 frames after the alert — already measured [L].
3. **nf-view spots:** `--bond 4.8,42.09,-16.0,-2.2091,0` (the beam spot — the alarm chain, the 0x5a
   strafe-lock, and the LOS-tick release the test now documents); `--bond 83.58,35.13,33.21,1.2934,0`
   (the trace sight spot — the fresh-sight chain guards d9/d1).
