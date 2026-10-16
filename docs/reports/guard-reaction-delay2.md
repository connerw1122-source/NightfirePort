# guard-reaction-delay2 — land 664 without breaking the measured 3x3 hit matrix

`s/guard-reaction-delay` (664) was HELD: on master plus 663/667/668 plus 664, the measured
`fix_guard_weapons_live::the_3x3_hit_rate_and_damage_matrix_matches_the_game` failed 4/4 standalone
runs (without 664 it passed 2/2). The matrix is measured on the real game, so both must hold: the
user's sight-to-fire delay ("give it half a second") AND the per-cell hit rates. Branch
`s/guard-reaction-delay2`.

## What was done

Reproduced on the exact merged tree first (origin/master e416e5f0 + the full 664 content): the 3x3
matrix fails deterministically, same-process single-threaded runs. Then bisected 664's three
sight-to-fire edits — A: the enter's `+0x3b = 0` after `DroneWeap_Fire(1)`; B: the not-seen UPDATE's
`+0x3b = 0`; C: the seen UPDATE's request-refresh replacing `Fire(1)` — one variant at a time on the
same merged tree, same harness, single-threaded, one process:

| variant | matrix | the edge cell d3/0x12 | reading |
|---|---|---|---|
| pre-664 (master) | PASS | 127/200 (0.635 vs model 0.750, dev 0.115) | the existing tolerance edge |
| A only | PASS | 141/200 (0.705) | green with margin, 3 runs |
| B only | FAIL | a damage pin: one hit at 1.5 = 0.75x2, the TooClose band | B's AI-dynamics change exposes a <3.0 close pass the matrix model does not cover |
| C only | PASS | stream byte-identical to pre-664 | C alone is a no-op in the measured scenarios |
| A+C | FAIL | 125/200 (0.625, dev 0.125), cells byte-identical to full-664 | the shift is the A×C interaction (the seen-window draw order) |
| full 664 (A+B+C) | FAIL | 4/4 standalone runs | the held result, reproduced |

B never fires in the measured scenarios (A+C is byte-identical to full-664), so the deterministic
shift is A×C. With A landed, the port's seen-UPDATE `Fire(1)` runs while the fire request is still
off (the enter cleared it), so it re-schedules — `DroneWeap_NextBulletTime` (0x179d40) draws again
and shifts every 07000001 cell. The game never re-schedules there: its refresh arm sets `+0x3b`
BEFORE its own `DroneWeap_Fire(1)` guard reads it ([M 0x164c08:63]; the Fire gate needs `+0x3b` NOT
already set, [M 0x17a8c0:9-14]), so the game's seen-window `DroneWeap_Fire(1)` sees the request
already on and never draws. The port's pre-664 seen-UPDATE `Fire(1)` was draw-equivalent only while
the enter did not clear the request; once A clears it, the port's next-tick `Fire(1)` re-schedules.
Porting C without first modelling the game's request lifecycle — the `Drone+0x3d` burst-finished
latch [M 0x171df8:160] and HandleFiring's request auto-clear [M 0x17a180:37-45] — cannot reproduce
that order, so C alone moves the streams.

## What landed

**664's A only — the entry-tick block** (`crates/nf-game/src/drone.rs`, the CombatNoRoute ENTER
arm): after `DroneWeap_Fire(1)` the port now clears the fire request (`self.firing = false`),
exactly [M 0x164c08 case 1, last line: `DroneWeap_Fire(1)` is followed by `+0x3b = 0`]. A
re-sighting guard cannot release a bullet on its entry tick — the user-visible behavior the reaction
delay is for — and the measured matrix stays green (d3/0x12 141/200 = 0.705 vs model 0.750, dev
0.045 < 0.12, three runs).

B and C are NOT landed. Each is held with a finding so the follow-up can re-land them:
- **B** (the not-seen `+0x3b = 0`, [M 0x164c08 case 3 not-seen] — right per the code) changes the
  guard's AI dynamics enough that the harness parks a <3.0 close pass it never saw pre-B; the
  damage pin then fails inside the game's own TooClose band. Needs the close pass modelled in the
  matrix model (or a live check, below) before landing — not a tolerance change.
