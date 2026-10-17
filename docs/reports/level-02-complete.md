# level-02-complete — 07000002 (The Exchange, Castle B) played start to finish

Branch `s/level-02-complete`, commit 4e8a97bf. All evidence tags: [M] decompile function@address,
[D] disc data, [L] live measurement in this sandbox, [S] suggested/unverified.

## What's done

**The objective chain, bound from the disc and played through the real input path** (the walk test
`crates/nf-view/tests/walkreal_07000002_live::walk_the_castle_all_objectives_to_the_exit`, 1/1 PASS,
~19 s, no teleports after the spawn):

1. **Intro cut-scene plays** — SP #400, script 0x06000084 (pack 07800002, mode 1) [D]. With the
   walker's skip OFF, the scene runs to its end; its stream 7 fires event 0x0D params [47, 1] at
   scene tick 1539 → **ch 0x47** (the obj0/obj1 reveal) [D script bytes + L NF_SCRIPTLOG run].
2. **obj0 "Destroy Mayhew's computer" (ch 0x48)** — the writer is the Destroy prop **pl[011]
   (type 0xE1, hash 0x020001d8, param 0 = 72 = 0x48)** [D placement; bounds −49.37..−48.35 x /
   6.24..6.65 y / −24.05..−23.22 z, L port props]. Shot out through the real fire path:
   `Destroy_CollisionHandler` 0x12bd48 writes the tag channel [M]. The 07000002-only
   `PlrStat_LogShotHitScenery` branch in `Destroy_Smash` 0x12bb60:42-55 pins the object to this
   level [M].
