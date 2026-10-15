# fix-door-react-path2 — the walk 2/2; the full suite is the remaining item

## What was done

Landed 110p (`s/fix-door-react-path`, merged into this branch from `origin/s/fix-door-react-path`,
WORKLOG conflict resolved append-only) and fixed the hold: `nf-view/tests/walkreal_07000001_live.rs`
`walk_from_the_spawn_to_the_exit_with_the_contact` no longer times out — **2/2 passed in 522.79 s**
on the merged tree.

**Where the walk stalled (probed, not guessed).** The contact stalled because armed guards stayed
alive around him, and every Bond flank nav leg to those guards died at door #69
(−14.06, 5.35, 13.05 — `SlidingDoor2`, kind 2, group 0, ch 0/0, radius 1.564 [D atlas door.md /
ee_start_07000001 snapshot]) after its 6000-tick cap. Two stacked facts from an env-gated per-tick
door trace of the real walk plus a standalone probe:

1. **The door at the stall point is game-accurate and NOT stuck.** It cycles open-dominant: the
   auto-close fires at `open_timer` 360 — unconditional on this level, the proximity scan skips
   everyone because player and drones carry `obj+0xF0` bit 0x10 [M 0x136678-0x136684; atlas
   door.md lines 139-141] — the leaf slides to pos01 0.63-0.83, and within 1-2 ticks the collide
   anti-crush reverses it to OPENING, because a live enabled guard parks at (−11.6, 6.3, 10.5),
   inside the open leaf's ×1.5 pick zone. That is the ported game rule working as designed.
2. **The stall is the walker's.** `use_door` on an already-open/opening door returned without
   moving Bond. The nav leg's straight line to the far node crosses the door plane at
   x ≈ −17.05 — west of the opening (x ≈ −14) — so Bond wedged on the jamb wall face ~2.7 short
   of the open doorway for 2500+ ticks (`segment_to_target` hitting at t 0.03-0.10) and the leg
   burned its cap. A/B on master's `door.rs`/`drone.rs` reproduces the same wedge from the same
   spot: it is pre-existing walker geometry, exposed because 110p's more accurate door states and
   sounds re-order the escort choreography into that dead-end.

**The fixes.**

- `crates/nf-view/tests/common/walker.rs` (test infra): `use_door` on an open/opening door now
  walks Bond through the doorway centre — steer at the door pos until dxz < 0.9, capped at 90
  ticks — the real player's move — instead of a bare return. The door's cycle (auto-close →
  ReactToHit reversal) simply re-runs the arm.
- `crates/nf-game/src/drone.rs` (the game's-way code fix): the class-2 contact gate in the collide
  hook gains the AI-active bit — `valid: !x.corpse && x.health > 0.0 && x.enabled` — implementing
  the full decompile gate `(ext+0x4F8 & 0x600) == 0 && (ext+0x4F8 & 0x100) != 0 && +0xAC > 0 &&
  !(obj+0xFE & 1)` [M Door_ReactToHit 0x135980:47-72; docs/fields/drone.md: 0x4F8 & 0x100 is AI
  active, set by DefaultInit 0x14b300 and Enable(1) 0x149dd0:27, cleared by Enable(0) 0x149dd0:14;
  0x200/0x400 are the dead/died bits]. The old stand-in missed 0x100, so a disabled-but-alive
  drone (enable channel off) could reverse a closing leaf, which the game never does.

## How it was verified

- **Reproduce before the fix:** the walk test killed at the 630 s cap on the merged 110p tree
  (rounds 9-10, the contact stalled at (−20.6, 6.3, 14.6), guard 2 alive at (−29.7, 10.2, 13.7)).
- **After both fixes:** `walkreal_07000001_live` **2 passed / 0 failed in 522.79 s**
  (`cargo test --offline --release -p nf-view --test walkreal_07000001_live -- --test-threads=1`,
  NF_ISO slim image), i.e. the escort walk AND the AND-touch filter sister.
- **Fail-first evidence** lives at the walk level: the test failed before the walker change and
  passes after (same tree, same ISO). The unit-level fail-first for the enabled gate is NOT yet
  written — the recipe is in "What's left".
- Scratch probe `crates/nf-view/tests/zz_door_stall_probe.rs` is untracked and not committed; the
  env-gated `NF_DOOR_TRACE` block added to the walker during diagnosis was reverted before the
  commit.

## What's left

1. **The full suite on the final tree** (`NF_ISO=… python3 tools/test/suite.py`, then once more
   merged with `origin/master`): the 2-hour window closed before it could run. The only affected
   binaries re-run in-window: the walk test (2/2, above). The suite total line is owed.
2. **The unit-level fail-first for the enabled gate** (needs a `Drones::tick` live test, ~30 min):
   a disabled-but-alive drone inside a closing kind-2 leaf's pick zone must NOT reverse it (the
   door completes the close); the sister — the same drone after its enable channel fires — must.
   Before the drone.rs fix the first assertion fails (the old stand-in had no enabled check).
3. The drone-side pick remains the documented 110p stand-in [S]: point capsule, radius 0.4, the
   validity gate as above; the real drone HITTEST replaces it when ported.

## Live checks for the main agent

1. PCSX2, 07000001, door #69 (−14.06, 5.35, 13.05): stand Bond ~2.7 south of the door with it
   open and wait 6 s — expect the auto-close to begin and `Door_ReactToHit` to reverse it while
   the leaf is still at pos01 ≈ 0.6-0.9 (the guard at (−11.6, 6.3, 10.5) is the qualifier); the
   door never reaches CLOSED while he stands there. Watch `ext+0x6C` (2-tick lockout) and
   `ext+0x70` bit 0x10 at the reversal tick.
2. PCSX2: disable a guard's enable channel (or find a pre-enable drone) within the pick zone of a
   closing door — expect NO reversal (the leaf closes through the zone). This pins the 0x100 gate
   the fix adds.
3. nf-view `--bond` spot (−13.45, 6.4, 10.4), door #69: E once, then walk north — expect Bond to
   pass through the open doorway (no jamb wedge) and the door to cycle open-dominant behind him.

## Shared files touched (per the shared-files rule)

- `crates/nf-game/src/drone.rs`: one validity-gate term in the collide hook's drone mover build
  (additive, one line + comment).
- `crates/nf-view/tests/common/walker.rs`: the `use_door` open-door arm (test infra, additive).

## ntfy failed

All three fallbacks returned the ntfy.sh daily quota 429:
- curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
- python3 urllib: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
- wget: empty reply with the same 429 (quota), no output.

The queue close (finish.sh `blocked 111c-fix-door-react-path2`, report pointer + result line appended and pushed to `origin/queue`) succeeded — this report is the notification of record.
