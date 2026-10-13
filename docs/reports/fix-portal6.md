# fix-portal6 — the 07 clock hall's props: `Vision_InView`'s d·r term, decoded and fixed

Task 130f. The main agent's post-portal5 render at `07000007 --bond 34.25,44.13,-70.71,1.549,-0.070`
drops the grandfather clock at the end of the hall and BOTH wall shields with crossed swords (walk ON;
`NF_NOPORTAL=1` draws them; the PCSX2 ground truth has them all [L docs/live/2026-10-13-portal-ground-truth.md]).
Branch `s/fix-portal6` = origin/s/fix-portal5 (073b055) + origin/master (0cda458); only `docs/WORKLOG.md`
conflicted (both sides kept, append-only). Branch-name note: the task header/rules say `s/fix-portal6`
(repeatedly), the tail said `s/portal6`; neither existed on origin — `s/fix-portal6` created and used
throughout (the fix-portal5 precedent).

## 1. Where the clock and the shields actually sit (probe, `fix_portal6_live.rs`)

- The walk at the hall spot resolves eye cell **33** (`Room_Corridor_I`) and visits exactly **[33, 29, 31]**
  (`Room_junction_J_1`, `Room_Corridor_J_2`).
- The clock (`OBJECT_GrandFatherClock1`, port cell **50**, sphere (44.3, 44.3, −70.5) r 2.65 — dead ahead,
  ~10 units down the hall) hangs on **room 29's** `+0x18` chain under BOTH owner rules (the old
  `find_cell_raycast` and fix-portal5's `build_link_objects_to_rooms` 0x1c9150 pick — no owner divergence).
- The wall shields hang on **room 33's** chain, likewise under both rules. At this spot's frustum the game's
  4:3 cone (half-angle 37.56°, from the snapshot's fov 1.0472 + aspect 1.3333) contains only the **x=38 pair**:
  `OBJECT_ShieldBlackwhite` (cell 45, left wall z=−73.1) + `OBJECT_ShieldBlackwhite1` (cell 46, right wall
  z=−67.8) and their crossed-swords overlays (`…Swords` cells 269/270) — "BOTH wall shields with crossed
  swords". The x=34..36 pairs sit 50..94° off-axis.
- **Every chain node of every visited room fails the port's `Vision_InView` at this spot** (20/20/2/4 nodes,
  0 pass) — that is the whole regression: fix-portal5's own-cell alignment stopped these materials keying
  under accidentally-visible cells (the fix-portal2-era "match" was the misalignment accident), exposing the
  chain-side gate that drops them.
- 07000007 also has 55 generic cells on NO chain (the radius/bbox/scoped-collision gates) — a separate,
  pre-existing divergence, not this regression (the clock/shields are not among them). Left as-is; live check
  below.

## 2. The mechanism: `Vision_InView` 0x1e89e0's d·r term

The chain-node gate in `View_AddCels` 0x1e6ba0 [M] is `Vision_InView(cel+0x8c, cel+0x80, viewer)` — same for
`View_AddObjects` 0x1e62d8 (objects, skipping 0x8000) and `View_AddForcedObjects` 0x1e5fb8 (0x8000 only). All
three gate on the viewer+0x70 planes with the sphere quad (centre.xyz, radius).

- The RAW disassembly of 0x1e89e0 (100 bytes, decoded instruction by instruction): `LQC2` the sphere quad and
  the plane quad, `VMUL` component-wise, three broadcast-adds (bc fields y, z, w — the horizontal dot), then
  **`c.lt.s f0, f12; bc1t → li v0,0; jr`** — reject iff `n·c + d·r < r`. The decompile's
  `if (fStack_20 < -param_1)` is a Ghidra float-compare polarity artifact.
- The planes: `Vision_Calculate_View_Fustrum` 0x1e7df8 [M] transforms five local points (the eye (0,0,0,1) and
  the far-quad corners (±tan·aspect·200, ±tan·200, 200)) by the viewer matrix, `Plane_PlaneEq` 0x1df8d0's them
  (n normalized, d = −n·ref), and `amemcpy`s the four side planes to viewer+0x70. **Verified against the game's
  own memory** [L ee_start_05.bin: glb_viewer[0] = 0x1fc6c10; the four quads have |n| = 1.000 exactly,
  n·eye + d = 0 within float noise (eye = matrix row 3 (104.954, 25.663, 108.825)), d = +51.4/+62.8/+95.9/+116.9,
  and the normals decompose exactly as cos(30°)·up + sin(30°)·fwd and cos(37.56°)·row0 + sin(37.56°)·fwd — the
  4:3 horizontal half-angle]. So d = −n·eye: **|d| scales with the eye's distance from the world origin**.
