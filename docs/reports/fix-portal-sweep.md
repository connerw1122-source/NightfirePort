# fix-portal-sweep — portal culling must never hide what the player can see

Branch `s/fix-portal-sweep` (from `origin/master` 3c77a9d). Session 00:23–02:22 UTC 2026-10-14.

## What was wrong

The playtest spot `07000005 --bond 16.67,30.04,58.60,1.755,-0.001` (the snow trench between stone
walls near grapple point #282) culled the near trench walls with the walk ON; `NF_NOPORTAL=1`
drew them. Reproduced headlessly (`crates/nf-view/tests/portal_sweep_live.rs`,
`probe_trench_spot_07000005`):

- The eye resolves to graph cell 14 — the giant outdoor room (0x0804c047, bbox (0.1,−8.8,0.1)..
  (197.8,72.0,168.2), r 135.9) that spans the whole map. The walk from it visits only
  `[14, 136, 31, 30, 16]` [M `Vision_Portal_Recurse` 0x1e86e8 → `Vision_Recurse` 0x1e82e0].
- The trench-wall generics ride the `+0x18` chains of the **overlapping rooms 15 and 24** — that
  is what the game's own owner rule computes: `build_link_objects_to_rooms` (0x1c9150) picks the
  0xc047/0xc023 room whose bbox holds the generic's centre and whose `Collide_SphereIntersect`
  (0x1ec940) hit is closest; the hit metric for the query type 0x101 is
  `Intersect_PointGeom`'s point-to-geometry distance [M 0x1ebaa8], which is exactly the port's
  `closest_point_scoped` reading. The port's owners match the game's rule.
- `View_AddCels` (0x1e6ba0, disassembly) walks the `+0x18` chain of **VisCelList rooms only**, so
  with the eye cel = 14 the rooms 15/24 are never visited and their chain nodes cannot draw:
  the walk keeps 719 World draws vs 1809 frustum-only; **806 draws within 45 units of the eye are
  all hidden**.
- The game does not have this hole because **its eye cel resolves to 15 at this spot**: the
  trench rooms then enter the VisCelList (the walk from 15 visits `[15, 24, 14, 30, 136, 16, 31]`
  and keeps 1536 draws, near-dropped 129). The game's eye cel comes from
  `build_FindCel` (0x1c8cb0)'s tie-break — per candidate an up ray then a down ray **against that
  candidate's own object list**. The port's stand-in scopes the shared collision world per cell
  (BFS across straddled portals), so both candidates see the same floor, the up/down hits tie,
  and the first candidate (14) stands. Which cel owns the trench floor poly is per-cel
  attribution the port does not model (the same attribution gap the 2026-10-13 ground-truth doc
  records for the 06 courtyard). Live check below settles it.

## The fix (evidence-cited)

`scene::near_los_rescue` (`crates/nf-view/src/scene.rs`), called from
`gpu::Renderer::cel_visibility` after the game-mask pass:

- A cel the walk drops still draws when (1) `Vision_InView` (0x1e89e0, the signed-distance form
  fix-portal6 ships) keeps its sphere, (2) the closest point of its bbox is within 30 units of
  the eye, and (3) the static LOS ray from the eye to that point is clear. Only the generic
  classes `build_link_objects_to_rooms` links ride it (0x8017..0x8019 = the chain nodes);
  room cels and their objects stay on the strict `VisCelList` mask, so intentionally hidden
  rooms stay hidden. Additive only — it can never hide anything the walk kept.
- `world::segment_hit_static` (`crates/nf-game/src/world.rs`, a minimal hook, shared file): the
  LOS ray must ignore the missile cage and the dynamic objects — the global ray was blocked by
  Bond's own collision standing next to the eye (hits at t 0.05–0.3 with capsule normals, [D
  probe dump]). `Vision_InView` consults no collision at all in the game; the ray is the port's
  unocclusion guard only.