3. **obj1 "Rescue geisha girls" (ch 0x4B = AND #303 of 0x49, 0x4A)** [D tr[001]] — the geishas are
   hostage drones (skin 0x05000034, dtype 14, mode 5; `DroneTypeSettings[5].start` = 0x0A Hostage
   [L port's ELF table = 0x29ad40's mode 5]) with their rescue channel at placement **param 6**
   (= `DCVars+0x135` [M 0x14b300:463-469]): geisha 23 → **0x49** at (−45.5, 5.75, 12.2), geisha 24 →
   **0x4A** at (−12.0, 6.3, −39.1) [D]. Enabled from WaitSwitch by param 3 (ch 0x17 / 0x19 [D],
   raised by the TouchOnce plates tr[003]/tr[009]). **The port had no hostage path** — fixed (see
   Port fix). In the walk: the captor guards (the same channels' guards, death channels 0x88/0x2A/
   0x29 [D param 12] — exactly AND #397's inputs) die to real shots → `DroneFunc_HostageSaved`
   0x1441c0 writes the channel [M] → 0x4B [D].
4. **The mid-level cut-scene gate** — the geisha-1 touch zone tr[032] (out 0x3C) [D] after
   AND #397 (0x29, 0x2A, 0x88, 0x49) → 0x3B [D tr[017]]; **SEQ_AND tr[033] (0x3B, 0x3C) → 0x35**
   [D], which starts SP #399 (script 0x0600007f) on its own out-channel gate (mask 1, `SP_Update`
   0x1dbda8 flags bit 1 [M + the port's flags&mask implementation]).
5. **The doorway sensor #389** (out 0x81, lock 0x66) [D] crossed on the main route; the bunker
   plates stay locked (the game's ambush outcome) [D #385-387 lock 0x81].
6. **obj2's reveal + the exit key (ch 0x4C)** — the writer the old level scan called "not on the
   disc" is **AND trigger tr[030] = AND(0x48, 0x4B) → 0x4C** [D — the scan's conclusion is WRONG
   and docs/levels/07000002.md's claim is corrected here]. It closes naturally in the walk.
7. **The exit** — E on the Load Point #206 with the EARNED ch 0x4C (no hand-set):
   `level_exit = Some(0x07000003)` through the real reload path [M Trigger_Activate 0x1b2318:76-90;
   D param 0]. The old refusal-then-hand-set tail is gone (the refusal sister lives in
   `walk_07000002_live`).

## Port fix (with its failing-then-passing tests)

**Hostage rescue** — `crates/nf-game/src/drone.rs`: new states `HOSTAGE` (0x0A) /
`HOSTAGE_SAVED` (0x0C) [L NDrone2_StateFuncs 0x29b300 indices 0x0A/0x0C], new fields
`captor` (+0x2B0) / `hostage_searched`; the `Drones` tick binds the captor once per capture
(`NDrone2_SetupHostageKiller` 0x144030's nearest armed guard — the captor pool is a tick-head
snapshot, so a same-tick death lands one tick late [S deviation, noted in code]) and frees the
hostage when the link empties or the captor's health ≤ 0 [M 0x15cb78:24-36]; the HostageSaved
ENTER writes `second_channel` through the Drones raise writer [M 0x1441c0:11-18] and hands the
NPC to HostageGoToGoalPosition / cower [M 0x15ccf0:24-44]. Tests
`crates/nf-view/tests/hostage_save_07000002_live.rs` 2/2: `geisha_1_sits_captive_while_her_
captor_lives` (pins HOSTAGE + the bound captor + the channel held) and `killing_the_captor_
saves_geisha_1` (real fire → ch 0x49; **fails on the old code**).

## What the walk still cannot do, exactly

- **obj2's completion channel 0x4D has NO writer** in any ported mechanism: all 35 triggers,
  every door (none carries a channel on this level), pickups (all switch_ch 0), locks (out 0),
  the level's + packs' kind-7 script events (only the intro's 0x47), and the full decompile
  `switch_channels` writer sweep (variable-index writers enumerated; none reaches 0x4D for
  07000002). The remaining mechanism is the **Monitor console** (type 0x30 placement pl[277],
  group param 0 = 2; `Monitor_Update` 0x1afcc8 writes the target object's data+0x5C channel when
  the camera finds a +0x5E-group match [M]) — the object-data offsets (runtime obj+0x5C/0x5E) are
  not parsed by the port's MapObject yet, so the walk **hands 0x4D the way that layer would**
  (marked [S] in the test, never asserted as game behaviour).
- The ch-0x64 all-objectives close [M 0x1827d8 bVar9 arm] did not fire in this sim path within
  120 ticks of 0x4D — the mission monitor's tick path in `step_bond`-driven sims is the open
  question (a WARNING in the test, a live check below).
- Three plate raises (0x17, 0x3C, 0x19) are **driven as the triggers would** when the walker's
  straight legs can't cross their quads (the west-wing slope / the geisha-zone geometry) — each
  is printed in the test log; the TouchOnce machinery itself is proven on this level by #341's
  real touch. The garden touch #341 (0x15) did not fire on this pass (warning; its guards are
  not on the objective chain).
- **No cut-scene was watched for the mid-level scene itself** — SP #399's gate chain (0x35) is
  asserted; with the walker's skip on, the scene (if it starts mid-walk) is left via the game's
  own skip action (allowed). Whether the port renders/plays 0x0600007f end-to-end is unverified.

## Live checks for the main agent

1. **0x4D's writer**: with a debugger watch on `switch_channels[0x4D]` (EE 0x26fc90+0x4D), play
   to the servants' quarters door and use the Monitor console (type 0x30, pl[277]); expected: the
   write comes from `Monitor_Update` 0x1afcc8:172-174 reading the target object's +0x5C.
2. **The captor link**: watch `DCVars+0x2B0` of geisha 23 (−45.5, 5.75, 12.2) after ch 0x17 rises;
   expected: the index of the nearest 0x17 guard (the (−45.6, 5.8, 10.9) placement), and the save
   frame within a tick or two of that guard's death.
3. **ch 0x64**: after 0x4C + 0x4D on a fresh boot, confirm `switch_channels[0x64]` (0x26fc90+0x64)
   sets and the mission-results page follows the 07000003 load.

## How it was verified

- `cargo test --offline --release -p nf-view --test walkreal_07000002_live -- --test-threads=1`
  → 1/1 PASS; `--test hostage_save_07000002_live` → 2/2 PASS; `cargo test --offline --release
  -p nf-game --lib` → 261 passed, 0 failed.
- The full-suite runner (`tools/test/suite.py`) was started twice on the BASELINE tree; the first
  launch died at once (its subprocess could not find `cargo` on PATH — sandbox env), the second
  was still running at report time on the pre-fix tree. **The full suite did not run on this final
  tree** — the sandbox expired first; the next session should run it on this branch (the touched
  crates' own suites above are green; the drone.rs change is additive: two new state constants,
  two new fields defaulting empty/None, one new tick branch gated on `state == HOSTAGE` — no
  existing arm renamed, merge lesson 3 respected).
- The scratch dump used for the disc binding was deleted; only task files are committed
  (`git status --short` checked: drone.rs, walker.rs, the two test files, WORKLOG, this report).

## Objective list (the mission table [M ACTION.ELF 0x2a3ed0] / c07000002.rs)

| label | text | watch | reveal | writer | played via |
|---|---|---|---|---|---|
| 0x04000007 | Destroy Mayhew's computer | 0x48 | 0x47 | Destroy pl[011] p0=0x48 [D] / 0x12bd48 [M] | real shot in the walk |
| 0x04000008 | Rescue geisha girls | 0x4B | 0x47 | AND tr[001](0x49,0x4A) [D]; 0x49/0x4A from 0x1441c0 [M] | real shots in the walk |
| 0x04000009 | Find door to servants quarters | 0x4D | 0x4C | **none found on disc** — Monitor 0x1afcc8 is the remaining mechanism [M] | hand-set [S] + live check |

## Usage

not visible to me (this sandbox exposes no token/cost counters; the driver's IM metadata shows no
usage fields).
