# fix-npcs — the C2 "drawn but never attacks / can't be damaged" class, the searchlight alarm path, the NPC retests

Branch: **s/fix-npcs** (from origin/master 7e75c58). Tests: `crates/nf-view/tests/fix_npcs_live.rs` + the music unit
test in `crates/nf-game/src/music.rs`. Docs updated: `docs/systems/drone-ai.md`, `docs/atlas/searchlight.md`,
`docs/WORKLOG.md` (the fix-npcs entry).

## What's done

**C2's real shape: the ch-waiter is an invisible statue in the game, and the port's remaining bug was the NIS window.**
The bathhouse/end-of-level "drawn frozen guard" is a drone in `NDrone2_DSTATE_WaitSwitch`. In the game its ENTER calls
`NDrone2_Enable(0)` [M 0x15b960:15 → 0x149dd0:8-15: `obj+0xF0 |= 0x10|0x20` — View_AddObjects 0x1e62d8:33 drops it
from the draw, Collide_PickObj 0x1eae80:26-30 from the pick — and `+0x4F8 &= ~0x1801A0`: no anim, no AI]; it wakes
only when its live switch channel fires [M 0x15b960:21-37]. The earlier fix-render4 WORKLOG claim ("in the game,
WaitSwitch stops vision/AI engagement but the drone stays at its post with its idle pose and takes damage") is wrong —
I verified the decompile directly and corrected the record. On current master the port already hid the post-intro
waiters (verified headless on 07000002 at tick 3000: state 1, enabled false, renderer-hidden; the civilian woman
enabled and drawn — asserted in the new test).

**What was actually broken and is now fixed — every cut-scene left the drones visible and damageable.** The port's
`DroneEnable` script event flipped only the global gate, so during ANY NIS every drone stayed per-drone `enabled`:
the renderer drew them frozen, and a bullet in flight could still hit them. The game's `Drone_EnableAll(0)` walks the
whole list disabling EVERY drone [M 0x1383e8:13-23], `Drone_CollisionHandler` returns while `NPCGlobals == 0` — no
damage during a cut-scene [M 0x140968:62] — and `Script_Free`'s re-enable skips state-1 WaitSwitch drones
[M 0x1da880:41-43, 0x1383e8:14]. The port now mirrors all three. Fail-before: at 07000002 tick 120 (mid-intro) every
present drone was `enabled=true` (drawn) and a fired bullet damaged the nearest; post-fix all are disabled and the
bullet passes through. After the intro the waiters sit in WaitSwitch disabled and the civilian stays visible.

**The searchlight's spot ray picks the player [M] — ported.** The game's ray runs with pick flags 0x18, and the
class-3 (player) exclusion is mask 0x400 which 0x18 does not set [M Collide_PickObj 0x1eae80:26-40]: when Bond stands
in the beam the ray's first hit is his world sphere (`obj+0x80`, radius `obj+0x8C` = 1.0576 [L]), so the spot lands ON
him and the arm test `Vec_Dist3D(spot, player) ≤ 2.0` [M 0x1cb900:251] fires. The port's ray tested static geometry
only. The fix adds the ray-vs-sphere test over the static segment (the closer hit wins; the spot orients by that
normal). Fail-before shown with a pinned synthetic head over the real 07000006 world: pre-fix the spot never draws
and the light never arms; post-fix it lands on Bond's sphere (dist 1.31) and the light arms and raises ch 55.
Honest note: the first cut of the fix had a real bug (the ray's `spherical(100,…)` direction is 100 long, my
quadratic assumed unit) that sent the spot to −10000 — **the existing fix_drone3_live regression test caught it**, I
fixed the quadratic, and the incident is recorded in the WORKLOG.

**The 07000006 alarm music — the missing per-level dispatch, ported.** Everything up to `Music_Event(2,1)` was
already ported; `UpdateMusicalEvents` ran the per-level updater for 07000005 only. `UpdateMFX_01CastleB` [M 0x113060]
is `under_attack ? PickRandomCombat(0) : PickRandomStealth()` — ported as the 0x07000006 arm. Unit test: streaming
`Music_Event(2,1)` per frame lands the combat jump on the 12th tick, targeting the combat set; sister: a quiet level
picks nothing. CastleB's NIS-trigger half (the 0x6000071 marker jump) needs MapMusic fields the port doesn't parse
yet — left to the audio session, documented.

**The 07000006 stationary guards and the 07000007 guests — verified game-accurate on master, locked by tests.**
Post-intro dump: 20 of 25 guards are ch-waiters (channels 11/12/13/15/16/18/19/55) — invisible statues by design;
d02/d15/d16/d14 are enabled idle guards. A same-floor idle guard sees Bond within 70 ticks, alerts, enters combat and
wounds him (the IDLE enter sets the sight bit [M 0x15bf18:30]; Idle UPDATE's `EnemyLookForOpponent` returns 0x56
[M 0x177ed0] — already ported by fix-drone3); locked as a sister test. The guests: fix-drone4's walk test still
passes with the freeze mirror in; the mission walk is a one-shot route (arrived → Civilian), PartyGirls stand
(game-accurate [M 0x160140 case 3]), waiter guests stay disabled — locked.

**The 07000005 patrol torso item — verified, no code.** fix-drone3's two-sided ±0.9 clamp is in and green; a re-read
of `AnimObjectAimAt` 0x1bfe58 confirms the port's sign convention is self-consistent (the game's aim angle comes from
`acos` + a scalar-triple-product sign, not a yaw difference — there is no absolute left/right datum to copy). The
"spine flipped" report matches the pre-fix-drone3 build. Remaining known gap: head tracking (bone 5, max dist 3.0,
rate ~0.13 [M 0x1472c8]) — cosmetic, left to a render-side session.

## How it was verified

- `fix_npcs_live` (4 tests, NF_ISO): the freeze mirror + bullet immunity (fail-before shown), the spot-ray player
  pick (fail-before shown), the idle-guard fight, the guest states.
- `Music::the_07000006_alarm_demands_combat_music` (unit, always on).
- Regression: fix_drone3_live (5, incl. the searchlight chase that caught my quadratic bug), fix_drone4_live (2),
  level_reload_live (9 — the 07000001 plate test now waits out the intro freeze, which is the game's own order),
  fix_truck_live (2).
- Full suite on the slim ISO (`NF_ISO=… cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio
  -p nf-view -- --test-threads=1`): everything green except the 3 allowed slim-ISO `disc_golden` failures
  (bigf_header_fields / bigf_tables / directory_tree).

## What's left

- Head tracking for idle/patrolling guards (the visible difference between the port and the game when a guard
  watches Bond) — cosmetic, needs the render-side pose path.
- CastleB's NIS-trigger half and the other levels' `UpdateMFX_*` updaters — needs the MapMusic marker fields in
  nf-formats (the audio session's file).
- The end-to-end ch-21 plate wake on 07000002 remains unverified on foot (fix-render4's Bond jammed at the plates);
  the freeze mirror does not change that path, and the wake itself is pinned by fix-drone2's channel test.
- The 07000007 BondTalk (guests chatting when unarmed Bond is near) — noted by fix-drone4, still unported.
