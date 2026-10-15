## Your task: `drive-ai-traffic`, the driving missions' AI cars

From DRIVING.ELF (docs/atlas/driving-elf.md): the enemy/traffic cars' AI — path following on the track's AI
splines (find them in the CRP/VIV data), speed control, attacks (the game's chase/shooting behaviours), and their
spawn/despawn per mission (MIS01.VIV's mission scripts). Port the data (the splines, the per-mission spawn lists)
and the AI step in `nf_game::driving` beside 506's physics (merge `origin/master` first; coordinate through the
data model if 506 hasn't landed). Tests: a headless AI car follows its spline at the game's speed numbers.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-ai-traffic`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-15T20:37Z

---
## Report (done, 2026-10-15T21:46Z)


---
## Report from the worker (done, 2026-10-15T21:46Z)

# Report: drive-ai-traffic (branch `s/drive-ai-traffic`)

Task 507 — the driving missions' AI cars: the road network (AI splines) out of the track `.crp`,
the per-mission spawn model, and the per-frame AI step, ported beside 506's vehicle physics.
**No drawing** — data/state/flow only, headless.

## What's done

1. **The system doc, evidence-first**: `docs/systems/driving-ai.md` — the `RNgp` road-network
   article and its five arrays (rn/rs/ri/rr/rj) with every record field proven
   (FUN_00220c28 0x220c28, FUN_002215f8 0x2215f8, FUN_002216b0 0x2216b0, FUN_00221f20 0x221f20,
   FUN_00225490 0x225490), the path-following geometry (FUN_00222468 0x222468 straight eval,
   FUN_00222d40 0x222d40 → FUN_001a8380 0x1a8380 4-point curve, FUN_00225c48 0x225c48 join,
   FUN_002247b0 0x2247b0 next-segment), the AIVehicle struct (0x2a0 B, ctor FUN_0012dd80), the
   per-frame Update chain (FUN_0012e350 0x12e350 → states 1–5 → FUN_0012e808 target speed →
   FUN_00130508 → FUN_00133ab0 drive control), the speed numbers (the segment limit rs+0x4c
   [M 0x12e808:165]; the ramp span 7.5, floor 0.05, the target floor 0.5, the arrival radius
   max(speed·0.5, 5.0) [M 0x133ab0]), the weapons dispatch (+0x84 9/10 → FUN_0012f730/0012fa28
   [M 0x130050]), and the spawn/despawn side (the AIEl database FUN_0012b8b0, the pool
   FUN_0013e688: 6 heap + 22 static vehicles + 16 helis, activate/deactivate FUN_00131508/00131590,
   the traffic director FUN_00126c98).
2. **The port**: `crates/nf-game/src/driving/` (new module; `lib.rs` gained one `pub mod driving;`
   line):
   - `road.rs`: `RoadNet::parse_crp` — the game's RNgp walk (RNhd u16[5] counts rn/rs/ri/rr/rj,
     per-index short-tag lookups, payload resolution, the `(flags & 0x8108) == 0` plain-road note,
     the rr occupancy zeroing), typed `RoadNode`/`RoadSeg`/`RouteRec` accessors over the payloads,
     `seg_point` (the FUN_00222468 evaluation), `curve_point` (the control-point path; the exact
     FUN_001a85d0 basis is a live check), `next_segment` (the flag/self-id/car-group filter chain).
   - `ai.rs`: `AiCar` (state, segment, end, t, target speed, forced-stop), `target_speed_traffic`
     (the FUN_0012e808 gates + the rs+0x4c base), `drive_control` (FUN_00133ab0 as a pure
     `DriveCommand{steer,throttle,brake,handbrake}`), the 60 Hz `step` (target → command → progress
     → the FUN_00222468 position → the FUN_002247b0 crossing). `SimCar` (pos/vel/forward/alive) is
     the seam where 506's physics plugs in — no throttle→acceleration model was invented.
3. **Tests** (`crates/nf-game/tests/driving_ai_follow.rs`, 9 tests, all green): a hand-built
   format-exact `.crp` fixture; the headless car follows its spline at the game's speed numbers
   (25 units over 1 s at the segment's limit, dead on the line, crossing at the node onto the clean
   link, position == `seg_point` exactly); the arrival stop; the FUN_00133ab0 numbers (saturated and
   mid ramps, the 0.05 pedal floor, the 0.5 target floor, both brake early-outs, the forced-stop
   handbrake, the steer clamp); the FUN_0012e808 gates; the FUN_002247b0 filters (turn restriction,
   car group); two malformed-fixture rejections (sister assertions); the curved-segment control
   point; and `real_mis01_track_rngp_walks_when_nf_driving_is_set` — the same walk against the real
   `MIS01.VIV` track (counts, unit directions, positive limits, an on-spline walk), gated on
   `NF_DRIVING` like `driving_golden.rs` and skipping cleanly without the folder.

## How it was verified

- `cargo test --offline --release -j2 -p nf-game -- --test-threads=1` on the **final merged tree**
  (origin/master cf6d90dc merged mid-task; WORKLOG conflict resolved "both kept"; no code overlap —
  the new master's mission.rs/profile.rs touched nothing of mine): **350 passed / 0 failed** across
  all 39 test binaries. `driving_ai_follow` 9/9 and the new `objective_state` 5/5 re-run after the
  merge.
- `cargo test --offline --release -j2 -p nf-formats -p nf-audio -- --test-threads=1`: **89 passed /
  3 failed** — the 3 are exactly the `disc_golden` tests the task allows to fail on the slim ISO
  (the DRIVING/SLUS directory extents are zeroed there). I changed no nf-formats code.
- `cargo test --offline --release -j2 -p nf-view -- --test-threads=1` in two foreground slices
  (57 of 303 binaries — the rest did not fit the window): **191 passed / 2 failed**; the 2 are the
  known no-GPU pylon pair (`the_0c_pylon_fills_the_view_from_the_closeup_spot`,
  `the_0c_pylon_is_on_screen_at_the_user_spot` — environmental, recorded the same way by drive-ptp).
- The background fast-runner start (`nohup python3 tools/test/suite.py > suite.out 2>&1 &`) died
  empty: this sandbox reaps detached processes (the same observation is in screen-adjust-model's
  WORKLOG entry). The foreground slices above are the suite coverage; `suite.out` stayed at 0 bytes.

## What's left

- **The real-data golden on the paris RNgp** wants the extracted `DRIVING/` folder
  (`Nightfire-driving.7z`, 428,903,899 B on the host): the gated test
  (`real_mis01_track_rngp_walks_when_nf_driving_is_set`) is ready and will parse the real track and
  walk it, but this sandbox had 545 MB free while the suite was compiling, so the 429 MB archive was
  not pulled. The main agent can run it with `NF_DRIVING=<extracted folder> cargo test -p nf-game
  --release --test driving_ai_follow`.
- The exact FUN_001a85d0 curve basis (the 4-point interpolation) and the rs+0x10 control layout
  beyond the first handle — live check L-3.
- The per-case mapping of the AI profile +0x84 to FUN_0012e808's constants (10.0/25.0/30.0 and the
  0.35..1.25 multipliers) — the traffic base rule (rs+0x4c) is pinned [M]; the case table is [S].
- `ri`/`rj`/`rr` non-occupancy fields (no readers in the decompile).
- The AIEl chunk's on-disc record layout (the port models the parsed runtime records; the raw chunk
  parse is a live check L-5 of the doc).
- The weapons fire (§3.3 of the doc) is documented, not ported — it needs the actor/animation layer.
- 506's physics lands beside this: `driving::ai::SimCar` is the agreed seam (a PVehicle plugs in).

## Shared files touched

- `crates/nf-game/src/lib.rs` — one added line (`pub mod driving;`), nothing reordered.
- `docs/WORKLOG.md` — appended my entry; the mid-task merge conflict resolved by keeping both sides.

## Live checks for the main agent

- **L-1 — rs+0x4c is the speed limit**: breakpoint FUN_0012e808 0x12e808 during a paris_mis01
  traffic run; expect the float read at `*(rs_table + nav->0x3e*4) + 0x4c` to land in AIVehicle+0x8c
  and to match the observed traffic speed [M expectation].
- **L-2 — the lane-offset table**: dump the 6-entry table at DAT_00427000 after "Init AI
  Controllers"; expect lane-center offsets in game units [M expectation; the port carries the
  table's existence, not its values].
- **L-3 — the curve eval**: breakpoint FUN_001a8380 0x1a8380 on a curved segment (flags&0x4000);
  record the four control points (param_2/3/5/4) to pin the P0..P3 order vs rs+0x10, and step
  FUN_001a85d0 0x1a85d0 for the interpolation basis [S expectation].
- **L-4 — the real RNgp golden**: with `NF_DRIVING` set, run the gated test and cross-check the
  parsed counts against a PCSX2 dump of the DAT_0033a5fc..a60c array lengths after "Init
  RoadNetwork" [M expectation].
- **L-5 — the AIEl chunk layout**: breakpoint FUN_0012b8b0 0x12b8b0 on mission load and dump the
  raw AIEl chunk bytes vs the parsed 0x100-byte records (type at +0x44, the activate flag at +0x48,
  the link at +0x50, the model name at +0x54) [S expectation].

## ntfy

(sending now — see the report tail if it failed)
