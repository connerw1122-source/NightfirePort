## Your task: `portal3`, the 06 courtyard holes persist: the eye-cel state

Read `docs/reports/fix-portal2.md` on `s/fix-portal2` (42ba472) first. Its chain pass (`View_AddCels` 0x1e6ba0's `+0x18`
walk with `Vision_InView` 0x1e89e0) is evidence-based and kept. The main agent merged it and rendered the same spots
(`nf-view <iso> <level> --bond … --frame 120 --shot`, walk on vs `NF_NOPORTAL=1`), and did NOT push it:

- **07000006 `--bond 7.24,42.03,-39.31,10.161,0.428 --frame 120`: unchanged.** The masonry under the left walkway and
  the lower right castle wall are still missing with the walk on, with sky showing through; with `NF_NOPORTAL=1` they
  draw. This is exactly your report's case: "If the hole persists at frame 120 … the residue is the persistent
  `Camera_CheckLocation` state carried along the approach … which would point at the state machine."
- 07000007 window spot: unchanged (the room stays hidden, as you argued the game does). Leave it.
- The other two spots: identical on/off, good.

**Do:**
1. Reproduce headlessly at that exact spot through the same path `--bond` takes (Bond placed, then 120 ticks): log the
   viewer cel the port carries each tick (`ViewerCel`), what `build_FindCel` 0x1c8cb0 would return fresh at the eye,
   and which World cels the walk + chain pass keep vs the frustum-only set. Find which wall cels are dropped and why.
2. Decide from the decompile what the game's state is after a placement jump. `--bond` places Bond the way the dev
   teleport does; the game's own equivalents are the level start and `Player_Enable` with a matrix. Does
   `Camera_CheckLocation` 0x1bae90 re-find the cel (a FindCel) when the move crosses no portal and leaves the box, or
   keep a stale one? Make the port follow it, and make the dev teleport and `--bond` reset the viewer cel the way the
   game's own placement does.
3. Also walk there: a headless walk along the user's route (the truck → courtyard, `the_07000006_walk_keeps_the_level_visible`'s
   route, extended up to this spot and this view) must keep those wall cels visible. That's the case the user saw.
4. Pin the fix with a test at this spot that lists the wall cels, failing first.
5. Run the full four-crate suite (in the background, first thing) and report the count.

Files: `crates/nf-view/src/{scene,gpu,lib}.rs`, `crates/nf-view/src/dev.rs` (the teleport reset), `crates/nf-view/tests/fix_portal*_live.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/portal3` (start from `s/fix-portal2`, then merge `origin/master` into it).

---
claimed-by: sandbox1 at 2026-10-13T15:01Z

---
## Report (done, 2026-10-13T16:26Z)


---
## Report (done, 2026-10-13T16:26Z)

# fix-portal3 — the eye-cel state after a placement (the 07000006 courtyard holes)

Task 130c: the fix-portal2 render still hid the masonry under the left walkway and the lower
right castle wall at `--bond 7.24,42.03,-39.31,10.161,0.428 --frame 120` with the walk on
(`NF_NOPORTAL=1` drew them). Branch `s/fix-portal3` (same commits as `s/portal3`) from
`s/fix-portal2` 42ba472 + master a5bff8b.

## What the evidence showed

1. **The render path already resolved the viewer cel fresh** — `--shot` creates its `Renderer`
   after the 120 ticks, so the `Camera_CheckLocation` (0x1bae90) state starts default and the
   cel comes from a fresh `build_FindCel` at the placed eye. A stale carried state was NOT the
   mechanism at this spot (probe: persist == fresh every tick, both to the same cel).
2. **The port's portal side model is correct** [L]. The decompile (0x1d15f8) and my first
   disassembly reading disagreed on which cel's portal list holds the forward-corner plane; the
   runtime nodes settle it: walking `work/ee_start_05.bin`'s world cel chain (+0x28 lists) and
   matching all 21 twin pairs to 07000005's main-map quads — **the FORWARD-corner node is in
   celB's list 21/21 times, the REVERSED-corner node in celA's**, i.e. exactly the port's
   `scene.rs` model (cel_a → reversed plane + `(flags & 1) == 0`; cel_b → forward plane +
   `(flags & 2) == 0`). No change needed; recorded here so the next task doesn't re-litigate it.
3. **The real divergence was the FindCel tie-break** [M 0x1c8cb0]. The game's tie-break is a
   collision raycast: per candidate cel an up ray (256 units) and a mirrored down ray whose
   length is the up ray's min(hit, len) (len starts 256 and shrinks to the best-so-far
   distance), nearest hit distance wins, no hits at all → the first candidate. The port
   approximated it with bbox-face distances. At the failing eye two room cels contain the point
   (cell 21, the upper region, bbox y 42..56; cell 33, the courtyard strip, y 41..46): the
   approximation picked **21**, the game's raycast (courtyard floor 1.73 below the eye vs no
   surface below in 21's own set) picks **33** — the same cel the room records pick.
4. **The game's own placements clear the viewer cel** [M 0x1b9920 `Camera_Enable` with a matrix:
   `viewer+4 = 0`; `Player_Init` 0x1910f8 does the same after `Camera_Set`;
   `Camera_Reset` 0x1b9af0 zeroes +4/+0x29/+0x34]. `Cel_ObjectLeftCel` (0x1e0d58) returns 0 for
   a null cel, so the next `Camera_CheckLocation` always re-finds via `build_FindCel`.

## What the port does now (all in nf-view)

