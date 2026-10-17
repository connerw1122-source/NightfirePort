## Your task: `sensor-volumes` — sensor volumes (0x28)

62 placements: `Sensor_Create` 0x1af010 and its update. Trip volumes that anything (not only the player) can set, writing switch channels. Test: a drone walking through sets the sensor's out channel; sister: an empty volume doesn't.

The script census (task 709, merged) lists this gap with occurrence counts and the game handler in `docs/systems/script-census.md` ("Top 10 ready-to-queue tasks"). Port from the decompile [M], test on real levels through the real path with a sister assertion, and update `crates/nf-formats/tests/script_census.rs`'s handled sets.

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/sensor-volumes`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-17T01:01Z

---
## Report (done, 2026-10-17T02:07Z)

# sensor-volumes (task 749, branch s/sensor-volumes)

The 0x28 placement type — 62 on the disc, `Sensor_Create` 0x1af010, updated by
`Sensor_Update` 0x1af618 through `control_funcs[0x37]` — is ported: `crates/nf-game/src/sensor.rs`,
wired into the control walk (`Drones::attach_level` create, the `what == 6` arm in
`tick_inner`'s order chain). A sensor is the wall/ceiling security camera: it sweeps a 0x19
key-list pose (`Spline_Interp` mode 2, the same key machinery as the doors/ss), and watches a
cone (`Intersect_ConeSphere` 0x1ee100 transcribed; range gate; `Collide_LineOfSight` stand-in).
After 32 in-cone ticks (`prev > 0x1E`, disasm `slti 0x1f` @0x1af9a4) it alerts, tracks the
player with `Vec_Track` 0x1e20a0 at 0.05/tick, and after 2 s of continuous sight writes
**1 to its out switch channel** (param 3) with the `switch_channels_time` stamp
[0x1af618:152-157]. Losing him for 61 ticks returns the head (state 2, slerp 0.1, done at
`Quat_IsEqual` 0.01). The disable channel (param 4) parks it in state 3 (destroyed glist,
hum stopped); cleared → re-arm [0x1af618:244-264]. No RNG draws anywhere in the create/update
path — the shared stream is untouched.

## The test correction (read this first)

The task brief and `docs/systems/script-census.md` described sensors as "trip volumes that
anything (not only the player) can set. Test: a drone walking through sets the sensor's out
channel." **The decompile contradicts that**: `Sensor_Update` loads `glb_players[0]` once
(disasm `lw $s4, -0x7720($v0)` @0x1af644, global 0x2d88e0 — a single obj_tag\*), passes it to
both `Sensor_InCone` calls (0x1af97c sweep, 0x1afae0 track) and to `Sensor_SetBeam` (0x1afc14);
`Sensor_InCone` has no other callers, and a full census of `switch_channels` writers (0x26fc90)
shows no NPC-proximity writer. The binary's sensor is player-only. Per the task's own rule the
TEST was fixed, not the code weakened: the pinned test drives the PLAYER through the cone, and
the report (this file) documents the change. The drone-trip premise likely came from the
monitor/CCTV pairing (monitors steer sensors, drones *react to* sensor channels — the direction
is sensor→AI, never AI→sensor).

## What is pinned by tests

`crates/nf-game/tests/sensor_live.rs` (0700000b, real level through the real load path):
* 0700000b carries exactly two main-map sensors [D probe]: placement 164 pos (−10.38, 49.64,
  11.16) cone 15° range 5 out ch 0x28 disable 0x3c, 5-key/600-tick sweep; placement 172 out
  ch 0x27 — same shape.
* The create reads: sweep limit = count−1, step = (count−1)·REC·(FRAME_RATE/rate) = 4/600
  [M 0x1af010:89-96].
* **Sister "empty volume"**: 300 ticks with the player far away — neither channel ever set
  (sweep clears its out channel every tick [M 0x1af618:169-171]).
* **Trip**: the player stands on the sweep arc (the bisector point comes from the same
  `Spline_Interp` port the sim uses, at t = 0.5), LOS-checked. Ch 0x28 turns on no earlier
  than tick 152 (32 alert + 120 trip ticks [M 0x1af618:211-216,154-157]) and stays on while
  the sight holds; the sister sensor's channel never sets.
* `crates/nf-formats/tests/script_census.rs`: 0x28 moved into the handled set (57 handled /
  1475 unhandled; "49 sim + 8 draw-only").
* `docs/fields/sensor.md` — the full SENSOR/level_tag field map with addresses; the census
  doc's tables updated.

## Not ported (all outside the sim's reach)

The lens/mount gfx proxy meshes (`hashtable_set_object_to_entity_gfx`), the destroyed-glist
swap (obj+0xD8), the monitor pairing (`Monitor_Update` 0x1afcc8 / `Monitor_Activate` 0x1b0058,
the CameraList 0x319930 consumers — placement type 0x30 is unported), all sounds (0x4AA hum,
0x4DD servo, 0x4A9 beeps), and the **posed sensor-head draw** (the static camera model renders
as part of the level; the sweeping head pose is sim-only for now).

## Verification

Foreground runs on the final tree (merged origin/master @ 0f739464):
* `cargo test -p nf-game --lib` — 251 passed.
* `cargo test -p nf-game --tests --skip walkreal --skip guards --test-threads=1` — 57 test
  binaries, all ok.
* `sensor_live` ✓, `guards_0c_live` ✓ (real-level AI unaffected), `walkreal_07000002_live` ✓,
  `walkreal_0700000b_live` ✓, `walkreal_07000012_live` ✓ (0700000a/14 walkreal: ignored — their
  normal state).
* `cargo test -p nf-formats` — 79 + 2 + 11 ok; the 3 `disc_golden` failures are the allowed
  slim-ISO ones. `nf-audio` 2 ok; `nf-view --lib` 17 ok.
* The full fast runner (`tools/test/suite.py`) was started four times (plain nohup, then
  setsid-detached) and was reaped between tool calls every time — this sandbox kills background
  jobs (the merged pause-music-1b entry hit the same, "died 4x"). The foreground runs above
  stand in; **no suite total line is available** (`tail -1 suite.out` = empty).

## Live checks for the main agent

1. On 0700000b, stand the player in the plaza under camera placement 164 (−10.38, 49.64, 11.16 —
   the sweep tilts 30° down, yawing k0 (0,−0.50,0.87) → k2 (0,−0.50,−0.87)); after ~4 s of
   standing in the swept cone the alarm should raise (`switch_channels[0x28] = 1` —
   `tools/trace/fieldprobe.py` on 0x26fc90+0x28 expecting 1 after the sweep pass; the port's
   trip tick measured 347 with the playhead phase this test creates).
2. The disable channel: set ch 0x3c (60) on 0700000b → the sensor stops sweeping (state 3);
   clear it → re-arms. Access type: write then observe obj class-0x37 state at +0xF4 = 3.

## Render notes for the drawing side

The sensor is sim-only in this pass; the level's static camera model draws at the placement
pose and does NOT yet follow the sweep/track matrix. Suggested `--bond` spots for the follow-up
draw pass (the posed matrix to attach = `Sensors::list[k].matrix`):
* `--bond -10.38,49.64,11.16,0.0,-0.5` (0700000b, camera 164 up close — the head should face
  (0,−0.50,0.87) at sweep start);
* `--bond -12.2,48.1,13.0,2.0,-0.4` — the test's player spot looking back at the camera.

## Left for the next round

The monitor (0x30) pairing + the posed-head draw; sensor sounds; the lens beam ray's
object-identifying raycast (the port's beam shortcut is the static world + a sphere test [S]).

## Usage

Not visible to me — this sandbox exposes no token, cost or context-window figures for the
session (no usage meter in any interface I can read). No numbers are estimated.