- The ray stops 0.5 units short of the target: the collision meshes run up to ~0.3 units in
  front of the render glist bboxes [D trench probe: node 40's face at 0.27], so a full-length
  ray always "hits" the very cel it is trying to clear.

Result at the trench spot: the shipped mask (walk + rescue) keeps 727+ World draws from the eye
cel 14, and the sweep's criterion holds with zero failures.

## The sweep (task item 2)

`sweep_every_level_near_los_never_culled` — every on-foot level on the disc (27 with portals),
the eye at the loaded spawn and at a grid along the nav nodes (capped 150/level; 3020 spots),
4 yaw directions each = **12080 spot-directions**. Assertion: every generic cel the frustum
alone keeps (`Vision_InView`), within 30 units of the eye with clear static LOS, is kept by the
shipped mask. **0 failures on all 27 levels.** The raw walk's failures are pinned as counts
(they document the causes; the rescue covers them all):

```
07000001: 138   07000002: 850   07000003: 372   07000005: 1382  07000006: 388
07000007: 770   07000008: 392   07000009: 258   0700000a: 4706  0700000b: 1487
0700000c: 900   0700000d: 191   07000011: 3195  07000012: 1466  07000013: 40
07000014: 981   07000015: 345   07000016: 2589  0700004a: 600
```

(0700001b/43/48/49 have no portals — skipped; 41/46/4b: 0 failures.)

## Ground truth kept (task item 3)

`ground_truth_spots_stay_put`:

- 07000006 courtyard `7.24,42.03,-39.31,10.161,0.428`: eye cel 33 (the ground-truth cell), the
  rescue adds nothing — the masonry verdict is unchanged (pinned further by `fix_portal3_live`).
- 07000006 start: eye cel 29, rescue 0.
- 07000007 ballroom window `61.11,44.12,-52.58,-2.541,-0.010`: **the rescue restores nothing** —
  the lit building stays hidden exactly as the game hides it.
- 07000007 clock hall `34.25,44.13,-70.71,1.549,-0.070`: eye cel 33, the grandfather clock
  (cell 50) and the in-frustum shields (45/46 + overlays 269/270) stay visible
  (`fix_portal6_live`'s pinned set); the rescue adds one near prop (cell 94) — additive, and the
  game shows the hall's props.

## Per-level fallback (task item 4)

Not needed this session: the rescue cleared the sweep on every level, so no level renders with
holes and the fallback list would be empty. If a future level regresses, the mechanism is a
one-line level-id check in `cel_visibility` forcing the `(None, None)` frustum-only path; the
sweep prints the per-level raw counts to drive that list.

## Files

- `crates/nf-view/src/scene.rs` — `los_ray_clear`, `near_los_rescue` (+ docs).
- `crates/nf-view/src/gpu.rs` — `cel_visibility` runs the rescue on the game mask (NF_PORTAL_DEBUG prints the count).
- `crates/nf-game/src/world.rs` — `segment_hit_static` (shared file, minimal hook, documented).
- `crates/nf-view/tests/portal_sweep_live.rs` — the trench probe, the 27-level sweep, the four ground-truth spots.

## Verification

- `cargo test --offline --release -p nf-view --test portal_sweep_live`: 3 passed (19 s).
- Full suite (`tools/test/suite.py`, retail image): see the WORKLOG entry for the final total line.

## What's left

- The root cause is the eye-cel tie-break's collision attribution (`build_FindCel` 0x1c8cb0's
  per-cel object lists). The rescue makes the port correct at every near spot regardless; making
  the tie-break itself faithful would need per-cel object attribution in the port's collision
  world (the same gap the 06 courtyard note records).
- The strict-mask side (rooms/objects of unvisited rooms) is untouched by design; if a level
  ever shows hidden-room OBJECTS leaking, that is a different rule (`View_DrawObjList`
  0x1e69a0), not this one.

## Live checks for the main agent

1. **The trench eye cel (the root cause, [L])**: in the GDB-server PCSX2, teleport Bond to
   `16.67, 30.04, 58.60` (yaw 1.755), hold 120 frames, then read the main viewer's cel —
   `Vision_Portal_Recurse` 0x1e86e8's `*(int *)(viewer+4)` (the 06 session had the main viewer
   at 0x01fc6c10; re-find it the same way). Expected if the attribution theory is right: the
   cel pointer is the pCelList record at `pCelList + 15*0x50` (cell 15's record), not +14*0x50.
   Also read `VisCelCount` (u16 0x30d0bc) and the VisCelList (0x34b3b0) — expected to hold the
   trench rooms (the port's 15/24 equivalents), NOT the two-cel list the courtyard spot showed.
2. **The trench walls draw [L]**: a `pcsx2_shot.py --pos 16.67 30.04 58.60 --yaw 1.755 --pitch
   -0.001 --wait 120 --god` render of the real game at the trench. Expected: the stone trench
   walls around Bond draw solid (no see-through to the distant cliff), matching
   `--bond 16.67,30.04,58.60,1.755,-0.001` on the port with this branch. Also the wall-hug
   marker (red/yellow) near grapple point #282.
3. **Per-cel attribution spot check [L]**: with the same session, `gdbcall.py` on
   `Collide_RayIntersect` (0x1ec690) arguments during a `Camera_CheckLocation` (0x1bae90) call at
   the trench — which cel's object list holds the floor poly directly under (16.67, 58.6):
   expected room 15's record, not the outdoor box's. This pins which side of the tie-break the
   game's data puts the floor on.
4. **Port renders to eyeball** (`--bond x,y,z,yaw,pitch`): the trench spot itself; the 06
   courtyard `7.24,42.03,-39.31,10.161,0.428`; the 07 window `61.11,44.12,-52.58,-2.541,-0.010`
   (the building must NOT draw); the 07 clock hall `34.25,44.13,-70.71,1.549,-0.070`.