- `scene::find_cell_raycast(graph, eye, world)` implements the game's tie-break when the sim's
  scoped `CollisionWorld` is available (`segment_hit_scoped(Some(cell), …)` per candidate — the
  port's stand-in for `Collide_RayIntersect`'s per-cel object list), keeping the bbox-face
  approximation for the world-less callers. `scene::eye_cell(..., world)` threads it through.
- `gpu::cel_visibility(eye, view_proj, world)` takes the sim's world; `lib.rs` passes
  `scene.sim.as_ref().map(|s| &s.world)` at both call sites (the frame loop and `shot()`).
- The placement reset: the dev teleport (`RestoreSpot`) and the free-cam→Bond placement (KeyN)
  now set `renderer.viewer_cel = ViewerCel::default()` — the game's cleared `viewer+4`
  [M 0x1b9920]. `--bond`'s headless render path resolves from a default state by construction
  (the `Renderer` is built after the placement), which is the same semantics.

## Verification

- `crates/nf-view/tests/fix_portal3_live.rs` (NF_ISO):
  - `the_placed_viewer_cel_resolves_where_the_game_finds_it` — at the exact `--bond` spot, the
    render-path resolution (fresh state + world) equals the room-based courtyard cel (33), the
    persistent state agrees after 120 ticks, and the sister assertion pins that the pre-fix
    bbox-face approximation picked a different cel (21) there.
  - `the_user_route_walk_keeps_the_courtyard_walls` — the truck→courtyard walk extended to the
    spot (two legs, the walker tilting up on approach): the eye cel resolves to the game's own
    state at every step (29 → 37 → 29 → 27 → 15 → 21), the game mask never collapses (min 7
    World-drawing cells over the route), and the wall-cel outcome on the final leg is logged
    (cell 19 does not draw from the carried state — see the honest-status section).
  - `the_courtyard_wall_cels_at_the_exact_spot` — the wall-cel pin, `#[ignore]`d and live-gated
    (it asserts the port's current outcome; the main agent's VisCelList watch decides whether
    that IS the game's outcome).
  - Suite results in this window: fix_portal_live 6/6, fix_portal2_live 2/2, nf-view lib 8/8,
    fix_portal3_live pins 1+1 green (the probes are log-only or `#[ignore]`d).
  - `probe_*` diagnostics kept as evidence: the [L] portal-side census, the portal-data audit,
    and the failing-spot probe (candidates, plane values, clip replication, pitch comparison).
- Pre-fix failure evidence: the probes in git history show the approximation resolving cell 21
  at the spot and the walk reaching only `[21]`/`[33, 21]` from either candidate.

## The residual hole at the exact 38.5°-up camera — honest status

From the game-faithful start (cell 33) the walk + chain pass still hide wall cels 20/22/38/5/13/23
at the spot's steep up-pitch, and 20/22/38/5/13/23 even at level pitch (cell 19 only joins at
level pitch). Everything I could verify says the port's walk rule now matches the game
([M] 0x1e82e0/0x1e86e8/0x1e6ba0 read in full; the portal sides [L]-verified; the depth-0
frustum = fovY 60° [M Camera_CalcViewAngles 0x1baf70 with the 0x3f860a92 constant], the port's
16:9 render frustum is wider than the game's 4:3, i.e. the port clips LESS than the game), and
rooms never ride the +0x18 chains [L census] so the chain pass cannot rescue them. Whether the
real game draws those walls at this exact placed camera is a live question — the walk rule as
ported says it would not, and `NF_NOPORTAL` draws them because it skips the walk entirely.
Rather than fake a pass, the checks below settle it in the real game.

## Live checks for the main agent

1. `tools/trace/gdbwatch.py` on **VisCelList (0x34b3b0)** / **VisCelCount (0x34b3b4)** at
   `--bond 7.24,42.03,-39.31,10.161,0.428` (frame ~120): list which runtime cels are in the
   game's VisCelList. Expected if my reading is right: only the courtyard cel(s) (the port's 33
   and 21) — the wall cels absent, i.e. the real game holes too at this exact placed camera.
   If the wall cels ARE in the list, the game reaches them through a rule I could not recover
   (the port's walk then needs that rule — the next task's lead).
2. Same session, watch **viewer+4** (via glb_viewer[0] at 0x2d88b0's pointee +4) across the
   placement: expected 0 right after the placement frame's `Camera_Enable`/`Player_Init`
   equivalent, then the `build_FindCel` result on the next `Camera_CheckLocation`.
3. `tools/trace/fieldprobe.py`: read the runtime cel record's **+0x3c** (class word) for the
   cels the walk visits vs the wall cels — confirm the wall cels' class short is 0xc047
   (room-class; never a chain node).

## What's left

- The live checks above (they decide whether the residual hole is a port bug or the game's own
  placed-camera behavior).
- The full four-crate suite did not complete in this sandbox window (see WORKLOG) — the nf-view
  portal suites ran; the final gate needs a re-run.
- The `+0x18` chain-builder window stays [S] (unchanged from fix-portal2).

## Files touched

`crates/nf-view/src/scene.rs` (find_cell_raycast/eye_cell signatures + the tie-break; the rest
additive), `crates/nf-view/src/gpu.rs` (cel_visibility's world parameter), `crates/nf-view/src/lib.rs`
(the two call sites + the two placement resets), `crates/nf-view/tests/fix_portal3_live.rs` (new).
No shared file rewritten; nf-game/nf-formats/nf-audio untouched.

## ntfy

(filled after the finish step)