- Consequence: the literal `n·c + d·r ≥ r` form is untestable-absurd in BOTH polarities at gameplay spots away
  from the origin. At the 05 start (|eye| = 153, all d > 0) the d·r term adds 130..300 of per-plane slack —
  draw-everything. At the clock hall (eye (34.25, 44.83, −70.71), yaw 1.549 looking away from the origin) the
  left plane's d = −69.6, so the on-axis clock (inside distance +7.2, r 2.65) computes 76.8 − 184.5 = −107.7
  and is rejected — **the game's own construction + its own formula cannot draw a clock the game demonstrably
  draws** [L]. No portal edge reaches the prop cells (07000007 has 34 portal records, all room↔room; 0 dropped
  in the port's parse), so the unconditional list-0 path cannot cover them either.
- The three readings of the SIMD sum (`n·c + d·r + r`, `n·c + d·r − r`, `n·c + d + r`) agree exactly when
  |d| ≪ 1 — i.e. the game's *effective* gate at gameplay must be the signed-distance form. The port ships
  `n·c + d + r >= 0` (the common limit), with the full analysis in `vision_in_view`'s doc comment and the open
  question under "Live checks" below.

Fix: `crates/nf-view/src/scene.rs` `vision_in_view` — one line of math changed (`+ p[3] + radius` instead of
`+ p[3] * radius + radius`) plus the evidence doc. No other code change.

## 3. The pins (RED first, then GREEN)

`crates/nf-view/tests/fix_portal6_live.rs` (NF_ISO):

- `the_clock_hall_props_draw_with_the_walk_on` — **RED before the fix** (`the grandfather clock draws with the
  walk ON` — cell 50 not in the game mask), GREEN after: the walk visits exactly [33, 29, 31]; the clock's
  cell 50 rides room 29's chain and draws with its ≥17 World materials; the x=38 shield bodies (45/46) and
  swords (269/270) ride room 33's chain and draw; sister: the directly-beside-the-eye shields (43/48, 86..94°
  off-axis) stay culled. Documented divergence: cells 44/47 (55..58°) sit outside even the port's wider 16:9
  frustum centre-edge but their r 0.59 spheres poke the plane — the port draws them GPU-clipped off-screen
  (the game's 4:3 cull drops them); inherent to rendering 16:9 with the game's 4:3-authored cull spheres.
- `the_07000006_second_spot_keeps_its_props` — the fourth ground-truth spot (`9.87,43.04,9.84,3.177,0`): the
  walk resolves (visits [23, 11, 24, 25, 30, 33, 19]) and both in-frustum barrel props draw.
- `probe_portal6_clock_hall` (log-only) — the full attribution dump: the walk, the prop cells/owners under
  both rules, the radius/bbox/scoped gates per candidate room, the per-plane `Vision_InView` values, the
  visited rooms' chains, the orphan census.
- `probe_portal6_viewer_planes` (log-only) — the [L] snapshot evidence: glb_viewer[0]'s four +0x70 quads, the
  matrix, the VisCelList at the captured moment, the |n| = 1 / n·eye + d = 0 / tilt-angle verification.

Kept green (run this session): `fix_portal5_live` 4/4 (the courtyard walls pin included),
`fix_portal2_live` 2/2 (the ballroom window stays hidden, the start band holds), `fix_portal_live` 6/6.

## Live checks for the main agent

1. **PINE/gdbcall at the clock-hall spot** (`07000007 --bond 34.25,44.13,-70.71,1.549,-0.070`, god mode,
   mid-gameplay): dump `glb_viewer[0]` (0x2d88b0 → the struct) +0x70..0xb0 (the four side-plane quads),
   +0x114 (aspect), +0x118 (fov), +0x120..0x150 (the matrix). Expected if the port's model is right: d values
   SMALL (|d| ≪ 1) or otherwise inconsistent with the 0x1e7df8 far-quad construction — that decides between
   the port's signed-distance form and a literal `d·r` reading. Also dump the clock cel's +0x80/+0x8c sphere
   and the clock OBJECT's obj+0x80/+0x8c (Control_BuildWorldSph 0x133070's values).
2. **gdbcall `Vision_InView` 0x1e89e0** at that spot for the clock's cel: the return value with the game's own
   inputs (expected 1 = visible, since the game draws it).
3. The 07000007 orphan generics: `cel+0x1c` owner word for the 55 unchained cells (e.g. cells 42/54/92/93/126..133)
   — whether the game chains them somewhere the port's gates reject (the radius gate `g.radius <= room.radius`
   and/or the per-cel collision assignment residue fix-portal5 named).

## Render spots for the main agent (all four pinned in tests)

- `07000006 --bond 7.24,42.03,-39.31,10.161,0.428` — the courtyard walls still solid (the portal5 pin).
- `07000007 --bond 61.11,44.12,-52.58,-2.541,-0.010` — the window: only the snowy mountain, no lit building.
- `07000006 --bond 9.87,43.04,9.84,3.177,0` — fine; the barrels at the bottom draw.
- `07000007 --bond 34.25,44.13,-70.71,1.549,-0.070` — **the clock at the end of the hall and BOTH wall
  shields with crossed swords (the x=38 pair) now draw; the two guards were never affected.**

## What's left

- The live checks above (the +0x70 quads mid-gameplay decide the final form of `Vision_InView`).
- The 55 orphan generics on 07000007 (and the per-cel collision-assignment residue fix-portal5 named).
- The full-suite numbers below were still running at the report commit; the portal groups + the four pins are
  green as listed.

## Suite status

Running in the background at report time (`cargo test --offline --release -j2 --no-fail-fast -p nf-game
-p nf-formats -p nf-audio -p nf-view -- --test-threads=1`); the count will be appended to the WORKLOG entry
when it finishes. Slim-ISO allowances: the 3 `disc_golden` stubs and `pss_movies_live` without NF_MOVIE_ISO.

## Files touched

`crates/nf-view/src/scene.rs` (`vision_in_view` + doc), `crates/nf-view/tests/fix_portal6_live.rs` (new: two
pins + two probes). No shared source file touched (the merge brought master's changes in; no conflicts outside
docs/WORKLOG.md).
