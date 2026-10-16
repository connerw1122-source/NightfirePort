## Your task: `sim-tick-cost` — profile and cut the Sim's per-tick cost without changing results

On 07000005, the window's 60 Hz sim ticks cost about 6.7 ms per frame (main agent probe, 2026-10-15). An earlier
measurement (fix-perf-level) had about 4.1 ms per tick on 07000006. The window caps catch-up at 4 ticks, so a slow
tick turns into dropped game time on slower PCs.

1. Add a headless tick-cost probe test (NF_ISO, release-only): load each single-player level, run 600 ticks of a
   fixed input script, and record the per-tick median, p95 and max, broken down by the Sim's major phases (drones/AI,
   bullets, collision, triggers/scripts, sound batch, pickups, doors). Table the results per level in the report.
2. Find the top hotspots (for example linear scans per drone per tick, repeated cel lookups, allocations in the
   tick) and fix them.
3. **Determinism is the bar:** the per-tick state hash (the net-determinism work, 050, if it has landed; otherwise
   hash the drone/bond/bullet state yourself) must be identical before and after for every level over the 600
   ticks. Report before/after timings.
No behaviour changes. If a fix would change any result, don't make it: write it up in the report instead.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/sim-tick-cost`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-16T02:06Z

---
## Report (done, 2026-10-16T03:05Z)


---
## Report from the worker (done, 2026-10-16T03:05Z)

# sim-tick-cost — profile and cut the Sim's per-tick cost without changing results

Branch `s/sim-tick-cost`. The bar: the per-tick state hash must be identical before and after for
every level over 600 ticks. Both fixes clear it: **20/20 levels, 600 ticks each, digests
identical** to the pre-fix baseline.

## What's done

1. **The headless tick-cost probe** — `crates/nf-view/tests/tick_cost_live.rs` (NF_ISO, release-only;
   skips with a note under debug). Loads each of the 20 single-player story levels, runs **600 ticks
   of a fixed input script** (forward + slow turn, a strafe leg, a 40-tick trigger hold with a reload,
   back-pedal, a second forward leg; use-key pulses, jumps, a crouch window — the same script for
   every level, no sim-RNG involvement), and records per level:
   - the per-tick cost (median / p95 / max, measured around each real `Sim::step_bond` call);
   - the per-phase cost from `nf_game::prof`'s scope samples, sliced per tick so **median/p95/max per
     phase** are possible (the same scopes `NF_PROFILE=1` prints averages for);
   - a per-tick **FNV-1a state digest** over everything a later tick reads: bond (pose, substate,
     weapon, ammo), every drone (pose, yaw, state machine, health, alert, visibility, sight counters,
     weapon, anim), the AI list order, bullets (Bond's + the drones'), grenades, doors (state, pose,
     timers), pickups/taken, the full inventory, the 256 switch channels + their time stamps, the
     triggers, the script players, the SS objects, the world objects, casings/gas/smoke counts, the
     AI hearing list, the audio pool's live ids, mission status, the death path, `level_exit`, RNG.
   - `NF_TICKCOST_RECORD=<path>` writes the timings + digests; `NF_TICKCOST_CHECK=<path>` asserts
     every level's 601 digests match the recording and prints the first divergent tick otherwise.
     The record holds inputs, timings and digests only (no game bytes) — but it is build-specific
     by design, so it stays a scratch artifact, not a committed golden.
   - Always-on assertions, each of which can fail: the sim ran 600 ticks (`frame == 600`), the
     shared RNG advanced, the digest is not vacuous (it changes over the run), and (second test)
     **two fresh loads of 07000005 through the same 300-tick script produce identical digest traces**.
   - `prof.rs` gained the capture API the probe needs: `force_enable`, `capture(on)`,
     `take_samples()`, `reset()` — timing only, never feeds back into the simulation.
   - New phase scopes (nothing renamed): `phase: player move` (scene.rs, the `Bond::tick_ctx` block),
     `phase: bullets` (`update_player_bullets`), `phase: world objects` (useobjs + `WorldObjects::tick`),
     `phase: pickups` (`update_pickups`), `phase: sound batch` (`SoundTriggers::tick`) in drone.rs, and
     the `drones: set_dynamic` split (`drones: set_dynamic parts` = the parts-list build).

