# fix-portal2 — land fix-portal without the wall holes (the `+0x18` chain pass)

Task 130b: fix-portal's walk (merged, then rendered by the main agent) hid geometry the player
can see through — 07000006's masonry under the left walkway and the lower right castle wall
(`--bond 7.24,42.03,-39.31,10.161,0.428 --frame 120`), and the lit building through 07000007's
ballroom window. Branch `s/fix-portal2` (= `s/portal2`, same commits, from `s/fix-portal`
a894d7c + master 8142a7f).

## The game rule the port was missing

The walk (`Vision_Portal_Recurse` 0x1e86e8 → `Vision_Recurse` 0x1e82e0) is only HALF of the
game's static visibility. `View_AddCels` (0x1e6ba0) — which walks the per-frame `cel+0x20` chain
`vision_generate_display_list` (0x1e8848) threads from the sorted VisCelList — has a second pass
the port never implemented:

- For every VisCelList cel it pushes the cel's own glist with the −90000 + 10·k keys (the port had
  this), **then walks the cel's `+0x18` chain of other cels** and, for each node, frustum-tests it
  with `Vision_InView` (0x1e89e0) and pushes it with ±dist² keys into lists 0/1/2 (list by
  `cel+0x94` bits 0x1 / 0x2000) [M 0x1e6ba0:39-91].
- `Vision_InView` is a **4-plane sphere test** against viewer+0x70 — the four side planes only,
  NO near/far planes: visible iff `n·c + d·r + r >= 0` (the SIMD dot of the cel's (centre, radius)
  VECTOR at cel+0x80..0x8c against each (n, d) plane, plus the radius slack `param_1` = the radius
  at +0x8c) [M 0x1e89e0]. This is how the game fills the fog-range far view the depth-0 near
  plane clips out of the portal walk — exactly the driver's candidate (a)+(b) combined: the
  courtyard walls sit beyond 200 units, the walk's depth-0 clip drops their portals, and only the
  chain pass (no near plane) draws them.
- Skip rules [M 0x1e6ba0:29-56,47-56]: nodes/cels whose glist has flag 0x10 at glist+0x44 never
  draw; **0x8018-class nodes are skipped entirely while the viewer's cel is 0xc023-class** (the
  low halves of the derived words; the runtime classes are 0x8017/0x8018/0x8019/0x28019/0x804c023/
  0x1804c023/0x804c047/0x1804c047 [M `parseentity_fixup_entity` 0x1d00c0]).

## What the `+0x18` chains are

Reconstructed from the EE snapshot (work/ee_start_05.bin, file offset = EE address) [L]:

- 137 world cels: 19 × class 0xc047, 113 × 0x8019, 5 × 0xffff. **2606 chain nodes total**, node
  classes ONLY 0x8019 / 0x8008019 (0x8019 | 0x8000000) — room-class cels are never chain nodes.
