# cover-nodes — the AI cover node placements (0xE5 corner, 0xE6 low)

Task 746. Branch `s/cover-nodes`. The census (task 709) listed the cover node placements as
"the port does not act on" — 968 disc-wide (0xE5 × 584, 0xE6 × 384). Since then `cover.rs` had
landed (232f5392: nodes, usability, FindCover, the UnderCover states) and the nodes already
spawned — **from the main map only**. The gap this task closes: the game creates the nodes from
EVERY map entry of the level archive, and the port now does the same.

## What the game does [M]

- The loader dispatches each type-0/1/2/11 entry of the level `.bin` to `parsemap_parsemap`
  (0x1d4880 case 0/1/2/0xb — docs/formats/level-bin.md, *measured*); the main map (type 1) is one
  entry among them. The block walk runs `parsemap_block_map_data_dynamic` 0x1d1b68 per placement
  and `parsemap_create_dynamic_objects` 0x1d04f0 case 0xe5 → `Drone_CoverCornerNode` 0x136ff0 /
  case 0xe6 → `Drone_CoverLowNode` 0x137340. Node order = maps in directory order × placements in
  record order.
- The disc data backs the counts: every 0xE5/0xE6 placement's flags word is exactly 0x000000e5 /
  0x000000e6 (584 + 384 records across the 07* archives, probed through `placement::parse`).
- Every mission level's shared sub-map **010000f3** carries one corner + one low template pair
  ("Cover Corner Node" / "Cover Low Node", placement translation (0,0,0), no parameters). Through
  the create functions' default arms: corner flags 0xC|0x60|0x180 = 0x1EC, angle 45°
  (0x3f490fdb = π/4), depth/offset bytes 8 × 0.1 [M 0x136ff0:44-101]; low idle 0x11 / aim 0xA, no
  flags [M 0x137340:65-87].
- Levels with NO main-map cover placements — 07000004, 09, 0a, 1b, 23, 25, 26, 27, 28, 4a — get
  their whole cover graph from that pair. Disc totals: 902 main-map + 66 sub-map nodes.

## The port change

`cover::build` (crates/nf-game/src/cover.rs) walks `level.maps` in order instead of the main
map's placements; call sites: nf-view scene load (scene.rs `Scene::build`) and nf-extract's
`covers`/`route` commands. No RNG draws added, nothing else touched.

- 07000006 now pins **81 corner + 35 low** (was 114 = main map's 80+34; the sub-map pair completes
  the census's "81+35").
- 07000004 (and the other nine levels above) go from 0 nodes to the game's 2.

## Verification (fail-first, real path)

`crates/nf-view/tests/cover_nodes_live.rs` (opt-in `NF_ISO`), through the real load
(`nf_view::load` → `Scene::build` → `cover::build`) and the real sim:

1. `cover_06_spawns_every_map_s_nodes` — 81 + 35 [D placement bytes; M 0x1d4880 → 0x1d04f0].
2. `cover_prefab_template_pair_spawns_from_the_shared_sub_map` — 07000004's pair with the
   decompile's default-arm values [M 0x136ff0, 0x137340; D placement bytes].
3. `a_06_guard_under_fire_takes_a_game_cover_node` — Bond stands grounded, in front of a
   prop-0x30 guard with a reachable, usable corner node; synthetic gunfire ([S] trigger only)
   drives the real heard-noise → sighting → combat chain; the guard enters
   `COMBAT_OUT_OF_RANGE 0x5a` → `RUN_FOR_COVER 0x8c` and claims one of `sim.covers` (observed:
   llist #12 claimed node 63), then dwells in `UnderCoverIdle 0x94` within 1.5 of his in-cover
   point [M docs/systems/drone-cover.md §States — 0x8C entry from 0x5A/0x5C/0x5F; 0x94's
   "> 1.5 from the goal → leave"].

**Fail-first proven**: with the change stashed (tests present, main-map-only build), all three
fail (114 ≠ 116; 07000004 = 0; no guard covered). With the change applied, 3/3 pass
(`cargo test --offline --release -p nf-view --test cover_nodes_live`).

## Stale pin fixed (test was wrong)

`crates/nf-formats/tests/script_census.rs` still listed 0xE5/0xE6 among "the 18
unhandled-or-ignored types" and pinned `HANDLED_PLACEMENT_TYPES = 56` — a contradiction with the
code since 232f5392 (the census pin predates the cover work; nothing cross-checked the constant
against the code). Fixed: the two kinds moved to the handled side with their disc counts
(584/384) kept as pins, constant → 58 ("50 sim + 8 draw-only"), unhandled sum 1537 → 569. The
decompile-backed disc totals were never weakened. `docs/systems/script-census.md` and
`docs/systems/drone-cover.md` updated to match (the census doc's "does not act on" table and the
Top-10 item 1 marked done).

## What I ran

- `cargo test --offline --release -p nf-view --test cover_nodes_live` — 3/3 pass (final tree).
- The stash/pop fail-first run of the same test (3/3 fail → 3/3 pass).
- `nf-extract covers <iso> 07000006.bin` — 114 nodes before, 116 after (the tool's own listing).
- The full suite (`tools/test/suite.py`, NF_ISO set) was started on the final tree (= origin/master
  bee8c129 + this task; master had not moved) — see the suite line below for how far it got
  before this session's deadline cut it off.

## Suite result (final tree)

See `suite.out` tail below — filled in at writing time with whatever the run had produced.

## What's left

- The full-suite total line for the final tree if the deadline cut the run short — the next
  session should re-run `NF_ISO=<iso> python3 tools/test/suite.py` and report the total.

## Live checks for the main agent

- **CoverNodes count at load [M 0x136ff0:105 / 0x137340:114]**: break on the second
  `Drone_CoverCornerNode` call after loading 07000006 and read `DAT_00299104` (the count,
  0x299104): expected 2 while the 010000f3 template pair parses (the sub-map precedes the main
  map in the archive), and 116 after the main map finishes. That directly verifies directory
  order and the all-maps scan.
- **Template pair position [D]**: the two 010000f3 nodes sit at the origin (0,0,0) — a
  fieldprobe of `CoverNodes[0]+0x20` (0x283E10 + 0x20) right after the sub-map parse should read
  0.0/0.0/0.0.
- **Optional visual**: the guard cover play is already exercised by the live check above; if a
  render is wanted, `--bond 49.35,39.0,12.28,0,0` on 07000006 (the test's Bond spot) should show
  the courtyard guard #12 breaking to cover under fire instead of standing in the open.

## Usage

not visible to me
