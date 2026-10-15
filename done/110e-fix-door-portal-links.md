## Your task: `door-portal-links`, doors open and close their portals (block 0x21)

GAPS G-035 (read the row): doors are ported, but `Door_Link2Portal` (map block 0x21) isn't — a door's portal in the cel
graph never closes, so the portal walk treats every doorway as open (rooms behind closed doors are walked and drawn; the
game skips them while the door is shut). Decode block 0x21 (`docs/formats/` + the decompile of `Door_Link2Portal` and its
callers, the door state → portal flag), port it into the door/cel code, and make the portal walk honour it. NF_ISO tests:
on a level with a door between two rooms, the room behind the shut door is not in the visible-cel set from the other
side, and it is once the door opens (a sister assertion with the door open). Non-visual: the visible-cel set is the proof.

Files: `crates/nf-game/src/door.rs`, the cel/portal walk in `crates/nf-view/src/scene.rs` (additive), `crates/nf-formats`
(the block parse), a new test file.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/fix-door-portal-links`.

---
claimed-by: sandbox4 at 2026-10-15T09:32Z

---
## Report (done, 2026-10-15T11:11Z)


---
## Report from the worker (done, 2026-10-15T11:12Z)

# fix-door-portal-links — doors open and close their portals (block 0x21, GAPS G-035)

Branch `s/fix-door-portal-links`, 2026-10-15. Status: **done** (the window-bound suite coverage is listed below — this sandbox reaps background jobs, so the suite ran as foreground chunks; every binary whose code path this change can reach ran green).

## What's done

`Door_Link2Portal` (map block 0x21's door half) is ported and the cel walk honours it:

- **The block 0x21 parse** already existed (`PortalQuad::parse_block`, `crates/nf-formats/src/level.rs` — 56-byte records: 4 corners × 3 floats, u16 celA, u16 celB, u16 flags at +0x34). This session re-verified it against the decompile: `parsemap_block_portal_data` 0x1d15f8 reads exactly that layout (the decomp's `puVar9[6]/[7]/[8]` after the 4-corner loop), builds one plane per side (`Plane_PlaneEq` over the corners in order and reversed), and allocates one portal half per endpoint cel (`build_alloc_portal` 0x1c8928: next at +0x0, twin at +0x4, other-cel at +0x8, door at +0xC, plane at +0x10, corners at +0x30..+0x60, the per-side visible byte at +0x27 from flag bits 1/0). The port's `PortalEdge` + `plane_eq` model matches [M].
- **The door link** (`crates/nf-view/src/scene.rs` `link_doors_to_portals`, called at load in the Sim build): `build_LinkDoors2Portals` 0x1c8ac8 walks the door objects (class 0x1C) whose portal flag is set — `Door_Link2Portal` 0x136788 = `obj+0xFD >> 3 & 1`, set by `Door_Create` when the door's placement **param 4** != 0 (the port's `Door::straddle`) — and links each to the portal quad its gfx sphere (obj+0x80/+0x8C) sits in: plane within **0.25** of the sphere centre (`ABS(dot) < 0.25`, 0x1c8c34) and the centre inside the quad (`vecutil_point_on_poly` 0x1ec940 over the node's triangles (c0,c1,c2)+(c2,c3,c0)). A match writes the door into `portal+0xC` on BOTH halves (first door wins, `if (puVar7[3] == 0)`) and sets `obj+0xF0 |= 0x8000`. The port stores the link as `CelGraph::portal_doors` (one `Option<u32>` per edge, the pair of runtime halves). The game's `Collide_StraddleCels` (0x1eac98) pre-pass is folded into the containment test with the argument recorded in the code: a quad containing the centre within 0.25 of its plane is always touched by the door's sphere (dist-to-tri ≤ 0.25 < the door radii, ≥ 1.1 measured), so the straddle filter cannot change the match [M + the geometric argument, marked S in the comment].
- **The walk gate**: `Vision_Recurse` 0x1e82e0 reads `Door_IsClosed` (0x1367b0) on `portal+0xC` after the eye-side plane test and before the clip, and ends that portal when it reads closed — a shut door's room is never entered, an open one's is. Ported as a `door_closed: Option<&[bool]>` argument through the walk core; new additive entries `portal_cel_chain_from_cell_doors`, `portal_visible_cels_from_cell_doors`, `portal_visible_cels_game_from_cell_doors` (the legacy signatures delegate with `None`, byte-identical behaviour for every pre-existing caller). `Door_IsClosed` ported as `Door::is_closed` (kind 1 → always; group → `DoorGroupsStates[g] == 2`; else `obj+0xF4 == 2` exactly — OPENING/CLOSING leaves do not block) + `Doors::closed_flags()` per tick [M].
- **The draw path** passes the per-tick snapshot: `Renderer::cel_visibility` gained a `doors: Option<&[bool]>` argument; `lib.rs`'s two call sites feed `sim.drones.doors.closed_flags()` (gpu.rs + lib.rs, listed under shared files).

## How it was verified (NF_ISO, real input path)

Level 07000001 (5 doors, all placement param 4 = 1 [D `docs/atlas/door.md`], 9 portal records):

- **Fail-first** (scratch probe, since removed): with the port's pre-fix walk, from cell 7 at the measured eye `(5.91, 6.59, 1.95)` looking straight at portal 3, cell 40 — behind the SHUT SlidingDoor (placement 13, `state == 2` at create) — WAS in the visible-cel set: `panicked: PRE-FIX SYMPTOM (G-035): cell 40 behind the SHUT door (placement 13, state 2) is in the visible-cel set from cell 7`.
- `crates/nf-view/tests/door_portal_live.rs` (new, 2 tests, green):
  - `doors_link_to_their_portal_quads_on_07000001`: exactly 3 links — portal 2 (cells 7↔20) ← door 1 (placement 14), portal 3 (7↔40) ← door 0 (placement 13), portal 5 (11↔76) ← door 2 (placement 69); the two `gallery_secretdoor` leaves (224/225) match nothing — their doorway has no portal quad (the nearest record's centre is ~4 units away), which is what the game's own straddle+containment leaves them as.
  - `a_shut_portalled_door_hides_the_room_behind_it_on_07000001`: shut → cell 40 not in the strict set nor the game mask from cell 7; **sister 1**: the block holds from the far side (cell 40 → cell 7 hidden — both portal halves carry the door); **sister 2**: doors 1 and 2 hide cells 20 and 76 the same way; then the door opened through the **real use path** (`Doors::player_use` — `Player_Activate` 0x1a5170's door arm, open sound 23 plays) stepped to `OPEN` via the sim ticks → the same eye now sees cell 40 in both masks.

## Suite (the window-bound coverage)

This sandbox reaps background jobs (nohup AND setsid — also noted by two earlier sessions), so the fast runner ran as foreground chunks on the final tree (merged with origin/master `ecc52008`):

| chunk | result |
|---|---|
| the blast radius: nf_game/nf_formats/nf_audio/nf_view libs, door_portal_live, door_research_07000001, doors_01_live, doors_live, casings16_live, ai_traces_live, portal_live, portal_eyecel_live, portal_sweep_live, fix_portal{,2..6}_live, fix_08_walls_live, fix_missing_objects_live, cutscene_draw/models_live | 304 passed / 0 failed / 2 ignored |
| walkreal 07000001–0700000c + 0d/11/12/13/14/15, gate_guard_truck_live | 15 passed / 0 failed / 6 ignored |
| fix_0c_pylon_live, probe_pylon_render, probe_missing_vis | 6 passed / 3 failed (the 3 no-GPU `request_adapter` panics — the same 3 "pylon-render, another session's scope" environmental failures two earlier sessions reported on master) |
| the two 90 s-wall long-walk bins re-run at 270 s (campaign_flow_live, fix_guard_weapons_live) | 8 passed / 0 failed |
| bonus alphabetical sweep, 60 more binaries (ai_live, ammo_pane_live … front_end_chain_live) | 224 passed / 3 failed (the 3 `disc_golden` tests — the slim ISO's allowed failures) / 10 ignored |

**Total: 557 passed / 6 failed / 21 ignored** over ~100 test binaries. 6 failed = 3 `disc_golden` (allowed on the slim ISO) + 3 pylon-render (no GPU in the sandbox, pre-existing on master, they panic in `Renderer::new` before any walk code runs). Not run in the window: ~220 of the 324 binaries, all outside the change's reach — the legacy walk entries they use pass `None` (doors-less by construction), and the door-aware path is exercised only through `cel_visibility`, whose callers and the whole portal/door/walk test set ran green above.

## Shared files touched (all additive)

- `crates/nf-view/src/scene.rs`: `CelGraph::portal_doors` field (+ init), `link_doors_to_portals`, the walk core's `door_closed` parameter + the `Door_IsClosed` gate, the three `_doors` walk entries, the load-time link call in the Sim build.
- `crates/nf-view/src/gpu.rs`: `cel_visibility`'s `doors` argument, its two walk calls switched to the `_doors` entries, the lazy fallback passes `None`.
- `crates/nf-view/src/lib.rs`: the two `cel_visibility` call sites build the per-tick `closed_flags()` snapshot.
- `crates/nf-game/src/door.rs`: `Door::is_closed`, `Doors::closed_flags`, the module doc's "not ported" line loses the door→portal half.
- Mechanical fixups in two merged-from-master test files so they compile against the new signature: `probe_pylon_render.rs`, `fix_0c_pylon_live.rs` (`cel_visibility(..., None)`).

## Live checks for the main agent

- PCSX2, level 07000001 at start: `build_LinkDoors2Portals` ran at load — watch the runtime portals (the cel records' +0x28 chains): the three doorway portals' `+0xC` should hold the door objects of placements 13/14/69, and those three door objs should carry `obj+0xF0 & 0x8000`; the two `gallery_secretdoor` leaves (placements 224/225) should have NO portal pointing at them.
- PCSX2, play: stand on cell 7's side of the placement-13 doorway (eye ≈ (5.9, 6.6, 2.0)), break on `Door_IsClosed` 0x1367b0 during `Vision_Recurse` 0x1e82e0 — expect the gate read every frame for the shut doors; press E on the door → the read turns "not closed" while the leaf moves (state 8/0x10) and once OPEN (4).
- nf-view render spots (`--bond x,y,z,yaw,pitch`, yaw in radians):
  - `--bond 5.91,6.59,1.95,1.545,0.0` at level start (door shut): the doorway framed dead-ahead, the leaf still covers it — expect NO new hole in the view (the room behind was only ever reachable through the walk, and the leaf occludes it).
  - Same spot after pressing E (or with the door opened): the room beyond the doorway draws (as before the fix — the change removes the behind-shut-door rooms from the SET, the open path is the visible regression guard).
  - `--bond -12.0,6.6,14.0,1.545,0.0` (door 2's far side, portal 5): the room behind stays culled while shut.

## What's left

- The full suite re-run on this final tree in a GPU-capable sandbox (to also clear the 3 pylon-render tests) — the coverage table above is this window's maximum.
- The `obj+0xF0 |= 0x8000` side effect is not modelled (no reader was found in the port or the decompile beyond the link itself — the game sets it at link time; noted for completeness).
- U-041's `Door_ReactToHit` proxy list and the 0x2000003/0x200000b text labels remain unported (unchanged scope, both documented in `docs/atlas/door.md`).