- **C** (the seen refresh) needs the `Drone+0x3d` latch and HandleFiring's auto-clear modelled
  first (above). Alone it is a no-op; with A it is the stream shift.

## How it was verified

- Reproduce + bisect: every variant above ran on the exact merged tree (origin/master e416e5f0 +
  the 664 content), the 3x3 matrix single-threaded in one process; full-664 failed 4/4 standalone
  runs exactly as held; A-only passed 3/3.
- `crates/nf-game/tests/guard_reaction_live.rs` (664's test, kept): the three sighting paths pinned
  through the real sim path — fresh-guard sight ack 55..=65 with the first bullet ≥ ack + 59; the
  alarm-raised guard < 30; the re-sight entry chain ≤ 4 ticks. Green on the final tree.
- **The full four-crate fast suite on the final tree** (this branch already contains origin/master
  e416e5f0 — its tip is the merge-base, so HEAD is origin/master + this task's diff):
  **408 test binaries, 1804 passed, 7 failed, 72 ignored.** The sandbox kills background jobs
  between tool calls, so the suite ran as seven foreground chunks of the same
  `python3 tools/test/suite.py` runner (build step reused; every one of the 408 binaries ran once
  single-threaded; two same-named targets in different crates double-ran green and are de-duped in
  the total). The 7 failures, none in code this branch touches:
  - 3 × `disc_golden` — the documented slim-ISO allowance.
  - `fix_0c_pylon_live` (2 tests) + `probe_pylon_render` — panic `adapter: NotFound` (wgpu): the
    sandbox has no GPU. Master-side render tests (merged 2026-10-15), nf-view only.
  - `driving_ai_follow::real_mis01_track_rngp_walks_when_nf_driving_is_set` — a master test merged
    2026-10-16 11:17 UTC (0abadb83, included here via the master merge); it asserts >8/9 of the
    mis01 rNgp's 811 segments have a positive speed limit and the slim on-foot ISO's data gives
    360/811. This branch's only source diff is the drone.rs CombatNoRoute enter block; the test
    builds its own `AiCar` on `nf_game::driving` and reads the ISO — the branch cannot reach it.
    Flagging for the main agent: verify on the full image before the merge.
- What I ran: the seven-chunk full suite above (nf-game, nf-formats, nf-audio, nf-view — every
  test binary, single-threaded), plus the bisect matrix runs it reports. `cargo test` crate lines:
  the suite covers them; no separate crate run beyond the chunks and the earlier per-variant
  matrix runs.

## What's left (for the follow-up)

1. Model `Drone+0x3d` (the router's burst-finished latch, [M 0x171df8:160]) and HandleFiring's
   request auto-clear ([M 0x17a180:37-45]), then port C and re-run the matrix — the trio can then
   land without the draw-order shift.
2. Land B together with the <3.0 close pass modelled (or live-checked) — do NOT widen tolerances
   for it.
3. 688 (cross-process determinism) is queued for the process-to-process variance seen during the
   bisect; every matrix run here was same-process for exactly that reason.

## Live checks for the main agent

For the B/C follow-up, on 07000001 (or the cellar), with a guard that loses sight mid-burst at
<3.0 units, gdbwatch on:
1. `Drone+0x3b` (the fire request) at the tick the guard enters the CombatNoRoute case-3 not-seen
   arm (0x164c08 case 3): expect a write of 0 on that tick and no bullet release while not-seen.
2. `Drone+0x3d` (the burst-finished latch) reads/writes around 0x171df8:160 in the same window:
   expect set when the scheduled burst drains, cleared on the next sight ack.
3. Whether the guard's next seen UPDATE calls `DroneWeap_NextBulletTime` (a draw) or only refreshes
   `+0x3b` ([M 0x164c08:63]): expect no draw while the burst is still scheduled.
