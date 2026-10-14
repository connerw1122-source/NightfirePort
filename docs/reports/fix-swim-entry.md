# fix-swim-entry — the swim entry rides the game's stateful player cel relink

Branch `s/fix-swim-entry` (from origin/master 6691ef9; merged origin/master mid-task — that merge brought
fix-08-collision2's work on the same relink line, resolved in its favour where its traced pin is the
tighter read, see the deviation below). Task `103d-fix-swim-entry`. Time note: suite.py cannot survive the
background in this sandbox (third session running); the verification was foreground chunks.

## What the game does (the evidence chain)

`Player_InWater` 0x195e50 [M] reads the surface off the player's RUNTIME cel: `*(obj+0x20)+0x90`, entering
the swim when `obj+0x34 − 0.3 < cel+0x90`. That cel is stateful, maintained by the movement handler's
moved pass, which runs AFTER `Player_Update` in the frame [M 0x1336f8:147-236], then copies
`obj+0x40 ← obj+0x30` [M 0x1336f8:212] — so the cel the InWater test reads on tick t was linked from the
move segment of tick t−1:

1. `control_handle_cel_change` 0x133288 [M]: the move segment `obj+0x40 → obj+0x30` against the CURRENT
   cel's portals — `Cel_ObjectLeftCel` 0x1e0d58 (two `Collide_RayTriangle` per portal quad, a crossing
   relinks to the portal's other cel and wins even while the new position is still inside the old bbox);
   else `build_point_in_bounding_box` 0x1c9500 keeps the cel while pos stays inside its box; else
2. `build_LinkToRoom` 0x1c9680 → `build_FindCel` 0x1c8cb0 [M]: the containing ROOM cels only
   (`cel+0x3c & 0x40000`, cap 0x40), one candidate returns directly, several go to the tie-break — per
   candidate an up ray (len 256 first, then the best-so-far distance) and a down ray (len
   min(up hit, len)), both scoped to the candidate's OWN geometry, the nearest hit distance wins, no hits
   at all → the first candidate stands, a miss leaves the object unlinked. The game passes the
   WORLD-SPHERE CENTRE `obj+0x80` as the search point [M 0x1c9680:41-45] (`Control_BuildWorldSph`
   0x133070; local centre (−0.1347, −0.1267, 0.2402) [L ee_start_05.bin M_CONTROL+0x80]).
3. A placement/teleport (`Player_StandAtNewPosition` 0x190d88) resets `obj+0x40` to the new position
   [M 0x190d88:46] and relinks directly with one `build_FindCel` [M 0x190d88:108-117] — no portal walk
   over the jump.

The port had NONE of this: a stateless smallest-containing-bbox lookup at pos every tick
(`find_cel`, nf-view scene.rs), feeding both `LadderCtx.water` and `bond.cel`.

## What was ported (every rule above, cited)

