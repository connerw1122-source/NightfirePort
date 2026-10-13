# fix-portal — the portal visibility walk fixed and back on by default

Task 130: the portal walk (`fix-perf-level`, `scene::portal_visible_cels`) hid the whole level on
07000006 (the user's empty night sky over water), and on 07000007 two partygoers inside a
portal-hidden room still drew in the snow. The walk is the default again; `NF_NOPORTAL` disables it.

## What was wrong, and the game rule for each

All four causes in the task list were investigated in the decompile; three were real, one was
measured harmless, and the graph itself turned out fine (44 portals, 0 dropped endpoints —
`the_portal_graph_planes_point_into_cel_a` still passes).

1. **The eye-cel choice** — the real collapse enabler. The port re-picked the walk's start cel
   every frame with a bbox-containment smallest-volume tie-break. The game carries the viewer's cel
   across frames (`viewer+4`) and updates it by rule, in order [M `Camera_CheckLocation` 0x1bae90]:
   - `Cel_ObjectLeftCel` (0x1e0d58) raycasts the frame's move against the current cel's portals
     (two ray-triangle tests on the quad corners +0xc/+0x10/+0x14 and +0x14/+0x18/+0xc) and relinks
     directly to the crossed portal's destination;
   - else `build_point_in_bounding_box` (0x1c9500) keeps the cel while the eye stays in its box;
   - else `build_FindCel` (0x1c8cb0) searches: the containing room cels (the runtime cel's +0x3c
     flags bit 0x40000, cap 0x40), one → it, several → the vertical-raycast tie-break (a ray up to
     256 units and its mirrored down ray; nearest hit wins; **no hit → the FIRST candidate**, the
     0x1c8cb0 tail), none → 0;
   - a FindCel miss KEEPS the previous cel, and a first-ever miss takes the world's first cel
     (`puVar1[1] = *glb_world`) [M 0x1bae90:34-38].
   Ported as `scene::ViewerCel` + `scene::eye_cell` (+ `portal_move_dest`, `find_cell_raycast`).
   The raycast's per-candidate collision geometry is not in the port's cel graph, so the rays test
   the candidate's own bbox faces (the ceiling/floor the game's ray effectively finds for a room) —
   documented as the port's approximation; a live check below can settle it.

2. **The runtime cel's flags word** — a data bug found on the way. `CelNode.flags` was taken from
   the placement's raw flags word, but the game DERIVES the runtime cel's `+0x3c` per class:
   0xc023 → **0x804c023**, 0xc047 → **0x804c047** (bit 0x40000 = the room bit `build_FindCel`
   walks — the raw 0xc047 does NOT carry it), the generics → 0x8019 [M `parseentity_fixup_entity`
   0x1d00c0:106/133]. Without it the fresh-search found no room cels at all and everything fell
   back to the world's first cel.

3. **The depth-0 plane convention** — ported exactly. Disassembling `Vision_Calculate_View_Fustrum`
   (0x1e7df8) shows the near plane is built through quad corners (c1,c3,c2) whose normal points
   BACK at the eye — it keeps `fw·p ≤ fw·eye + 200`, clipping every portal polygon beyond 200 units
   (the fog range; `View_AddCels`' inner nodes still draw frustum-only, which is how the game fills
   the far view). The port passed a Gribb-Hartmann near plane (keep-everything-in-front) instead.
   The four side planes are the same half-spaces either way. `gpu.rs::depth0_game`.

4. **The 0x40 VisCelCount cap** — ported [M `Vision_Recurse` 0x1e82e0 / `Vision_Portal_Recurse`
   0x1e86e8]: the insert is guarded by `VisCelCount < 0x40`; the recursion continues past it.

5. **Doors** — left conservative on purpose: the port treats every portal open, which can only
   KEEP cels visible (never hide); the game skips closed-door portals [M 0x1e82e0
   `Door_IsClosed(portal+0x0C)`]. Linking doors would need `build_LinkDoors2Portals`' quad match;
   noted as follow-up, not a collapse cause (it hides, never empties).

6. **The object half of the cull** (the second symptom) — ported [M `View_DrawObjList` 0x1e69a0 /
   `View_AddObjects` 0x1e62d8: only the visible cels' object lists draw; each object's cel via
   `build_LinkToRoom` 0x1c9680 → `build_FindCel`]: `Scene::skin_palette` now takes the frame's
   room-visibility mask and collapses the palette of posed objects (characters, doors, pickups,
   searchlights) whose room cel the walk hides — the port's established "not drawn" state (the same
   mechanism as `fs.hidden`). Objects the cel pass cannot place keep drawing (the game's unlinked
   objects draw from the forced list, `View_AddForcedObjects` 0x1e5fb8).