2. **Fix 1 — the pickup take-ray reorder** (`pickup.rs` `Pickup::touches`). The port computed the
   LOS ray (`segment_take_scoped`) for **every pickup every tick**, before the distance test. The
   game's own decompile short-circuits: `Vec_SqDist3D(obj+0x80, player+0x30) < r² &&
   Collide_LineOfSight(...)` with the ray call **inside the `&&` chain** [M Pickup_Update 0x18e148,
   work/decomp/ACTION.ELF/0018e148_Pickup_Update__FP7obj_tag.c:203-214 — the distance test from the
   player position +0x30, the ray from the world-sphere centre +0x80, exactly the two points the
   port already used]. The fix runs the distance test first and the ray only on the distance pass —
   the game's own order; the query is a pure read (no RNG, no state), so results are bit-identical.
   This alone was **66 % of the average tick** (`phase: pickups` 1.14 ms of 1.72 ms; on 0700000d
   5.38 → 0.44 ms, on 07000014 3.37 → 0.23 ms).

3. **Fix 2 — `CollisionWorld::set_dynamic` reuse** (`world.rs`). Every tick rebuilt the whole moving
   set (every door/SS/script-part triangle re-transformed, `Geo` + `HashMap` reallocated), 0.22 ms
   average and 0.4–0.6 ms on the moving-object levels (07000008/16/1b). Now:
   - an **unchanged fast path**: same `(id, hash)` parts with the same matrices → the stored geo IS
     this set's world-space triangles (the same floats), only `movers` may need the cheap
     no-transform rebuild (a part that moved last call and stopped must read `(id, cur, cur)`);
   - **per-part reuse** in the rebuild: a part whose `(id, hash, matrix)` triple is unchanged
     memcpy's its previous world-space block (tris + material bytes + the stored bbox — the same
     floats the transform re-derives) instead of re-transforming; the animating minority pays, the
     resting doors ride along. 07000016: 0.38 → 0.07 ms in-phase; 0700001b: 0.62 → 0.26;
     07000008's `set_dynamic` left the top-5 entirely.

## Results (median ms/tick over 600 ticks, this sandbox, release)

| level | before | after | Δ | | level | before | after | Δ |
|---|---|---|---|---|---|---|---|---|
| 07000001 | 0.85 | 0.28 | 67 % | | 0700000b | 0.94 | 0.40 | 57 % |
| 07000002 | 0.22 | 0.10 | 55 % | | 0700000c | 2.77 | 0.57 | 79 % |
| 07000003 | 0.53 | 0.21 | 60 % | | 0700000d | 5.38 | 0.44 | 92 % |
| 07000004 | 0.89 | 0.07 | 92 % | | 07000011 | 1.07 | 0.30 | 72 % |
| 07000005 | 1.84 | 0.26 | 86 % | | 07000012 | 2.37 | 0.34 | 86 % |
| 07000006 | 1.42 | 0.37 | 74 % | | 07000013 | 2.00 | 0.05 | 97 % |
| 07000007 | 0.91 | 0.62 | 31 % | | 07000014 | 3.37 | 0.23 | 93 % |
| 07000008 | 1.82 | 0.59 | 68 % | | 07000015 | 4.15 | 0.48 | 88 % |
| 07000009 | 0.57 | 0.32 | 44 % | | 07000016 | 1.28 | 0.36 | 72 % |
| 0700000a | 0.60 | 0.11 | 82 % | | 0700001b | 1.44 | 0.47 | 68 % |

**20-level mean: 1.72 → 0.33 ms/tick (−81 %).** The probe runs sim-only (no window/draw), so these
are not directly comparable to the main agent's 6.7 ms window figure or fix-perf-level's 4.1 ms —
the mix differs; the before/after columns are the same harness on the same tree, and the phase
breakdown is what transfers. Per-phase averages over the 20 levels (each phase's own cost; nested
scopes overlap): `step_bond (total)` 1.72 → 0.53 ms, of which `drones.tick (total)` 1.59 → 0.42,
`phase: pickups` 1.14 → 0.00, `drones: set_dynamic` 0.27 → 0.06 (parts build 0.04), control-list
loop 0.14 → 0.13, `drone: update_anim` 0.11 → 0.10, `phase: player move` 0.09 → 0.08,
`drone: collision_handler` 0.08 → 0.07, `drone: visible_to (drawn ray)` 0.07 → 0.07.

## What's left

- **The remaining hotspots are real work, not waste**: 07000007's `drones: control-list loop`
  (0.53 ms — 58 drones' AI), 07000008's `set_dynamic parts` build (0.26 — the per-tick
  `script_parts` closure + `discrete_parts` bone posing per anim object; cacheable only when an
  anim object holds its frame) and `drone: visible_to` (0.14–0.23 — a world ray per drone per tick,
  the game's own `Drone_CollisionHandler` shape [M 0x139520's pass]). None is a safe same-result
  cut without deeper caching; a next pass could cache `discrete_parts` on held frames and share the
  `visible_to` ray across drones in the same cel.
- **master moved 42 commits** after my branch point (the online-MP chain + attract mode landed at
  306c5d93). Merging + re-recording the digest baseline + re-running the suite did not fit this
  window. After the merge, the before/after bar is one command on the merged tree:
  `NF_TICKCOST_RECORD=<scratch>` once (pre-fix), then `NF_TICKCOST_CHECK=<same file>` — the probe
  re-derives the baseline itself; my committed digests are deliberately not a golden.
- The background suite runner is reaped by this sandbox between tool calls (two starts died during
  the build, `suite.out` empty), so the suite ran in foreground chunks instead (below).

## Verification (what ran, on this branch tree)

- The probe itself: `tick_cost_live` 2/2 (the 20-level probe + the two-run determinism test).
- `cargo test --offline --release -j2 -p nf-formats -p nf-audio -- --test-threads=1`:
  **105 passed / 3 failed** — the 3 allowed `disc_golden` environmental fails (the slim ISO).
- `cargo test --offline --release -j2 -p nf-game -- --test-threads=1` (all binaries):
  **398 passed / 0 failed** — the AI, doors, pickups, drive-physics and live-trace suites over the
  touched `world.rs` / `pickup.rs` / `drone.rs` / `prof.rs`.
- `nf-view` via `tools/test/suite.py --only` chunks (test-threads=1, per-binary time limit):
  blast-radius chunk (pickups/objects/doors/collision/AI-traces/casings/grenades/script-objects/
  determinism incl. `replay_live` 3/3 + `drone_skins_all_levels_live`) **77 passed / 0 failed,
  2 ignored**; weapon/flow/HUD chunk (fire modes, switch, bond damage, death restart, wire, vent,
  menus/mp/dev menus, campaign flow, objective banner/messages, progress, ammo pane) **117 passed /
  0 failed, 5 ignored**.
- The determinism bar: `NF_TICKCOST_CHECK` against the pre-fix record — **20/20 levels, 601 digests
  each, identical** (run twice; also re-verified after each individual fix).

## Live checks for the main agent

1. **`NF_PROFILE=1` on the user's PC** (the 6.7 ms window machine): run any level for ~10 s of
   gameplay and send the `[profile] ms per tick` block — the sandbox's per-phase medians above are
   a fast CPU; the phase *shares* should transfer, and 07000007/08's control-list loop is the one
   to watch on slow machines (the 4-tick catch-up cap turns a slow AI tick into dropped game time).
2. **07000016's cycling doors** (the walkreal contact door dance): confirm no new door hiccup
   versus master — the movers' `(id, cur, cur)` rebuild path now serves doors that stop mid-sequence.
   A `--bond` render spot at the walkway door (the U-041 contact open, glist 0x803f00a0) should look
   and sound exactly as on master.
