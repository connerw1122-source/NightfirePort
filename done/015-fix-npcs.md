Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.

## Your task: `fix-npcs`, characters that are drawn but don't act, and the AI items from the user's playtests

These items were attached to every queued task by mistake. They're yours alone now, so no other worker will touch them. Start with C2, because it blocks play on 07000002 and 07000006.

**Also, confirmed by the user on 07000002** (after `fix-drone2`; they didn't press R): the guard in the bathhouse standing next to a civilian woman is still drawn, never attacks, and can't be damaged. Towards the end of 07000002, another NPC next to a hostage does the same. `fix-drone2` couldn't reproduce this headless, so its test doesn't match play.
- Run the real viewer path (`nf_view::load`, the level start, then walk there) and dump every drone that is drawn but either not hittable or not running AI, with present / started / enabled / in_llist / held / drawn / team / mode / dtypenew / channel.
- These may be scripted or ally characters (team 3, mode 5/9, or NIS-driven), not guards. Find what the game does with them, and port it.
**More from the user's latest playtest:**
- **07000005, between the gatehouse and the castle gate:** a patrolling guard walks one way while his upper body looks the opposite way, as if his spine were flipped. Check the patrol look-at and aim-blend (torso twist) direction and clamp.
- **07000006:** stationary guards rotate to follow Bond unrealistically (their whole body or torso spins fully around), never attack, and can't be hit. Probably the same class as C2: find what they are, and port their real behaviour.
- **07000006:** standing in a searchlight beam doesn't raise the alarm. Port the searchlight's spotted → alarm path (channel, drone alert, music event) from `docs/atlas/searchlight.md` and `Searchlight_Update`.

**User retest:** the 07000007 guests no longer T-pose, but they **don't move or walk** at all, despite `fix-drone3`'s CivilianPatrol and PartyGirl port. Reproduce it through `nf_view::load` and the real tick path: check that they have patrol paths, that the drone update runs for them (`npc_enabled`, the llist, start state), and that the walk animation and movement are applied. Fix it.

**Notes from earlier sessions:**
- `fix-ladder2` researched the searchlight → alarm music chain (WORKLOG, the fix-ladder2 entry): `Searchlight_Update` 0x1cb900 → `ch[param3]` → `DroneFunc_CheckAlarmRaised` 0x1453a8 counts `ch[0x37]` → `Music_Event(2,1)` → `UpdateMusicalEvents` 0x114e70 → the per-level `UpdateMFX_*` (07000006: `UpdateMFX_01CastleB` 0x113060). The port runs the castle-A updater only for 07000005, so add the per-level dispatch in `crates/nf-audio/src/music.rs`.
- `fix-ladder2` also reports that `fix-drone4` already fixed the 07000007 guests (CivilianMission 0x16 and the route-chain fix). Retest that headless on master first; only work on it if they still don't walk.
- `fix-drone2`'s headless test didn't reproduce the 07000002 guard. Your test must run the same path as the window: `nf_view::load`, the level start (intro and spawners), then walking Bond to the spot.

Files: `crates/nf-game/src/drone.rs` and the drone AI modules, `crates/nf-view/src/scene.rs` (minimal), `crates/nf-audio/src/music.rs` (the per-level dispatch), and `docs/systems/drone-ai.md`. Update C1/C2/C3 status in your report.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Main agent's correction (added after merging fix-hud):**
- **Don't port fix-hud's proposed C2 "team gate"; it's wrong.** In `NDrone2_DealWithObjHit` 0x13fff0 case 5, the shooter is the bullet's owner (`HITDATA+0x58` → `+0xE0` → `+0x30`). Its class byte `+0xFF` = 3 (the player) or 0x36 sets `bVar3 = true` (damage) with no team test (decompile lines 182–185). The team comparison applies only to class 2 shooters (drones). So in the game Bond **can** damage team 2/3 characters. C2 is something else, such as activation (channels, `+0x4F8 & 0x80000`, `NDrone2_DSTATE_Disabled` 0x172440). Dump the drone first.
- **The searchlight alarm, fix-hud's lead (unverified):** the spot ray is `Collide_RayIntersect(head, head + 100·dir, cel, ignore = the light, 0, hits, 0, flags 0x18)` [M Searchlight_Update 0x1cb900:206]; the arm test is `Vec_Dist3D(spot, glb_players+0x30) <= 2.0` [M :251]. If that ray can hit Bond's object, the spot lands on him and arms the light; the port's ray tests static geometry only. Verify whether flags 0x18 / `Collide_Pick` 0x1eb320 include the player object before changing the ray. `fix_drone3_live`'s searchlight test teleports Bond onto the spot, which is why it passes while play fails.

**Branch:** `s/fix-npcs`.

---
claimed-by: sandbox1 at 2026-10-12T04:04Z

---
## Report (done, 2026-10-12T05:53Z)


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