## Reproduce-before-fix evidence

The headless walk (`crates/nf-view/tests/fix_portal_live.rs`, all NF_ISO, all green):

- **07000006 truck → courtyard** (the user's route): the OLD code dropped to 117 submitted draws at
  t=0.70 (the frustum saw ~4400) and sat at 342-351 in the courtyard — the near-empty night-sky
  state. The FIXED walk pins **min 104 draws** along the route and the viewer cel following
  29 → 37 (`the_07000006_walk_keeps_the_level_visible`).
- **The castle door** opens: the walk crosses 29 ↔ 34 and keeps the castle hall
  (`the_walk_propagates_through_the_castle_door`).
- **07000007 ballroom window** (`--bond 61.11,44.12,-52.58,-2.541,-0.010`): the hidden room's World
  draws are **0 with the walk on / 543 with it off**, and **9 partygoers** inside walk-hidden rooms
  collapse with the walk on and stand with it off
  (`the_hidden_ballroom_room_and_its_partygoers_are_skipped`) — the user's two are among them.
- **07000007 start view**: the fixed walk submits **exactly 177 draws — the number fix-perf-level
  measured from the game on master** (4526 → 177). A strong cross-check that the walk, the depth-0
  planes and the eye-cel rule now match the game (`the_walk_holds_on_the_07000005_and_07000007_routes`).
- **07000005 truck-deck walk**: min 1058 draws, the eye's room never culls.

## Verification

- `NF_ISO=... cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view
  -- --test-threads=1`: green except the 3 allowed slim-ISO golden tests (bigf/disc manifest, in
  nf-formats — untouched by this task).
- The 07000007 start view hitting the game-measured 177 is the strongest headless evidence; the
  real-window check is listed below for the main agent.

## Shared files touched (all minimal, additive)

- `crates/nf-view/src/gpu.rs`: `cel_visibility` (one walk per frame, shared by palette + render),
  `depth0_game`, `Renderer.viewer_cel/last_vis`; the cull reuses the stored mask; NF_PORTAL →
  NF_NOPORTAL (default flipped).
- `crates/nf-view/src/lib.rs`: the frame loop computes `draw_cam`/vp before the palette and calls
  `cel_visibility` once; `shot()`/bench paths updated.
- `crates/nf-view/src/scene.rs`: `ViewerCel`/`eye_cell`/`portal_move_dest`/`find_cell_raycast`
  (new), `CelNode.flags` (own line) + `cel_flags_word`, the 0x40 cap, `portal_*_from_cell`
  variants (the room-index originals untouched for the older tests), `skin_palette`'s `vis` param.
- `crates/nf-view/tests/gpu_skin_live.rs`: the `skin_palette` call sites gained `None`.
- `crates/nf-formats/src/level.rs`: untouched (the task named it; no parse change was needed — the
  portal block already parses cleanly, `dropped_portals == 0`).

## What's left / follow-ups

- The FindCel raycast tie-break uses bbox faces, not per-cel collision geometry (no collision in
  the graph); a live check can confirm the game's choice on an overlapping-bbox spot.
- Door-portal linking (`build_LinkDoors2Portals`) so closed doors stop the walk (conservative today).
- `posed_vertices` (the CPU skinning path) does not take the mask yet; the GPU palette path (what
  nf-view renders with) does.
- Attachments follow their owner's collapse only through their own palette slots; a hidden owner's
  attachment slots are not explicitly FAR'd (the existing `fs.hidden` path has the same shape).

## Live checks for the main agent

1. `tools/trace/gdbwatch.py` on **VisCelCount (0x34b3b0)** and **VisionRecurseCount**, standing in
   the 07000006 courtyard looking out at the water (Bond at ≈ 83.6, 35.1, 33.2, camera east):
   expected VisCelCount small (the port's fixed walk keeps the courtyard's room alone → its draws
   ≈ 104-350); access type: read per frame.
2. Same watch, standing in the truck area looking at the castle door (Bond ≈ 57.2, 39.7, 22.6,
   yaw ≈ −0.29): expected VisCelCount ≥ 2 (the door portal opens into the castle hall).
3. `tools/trace/fieldprobe.py` poke: place Bond at (77.0, 37.5, 30.6) on 07000006 (the
   overlapping-bbox spot: room 31's gatehouse box and room 32's courtyard box both brush it) and
   read the viewer's cel pointer (`viewer+4`): expected either the gatehouse or the courtyard's
   runtime cel — the port's bbox-face raycast picks by nearest ceiling/floor; this settles which
   rule the real game's per-cel collision raycast applies there.

## ntfy

(filled after the finish step)