- `crates/nf-game/src/world.rs`: `ScopeGraph.room` (new field, own line — the 0x40000 room-cel filter
  build_FindCel walks); `CollisionWorld::cel_portal_dest` (0x1e0d58), `cel_contains` (0x1c9500),
  `find_cel` (0x1c8cb0 with the tie-break over `segment_hit_cell` — the candidate's own buckets),
  `player_cel_relink` (0x133288).
- `crates/nf-game/src/player.rs`: `Bond::prev_pos` (obj+0x40, own line), initialised to the spawn pos.
- `crates/nf-view/src/scene.rs` (shared file, small additive hooks): `scope_graph` fills `room`;
  Sim::run replaces the two stateless `cel_at` reads with one relink before `tick_ctx` (the water surface
  = the relinked cel's room cel's +0x90; `bond.cel` = the relink) and copies `prev_pos` after the tick
  [M 0x1336f8:212's order]. A >3.0-per-axis jump in the segment is treated as a placement: prev_pos
  resets and the relink is one `find_cel` [M 0x190d88:46,108-117; the 3.0 bound is [S] — above any
  per-tick move the sim produces, below every placement jump].

**Deviation [S], cited:** the game's `build_LinkToRoom` searches at the world-sphere centre (obj+0x80);
the port searches at POS. Reason: the port's per-cell bucket attribution (`build_scoped`) is coarser
than the game's per-cel chains, and at the 07000001 seam the 0.1267-shorter down-ray flips the tie-break
to a root whose bucket misses the floor under Bond — fix-08-collision2's traced 01 pin (kept green here).
`find_cel` itself (candidates, cap, rays, tie-break, no-hit rule) is the game's.

## Verification — and an honest negative

The reported symptom numbers reproduce: the bank walk (`fix_ladder2_live::swim_repro…`) enters SS_SWIM at
tick 1621, pos.y 3.725 vs surface 3.7675 — the root well under, the "very, very slowly" feel. But on every
path I could construct, the OLD stateless rule and the game's stateful rule COINCIDE, so the fail-first
run does not fail:

1. **The bank fall**: the move segment crosses the pond-top portal (91↔184, the plane y 3.765 [D probe])
   in the same tick the position leaves the land cel's bbox — the entry tick is 1621 both ways (measured;
   also the stateless ctx snapshot and the game's one-tick-old cel read agree, both being "the previous
   move segment").
2. **The wade band** (position above the water cel's bbox top 3.7675, sphere centre inside it): the
   position lookup also reads the land cel there (the water bbox excludes the POSITION), so a spawn in
   the band walks in both.
3. **The tie-break spawn band**: the game's tie-break hands the {land, water} candidate pair to the LAND
   cel (the water cel's mat-14 surface plane is up-facing — an up-ray cannot hit it one-sided
   [M 0x10a8c8] — and its scope has no plane under the slope column), and the position lookup reads the
   land cel too.

The fail-first proof run: `git stash` of the three src files → both new tests PASS on the pre-fix tree →
no behavioral divergence found on this level's geometry. The work stays because it IS the game's mechanism
(every rule cited), it removes the divergence class wholesale (the stateful keep vs the stateless snap,
portal crossings, the tie-break fallback, the placement reset), and fix-08-collision2 (merged during this
session) independently landed the FindCel-tie-break half on master — this task adds the stateful half it
explicitly left out ("the portal-centre crossings — the other half of this task", its comment).

- `crates/nf-view/tests/fix_swim_entry_live.rs` (NF_ISO, 2/2): the bank walk enters the swim ON the game's
  relink (the entry tick's runtime cel IS the water cell; the InWater inequality; the swim holds mid-pond
  with the cel state); the fresh-link semantics (the wade band links the land cel — no swim; the deep
  spawn links the water cel — swim; the dry bank stays walk). Sister assertions in both.
- Probe kept: `crates/nf-view/examples/probe_swim_entry.rs` (the per-tick pick comparison + the column
  census that produced the [D] numbers above).

Suite on the final tree (origin/master merged at 9462af4 + the deviation commit 2d9572d): **nf-game 143/143;
the nf-view swim/ladder/walk/collision families 47/47** (fix_ladder2_live 4/4, fix_swim_entry_live 2/2,
fix_swim_vent_live 2/2, fix_swim_vent5_live 2/2, walkreal_probe3_live 2/2, fix_08_collision2_live 2/2,
+ the 35-test nf-view bucket). The remaining nf-view/nf-formats/nf-audio families did not fit the window
after the mid-task master merge; the merge-resolved tree is pushed so the next session can run
`tools/test/suite.py` first thing. No RNG draws added; no wall-clock in Sim.

## What's left / live checks for the main agent

1. **The real entry depth** (the open question this task could not settle headless): in PCSX2 at the 02
   bank (`--bond -82.3,5.3,32.05,1.57`, walk in slowly AND walk off the edge), with gdbwatch on the
   player's `obj+0xF6` (substate), `obj+0x34` (pos.y), `obj+0x20` (cel) and `cel+0x90`: report the tick
   where substate flips to 3 relative to pos.y. If the real game enters when pos.y − 0.3 first dips under
   3.7675 (the surface), the port now matches by construction; if it enters EARLIER (shallower), the
   remaining delta is not the cel pick (it is now the game's) but something upstream — candidates: the
   walk root height cc, or the InWater read being cel-fresh one tick earlier in the real frame order.
2. **nf-view render**: `--bond -82.3,5.3,32.05,1.57` walking forward — the swim should start at the pond
   edge as before (no visual change expected on this path); `--bond -75.4,3.82,32.05` (a wade-band
   placement) should show Bond WADING on the submerged slope, not swimming.
3. The full four-crate suite on this merged tree (the next session's first run).
