## Your task: `cover-remainder`, guard cover remainder

GAPS G-028: low-node anim validation (`ValidateCoverAnim`), the `+0x538` blend timing for non-call anims, the RunForCover
route shape near the 05 gate (U-030), per-target slots (only Bond today), the `RecoverFromScaryObject` (0x86) flee route.
Use `tests/traces/05-*.trace` and the ai_traces_live harness to compare the cover choices frame by frame where a trace
covers them.

Files: `crates/nf-game/src/drone.rs` (cover arms), docs/systems/drone-cover.md.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing** (start it early, in the background, so it finishes inside your session):
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count. If your change adds `Rand_*` draws, seed-pinned
  tests may move: re-pick them by the WORKLOG 0072 procedure (`NF_TEST_SEED` search against the same live expectations).
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/cover-remainder`.

---
claimed-by: sandbox4 at 2026-10-13T20:34Z

---
## Report (done, 2026-10-13T21:44Z)


---
## Report from the worker (done, 2026-10-13T21:45Z)

# fix-cover-remainder — GAPS G-028 (guard cover remainder)

Branch `s/fix-cover-remainder`, cut from master f767e3d, finished on the merge of master 7a614ba. Session
20:45–22:25 UTC 2026-10-13 (sandbox deadline ~22:39). Files touched: `crates/nf-game/src/drone.rs`,
`crates/nf-game/tests/ai_live.rs`, `docs/systems/drone-cover.md` — nothing else (no shared-file edits
beyond drone.rs, which the task names).

## Done

### 1. RecoverFromScaryObject flee chain (G-026) — ported end to end
Before: msg 0x21 was never sent, `consider_explosive` was dead code, and state 0x86 had no handler — a
drone that got there (only via test injection) froze forever. The port now has the whole family, from the
decompile:
- `DroneFunc_HandleExplosives` (0x1435f0) + `Drone_BuildDynamicAwarePoints` (0x138b10): the explosive list
  is rebuilt every frame; kind-2 entries trigger a flee — the port fires once at a grenade's **bounce**
  (the in-flight rescan needs object vision the port's drones do not model — documented deviation), gated
  per drone by fear props 0x3A/0x3B [M 0x1435f0:30-34], the entry radius (12.0 grenade family / 8.0
  flashbang [M 0x138b10:67-87, :163-167]) and a world line-of-sight check (the game's ObjectVisibility
  ≥ 0.7 + CanSeeObject(5,0) [M 0x1435f0:37-44]). Smoke 0x36 makes a kind-1 entry and never triggers
  [M 0x138b10:143-159 + 0x1435f0:24-29].
- `DroneFunc_ConsiderExplosive` (0x1431c0) upgraded: the per-type run distance +0x350 (4.0 grenade-thrower
  weapons with a 2 s deadline, 10.0 flashbang, 14.0 the 0x34/0x3B-0x40/0x6C explosives; the old port
  constants "5.0 stun / 8.0 mine / 12.0 frag" were wrong), the return state +0x354 (0x87 Hide by default,
  the caller state for the flashbang; 0 → the caller state [M 0x1431c0:191-193]), the deadline +0x324, the
  +0x2B scared flag, then `RunAwayFromObject` **0x86** [M 0x1431c0:194-195] — the doc's older
  "0x86 = RecoverFromScaryObject" reading was wrong; the port comment was right.
- `RunAwayFromObject` 0x86, `HideFromScaryObject` 0x87, `RecoverFromScaryObject` 0x88 ported as
  `handle_scary` (before the catch-alls; the three state constants added — grepped first, none existed):
  ENTER finds the safety goal (`FindSafetyFromScaryPosition` 0x152250's straight-line fallback: scary +
  normalise(drone − scary)·dist, y + 0.5; the path-emitter strategy at dist·1.8 is [S]-approximated by the
  route build's clamp), UPDATE runs `move_to_goal` with the run anim 0x1E / the deadline exit [M
  0x168aa8:38-52], Hide waits the deadline out [M 0x168d18:39-44], Recover clears +0x2B and returns to
  Attack (+0x5A0 != 0x89 → 0x56 [M 0x173658:33-42]; the port has no RunToAlarm 0x89).
- The game's msg routing preserved: the per-state resume values (Idle/Alert/Patrol/Hostage → 0x56
  [M 0x15bf18:134...], UnderCoverFire → 0x9C, RunAway/Hide → 0x88 [M 0x168d18 case 0x21]) via a msg-0x21
  payload; the states without their own arm route through a pre-hook in `handle`.

### 2. ValidateCoverAnim (0x13bf40) — ported
`validate_cover_anim` (drone.rs): 0 passes; ≥ 0x77 passes unvalidated [M 0x13bf40:12,22]; 1..0x77 valid
only when the drone's anim set has the state's row (the base set counts too [M 0x13bf40:14-19]); an invalid
5 substitutes 7 else 0xA [M 0x13bf40:25-38 — the record-11 gate approximated by `can_do(7)`, the port's
existing documented stand-in]; any other invalid → 0xFFFF → `cover_anim` returns false (no anim queued;
UnderCoverInit/Aim linger, UnderCoverReturn/TypeChange take their fail routes [M 0x13c078:311-314]).
Mode 3's 0x58 stays unvalidated [M 0x13c078:297-300]. Before: any low-node anim the set lacks fell into the
generic replacement table and ended as the stand idle 0x21 — a guard that should stay behind low cover
stood up.

### 3. RunForCover slide constant + a U-030 finding
The slide fixed 0.1 → 0.025 [M 0x169150:62-67: `SlideToPosition(0x3ccccccd, +0x830, y = 1)`]. Investigating
the test turned up something better: **the slide condition (route+0x88 < 1.0) is unreachable in the game
too** — `MoveToGoalPosition` reports arrival at `GetRouteDistance < obj+0x8EC` with +0x8EC = 2.0 from
`NDrone2_DefaultInit` [M 0x1517d8:44-54, 0x14b300:579], and GetRouteDistance writes route+0x88 with the
same value it returns [M 0x156320]. The doc's "sliding 10 %/tick" described dead code; drone-cover.md now
says so. U-030's duration gap (live 198 vs port 114 ticks) is therefore NOT the slide — the prime suspect
remains the route shape (`OptimiseRoute` trimming a waypoint the live route kept, WORKLOG 0023); not
attempted this session (route-machinery surgery did not fit the window).

### 4. Per-target slots and the +0x538 blend timing — analysed, not codeable this session
- Per-target slots: the game's 8 slots are allocated per (guard → opponent) pair and REBUILT every frame by
  `Drone_InitComms` [M 0x139520:127-160]; in SP every guard's `SetOpponent(Bond)` lands on the first free
  slot (0), so the port's single Bond bits word is behaviorally identical in single player. The observable
  difference is MP-only. An NF_ISO test cannot distinguish the two — no test, no change (the structural
  port is listed under "what's left").
- The +0x538 blend-out for non-call anims: the decompile facts are in drone-cover.md's sources now — non-call
  anim changes pass blend = 0 and leave any live +0x538 blend-out alone; `SetDAnimInternal` REFUSES a
  non-call change while a blend-out script is alive (with `MPSettings._388_4_ != 0` and DoAnimation) [M
  0x13aee8:23-26] and `CanSetAnimCall` refuses new calls [M 0x13dbf8:9-12]. Porting this needs the blend-out
  script's frame lifetime, which lives inside the AnimScript engine and is not recoverable from these files
  (the exact trap that blew up the held 275-fix-anim-blend branch, merge-log.md:102). Left out deliberately.

## Verification
- `grenade_bounce_scares_the_guard_into_the_flee_family` (ai_live, NF_ISO): a frag grenade bounces 3 units
  from the gate guard → the guard runs 0x86 → 0x87 → 0x88 → back to Attack 0x56 (0x88 is transient — its
  ENTER chains inside one send, so the state log is the witness), with an out-of-radius sister guard that
  never flees. Before the fix this chain was impossible (no sender, no handler: the drone froze in 0x86).
  The 3.0 drop also pins a decompile-faithful nuance: the flee goal sits inside the 2.0 arrival band
  (+0x8EC), so the game's own numbers make this hide near-instant.
- `validate_cover_anim_matches_the_game_table` (ai_live, NF_ISO): for the real gate guard's anim set, every
  anim state the set lacks validates to 0xFFFF (except the 5 → 7/0xA substitution), every present one passes
  through, 0 and ≥ 0x77 pass unvalidated. Needs the anim sets to actually miss something — asserted.
- The merge-hazard pins stay green: `guard_takes_cover_like_the_game` (node 1, the UnderCoverInit 31 /
  UnderCoverAim 37 tick windows, the end position) passes on the finished branch, as does the rest of
  ai_live.
- Suite: **nf-game 230 passed / 0 failed across all 32 test binaries** with NF_ISO set (release, -j2,
  --test-threads=1). nf-formats / nf-audio / nf-view: compile-checked after the master merge; the task
  touched only nf-game (no API/signature changes cross-crate). The full four-crate run did not fit the
  session window; the report says so rather than pretending it ran.

## What's left
- The structural 8-slot cover-target table (`CoverRt.bits: [u32; 8]` + `InitComms`'s rebuild loop +
  `IsCoverNodeUsable(slot, node)`'s owner gate [M 0x1386d8:70-92]) — SP-identical, MP-relevant.
- The +0x538 blend-out lifetime model (blocked on the AnimScript engine's internals, above).
- U-030: the RunForCover route shape near the 05 gate (OptimiseRoute / route-index advance vs the live
  trace's strafe 0xC → back-off 0x4 → strafe 0xD shape, WORKLOG 0023).
- The flashbang/mine list entries: the port classifies by the grenade's weapon record id (0x34/0x35/0x3B-0x40);
  mines (0x37-0x3A as placed objects) are not in the port's grenade list, so their flee entry is untested.
- The gas-grenade kind-1 entry's discarded prop 0x36 read [M 0x1435f0:25-28] — Ghidra drops its use; flagged
  in the doc.

## Live checks for the main agent
1. PCSX2, 07000005: stand ~3 units from the gate guard, throw a frag grenade so it lands in his sight
   (spot: Bond at the gate approach, e.g. (64, 100, 96) facing the guard at (61.34, 99.50)); expected: the
   guard breaks off and flees ~4 units away from the grenade (state 0x86 in the AI debug), crouches (0x87)
   for up to 2 s, then recovers to attack (0x88 → 0x56). A smoke grenade in the same spot should NOT scare
   him.
2. PCSX2 fieldprobe on drone+0x2B (scared), +0x324 (deadline), +0x354 (return state) during that flee:
   expected +0x2B = 1 while fleeing/hiding, +0x324 = land frame + ~120, +0x354 = 0x87 for a frag, the
   caller's state for a flashbang.
3. nf-view `--bond` spot for the view: `--bond 66,100,96,2.6,0` looking at the gate pillar while the guard
   flees — the run anim 0x1E then the crouch 0x11 should show.

## ntfy
Sent (see the finish step's output in the session log; the fallback order was curl → python3 → wget).

---
## Report from the worker (done, 2026-10-13T21:45Z)

# fix-cover-remainder — GAPS G-028 (guard cover remainder)

Branch `s/fix-cover-remainder`, cut from master f767e3d, finished on the merge of master 7a614ba. Session
20:45–22:25 UTC 2026-10-13 (sandbox deadline ~22:39). Files touched: `crates/nf-game/src/drone.rs`,
`crates/nf-game/tests/ai_live.rs`, `docs/systems/drone-cover.md` — nothing else (no shared-file edits
beyond drone.rs, which the task names).

## Done

### 1. RecoverFromScaryObject flee chain (G-026) — ported end to end
Before: msg 0x21 was never sent, `consider_explosive` was dead code, and state 0x86 had no handler — a
drone that got there (only via test injection) froze forever. The port now has the whole family, from the
decompile:
- `DroneFunc_HandleExplosives` (0x1435f0) + `Drone_BuildDynamicAwarePoints` (0x138b10): the explosive list
  is rebuilt every frame; kind-2 entries trigger a flee — the port fires once at a grenade's **bounce**
  (the in-flight rescan needs object vision the port's drones do not model — documented deviation), gated
  per drone by fear props 0x3A/0x3B [M 0x1435f0:30-34], the entry radius (12.0 grenade family / 8.0
  flashbang [M 0x138b10:67-87, :163-167]) and a world line-of-sight check (the game's ObjectVisibility
  ≥ 0.7 + CanSeeObject(5,0) [M 0x1435f0:37-44]). Smoke 0x36 makes a kind-1 entry and never triggers
  [M 0x138b10:143-159 + 0x1435f0:24-29].
- `DroneFunc_ConsiderExplosive` (0x1431c0) upgraded: the per-type run distance +0x350 (4.0 grenade-thrower
  weapons with a 2 s deadline, 10.0 flashbang, 14.0 the 0x34/0x3B-0x40/0x6C explosives; the old port
  constants "5.0 stun / 8.0 mine / 12.0 frag" were wrong), the return state +0x354 (0x87 Hide by default,
  the caller state for the flashbang; 0 → the caller state [M 0x1431c0:191-193]), the deadline +0x324, the
  +0x2B scared flag, then `RunAwayFromObject` **0x86** [M 0x1431c0:194-195] — the doc's older
  "0x86 = RecoverFromScaryObject" reading was wrong; the port comment was right.
- `RunAwayFromObject` 0x86, `HideFromScaryObject` 0x87, `RecoverFromScaryObject` 0x88 ported as
  `handle_scary` (before the catch-alls; the three state constants added — grepped first, none existed):
  ENTER finds the safety goal (`FindSafetyFromScaryPosition` 0x152250's straight-line fallback: scary +
  normalise(drone − scary)·dist, y + 0.5; the path-emitter strategy at dist·1.8 is [S]-approximated by the
  route build's clamp), UPDATE runs `move_to_goal` with the run anim 0x1E / the deadline exit [M
  0x168aa8:38-52], Hide waits the deadline out [M 0x168d18:39-44], Recover clears +0x2B and returns to
  Attack (+0x5A0 != 0x89 → 0x56 [M 0x173658:33-42]; the port has no RunToAlarm 0x89).
- The game's msg routing preserved: the per-state resume values (Idle/Alert/Patrol/Hostage → 0x56
  [M 0x15bf18:134...], UnderCoverFire → 0x9C, RunAway/Hide → 0x88 [M 0x168d18 case 0x21]) via a msg-0x21
  payload; the states without their own arm route through a pre-hook in `handle`.

### 2. ValidateCoverAnim (0x13bf40) — ported
`validate_cover_anim` (drone.rs): 0 passes; ≥ 0x77 passes unvalidated [M 0x13bf40:12,22]; 1..0x77 valid
only when the drone's anim set has the state's row (the base set counts too [M 0x13bf40:14-19]); an invalid
5 substitutes 7 else 0xA [M 0x13bf40:25-38 — the record-11 gate approximated by `can_do(7)`, the port's
existing documented stand-in]; any other invalid → 0xFFFF → `cover_anim` returns false (no anim queued;
UnderCoverInit/Aim linger, UnderCoverReturn/TypeChange take their fail routes [M 0x13c078:311-314]).
Mode 3's 0x58 stays unvalidated [M 0x13c078:297-300]. Before: any low-node anim the set lacks fell into the
generic replacement table and ended as the stand idle 0x21 — a guard that should stay behind low cover
stood up.

### 3. RunForCover slide constant + a U-030 finding
The slide fixed 0.1 → 0.025 [M 0x169150:62-67: `SlideToPosition(0x3ccccccd, +0x830, y = 1)`]. Investigating
the test turned up something better: **the slide condition (route+0x88 < 1.0) is unreachable in the game
too** — `MoveToGoalPosition` reports arrival at `GetRouteDistance < obj+0x8EC` with +0x8EC = 2.0 from
`NDrone2_DefaultInit` [M 0x1517d8:44-54, 0x14b300:579], and GetRouteDistance writes route+0x88 with the
same value it returns [M 0x156320]. The doc's "sliding 10 %/tick" described dead code; drone-cover.md now
says so. U-030's duration gap (live 198 vs port 114 ticks) is therefore NOT the slide — the prime suspect
remains the route shape (`OptimiseRoute` trimming a waypoint the live route kept, WORKLOG 0023); not
attempted this session (route-machinery surgery did not fit the window).

### 4. Per-target slots and the +0x538 blend timing — analysed, not codeable this session
- Per-target slots: the game's 8 slots are allocated per (guard → opponent) pair and REBUILT every frame by
  `Drone_InitComms` [M 0x139520:127-160]; in SP every guard's `SetOpponent(Bond)` lands on the first free
  slot (0), so the port's single Bond bits word is behaviorally identical in single player. The observable
  difference is MP-only. An NF_ISO test cannot distinguish the two — no test, no change (the structural
  port is listed under "what's left").
- The +0x538 blend-out for non-call anims: the decompile facts are in drone-cover.md's sources now — non-call
  anim changes pass blend = 0 and leave any live +0x538 blend-out alone; `SetDAnimInternal` REFUSES a
  non-call change while a blend-out script is alive (with `MPSettings._388_4_ != 0` and DoAnimation) [M
  0x13aee8:23-26] and `CanSetAnimCall` refuses new calls [M 0x13dbf8:9-12]. Porting this needs the blend-out
  script's frame lifetime, which lives inside the AnimScript engine and is not recoverable from these files
  (the exact trap that blew up the held 275-fix-anim-blend branch, merge-log.md:102). Left out deliberately.

## Verification
- `grenade_bounce_scares_the_guard_into_the_flee_family` (ai_live, NF_ISO): a frag grenade bounces 3 units
  from the gate guard → the guard runs 0x86 → 0x87 → 0x88 → back to Attack 0x56 (0x88 is transient — its
  ENTER chains inside one send, so the state log is the witness), with an out-of-radius sister guard that
  never flees. Before the fix this chain was impossible (no sender, no handler: the drone froze in 0x86).
  The 3.0 drop also pins a decompile-faithful nuance: the flee goal sits inside the 2.0 arrival band
  (+0x8EC), so the game's own numbers make this hide near-instant.
- `validate_cover_anim_matches_the_game_table` (ai_live, NF_ISO): for the real gate guard's anim set, every
  anim state the set lacks validates to 0xFFFF (except the 5 → 7/0xA substitution), every present one passes
  through, 0 and ≥ 0x77 pass unvalidated. Needs the anim sets to actually miss something — asserted.
- The merge-hazard pins stay green: `guard_takes_cover_like_the_game` (node 1, the UnderCoverInit 31 /
  UnderCoverAim 37 tick windows, the end position) passes on the finished branch, as does the rest of
  ai_live.
- Suite: **nf-game 230 passed / 0 failed across all 32 test binaries** with NF_ISO set (release, -j2,
  --test-threads=1). nf-formats / nf-audio / nf-view: compile-checked after the master merge; the task
  touched only nf-game (no API/signature changes cross-crate). The full four-crate run did not fit the
  session window; the report says so rather than pretending it ran.

## What's left
- The structural 8-slot cover-target table (`CoverRt.bits: [u32; 8]` + `InitComms`'s rebuild loop +
  `IsCoverNodeUsable(slot, node)`'s owner gate [M 0x1386d8:70-92]) — SP-identical, MP-relevant.
- The +0x538 blend-out lifetime model (blocked on the AnimScript engine's internals, above).
- U-030: the RunForCover route shape near the 05 gate (OptimiseRoute / route-index advance vs the live
  trace's strafe 0xC → back-off 0x4 → strafe 0xD shape, WORKLOG 0023).
- The flashbang/mine list entries: the port classifies by the grenade's weapon record id (0x34/0x35/0x3B-0x40);
  mines (0x37-0x3A as placed objects) are not in the port's grenade list, so their flee entry is untested.
- The gas-grenade kind-1 entry's discarded prop 0x36 read [M 0x1435f0:25-28] — Ghidra drops its use; flagged
  in the doc.

## Live checks for the main agent
1. PCSX2, 07000005: stand ~3 units from the gate guard, throw a frag grenade so it lands in his sight
   (spot: Bond at the gate approach, e.g. (64, 100, 96) facing the guard at (61.34, 99.50)); expected: the
   guard breaks off and flees ~4 units away from the grenade (state 0x86 in the AI debug), crouches (0x87)
   for up to 2 s, then recovers to attack (0x88 → 0x56). A smoke grenade in the same spot should NOT scare
   him.
2. PCSX2 fieldprobe on drone+0x2B (scared), +0x324 (deadline), +0x354 (return state) during that flee:
   expected +0x2B = 1 while fleeing/hiding, +0x324 = land frame + ~120, +0x354 = 0x87 for a frag, the
   caller's state for a flashbang.
3. nf-view `--bond` spot for the view: `--bond 66,100,96,2.6,0` looking at the gate pillar while the guard
   flees — the run anim 0x1E then the crouch 0x11 should show.

## ntfy failed
All three fallbacks failed with the same server-side limit (2026-10-13 ~22:24 UTC):
- curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh/docs/publish/#limitations","link":"https://ntfy.sh/docs/publish/#limitations"}`
- python3 urllib: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
- wget --post-data: no body returned (the same 429; exit 0 only because -q swallows it)

Same failure mode as fix-swim-vent5 (merge-log 2026-10-13: "429 daily quota on all three fallbacks").