- Chains tie each cel to generic (0x8018/0x8019) cel placements parsed near it in the same
  map-data block: per-owner counts grow 0..58 as parsing proceeds, with resets (three family
  tails on 05: [11,108,110,116,122], [98,97,107,109,117,144], [57,58,61..88]) — never exceeding 58
  (< the game's 0x40 list caps).
- The builder could not be recovered this session: no decomp or disassembled writer of cel+0x18
  exists (`parsemap_block_map_data_static` 0x1d1998 disassembled end-to-end — it writes
  +0x38/+0x3c/+0x94/+0x60..0x78/+0x80 only; `build_alloc_cel` 0x1c93f8 zeroes +0x18; no other
  function touches it — the store was dropped by the decompiler inside the parse loop).

## What the port does now (all in nf-view; nf-game/nf-formats/nf-audio untouched)

- `scene::parse_cel_graph` builds `CelGraph::chains` (per cell → its chain-node cells) with the
  per-map-block model, nodes = flags16 0x8018/0x8019, **each owner capped at its first 0x40
  nodes** — the cap is [S] (the exact window is unresolved) but evidence-shaped (snapshot max 58;
  without it the whole-block model over-rescues: 814 draws vs the game's 177 on the 07000007
  start view; with it: **180**).
- `CelNode.sphere` — the cel's world (centre, radius) from the placement-transformed bbox corners
  (`parseentity_transform_bounding_box` 0x1d0fb0 [M]; `min`/`max` stay raw — `build_FindCel`
  0x1c8cb0 tests the raw glist bbox). This was a real data bug: the raw bbox put generic cel
  spheres near the world origin.
- `scene::vision_in_view` — the faithful `Vision_InView` formula; `scene::
  portal_visible_cels_game_from_cell` — the strict walk PLUS the chain rescue (with the
  0x8018/0xc023 skip). The strict `portal_visible_cels_from_cell` is unchanged and still gates
  the rooms/objects (`View_DrawObjList` 0x1e69a0 walks the VisCelList cels' object lists; chain
  nodes never enter the VisCelList — `Vision_AddCelToDraw` 0x1e8908 is the only inserter), so
  the 07000007 partygoers stay collapsed.
- `gpu.rs`: `last_cell_vis` (the cell-level game mask) alongside `last_vis` (the room mask); the
  World gate is now `Material::node != u32::MAX → cell mask` (a cel placement's glist draws when
  ITS cel is visible — `View_AddCels` draws the placement's own glist, not the room its centre
  lands in), else the old room path for the dynamics. `scene.rs` stamps `node` = the placement's
  own cell for every main-map creates_cel placement (aligned with `instances()`' resolution over
  `CelGraph::own_cells_main`, by placement ordinal).

## Reproduce-before-fix evidence

`crates/nf-view/tests/fix_portal2_live.rs` (NF_ISO, both green after the fix):

- `the_chain_pass_rescues_the_cels_the_strict_walk_hides` — along the truck→courtyard route the
  game mask rescues material-bearing cells the strict walk hides (the first version of this test
  failed on the pre-fix branch: the rescue set was empty because the port had no chains and the
  cel spheres sat at the raw-bbox origin; the intermediate commits 5cf8fd8/17755e8 show the
  progression). It also pins the structural facts: chain nodes are only 0x8018/0x8019, rooms own
  but never ride chains, and the game mask never drops a strict-visible cell.
- `the_07000007_hidden_room_stays_hidden_and_the_start_view_holds` — the start view submits **180**
  World draws under the game rule (the game measured **177** — fix-perf-level), and every
  strict-hidden room stays game-hidden (rooms are never chain nodes — this is the [M]-grounded
  answer to the driver's window question: the building through the window is hidden in the game
  because the chain pass cannot rescue a room-class cel; only the generic class rides chains).

## Verification status (honest)

- `cargo test --offline --release -j2 -p nf-view -- --test-threads=1`: **green through every test
  binary until the sandbox window cut it** during `levels_sweep_live::deep_sweep_1800_ticks…`
  (the longest walk test; everything before it passed, including all fix_portal_live tests, the
  cel-order, frustum, render and walkreal suites that had run by then). The full 4-crate suite
  did NOT complete in this session's window — please re-run:
  `NF_ISO=$NF_ISO cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  (nf-game/nf-formats/nf-audio are untouched by this diff; only nf-view changed).
- No GPU here — the render itself needs the main agent's check (spots below).

## Shared files touched

None outside the task's named set: `crates/nf-view/src/scene.rs`, `crates/nf-view/src/gpu.rs`,
`crates/nf-view/tests/fix_portal2_live.rs` (new). `lib.rs` untouched (cel_visibility's signature
is unchanged).

## Render spots for the main agent

1. **07000006 `--bond 7.24,42.03,-39.31,10.161,0.428 --frame 120`** (the failing spot): the
   masonry under the left walkway and the lower right castle wall should now draw with the walk
   ON (they already drew with `NF_NOPORTAL=1`). If the hole persists at frame 120 but the
   geometry is right at the spot itself, the residue is the persistent `Camera_CheckLocation`
   state carried along the approach (the headless fresh-state mask at this eye hides nothing —
   the strict walk keeps everything with materials there), which would point at the state
   machine, not the chain pass.
2. **07000006 `--bond 9.87,43.04,9.84,3.177,0`** (was identical on/off): should stay identical.
3. **07000007 `--bond 61.11,44.12,-52.58,-2.541,-0.010`** (the window): the hidden room's
   room-class geometry must stay hidden (the game hides it; the chain pass cannot rescue rooms).
   The generic chunks near it may now draw — the game draws chain-rescued generics too; if the
   "lit building" reappears, compare against a PCSX2 frame at the same spot before judging.
4. **07000007 `--bond 34.25,44.13,-70.71,1.549,-0.070`** (was identical on/off): should stay
   identical.

## Live checks for the main agent

1. `tools/trace/gdbwatch.py` on a courtyard generic runtime cel's **+0x18** (pick any cel from
   the 07000006 pCelList, e.g. via `viewer+4` then `cel+0x14` walk), access type: read at
   `View_AddCels` (0x1e6ba0) — expected: a non-zero chain walk on frames where the courtyard
   walls draw, confirming the chain pass runs in the real game at this spot.
2. Same session: watch **VisCelCount (0x34b3b0)** at the 07000006 spot — expected unchanged from
   fix-portal's measurements (the chain pass does not touch VisCelCount).
3. `tools/trace/fieldprobe.py`: read a generic cel's **+0x80..0x8c** (centre, radius) — expected
   the world-space sphere (the port now derives it placement-transformed); compare against the
   cel's placement translation to confirm the transform.

## What's left

- The exact `+0x18` chain-builder window (the 0x40 cap is [S]); a gdb chain dump on any level
  would pin it — the 05 snapshot's per-cel counts are in this report for comparison.
- The glist+0x44 & 0x10 skip and the list-1/list-2 (±dist², flags 0x1/0x2000) draw-order halves
  of `View_AddCels` are not modeled in the port's sort yet (the port keeps its established
  order); visibility-wise they change nothing.
- Re-run the full 4-crate suite.

## ntfy

(filled after the finish step)
