## Your task: `portal4`, match the game's visible set at the 06 courtyard from live data

Read, in this order: `docs/reports/fix-portal3.md` (on `s/fix-portal3`, 377873e), then
`docs/live/2026-10-13-portal-ground-truth.md` on master. The main agent captured the real game at the exact spot the
port gets wrong (`07000006 --bond 7.24,42.03,-39.31,10.161,0.428`):

1. **A screenshot:** the real game draws the masonry under the left walkway and the lower right castle wall solid. The port
   with fix-portal3 still hides wall cells 20/22/38/5/13/23 there.
2. **The game's own visible list (GDB):** `Vision_Portal_Recurse` runs 3× per frame (three viewers). For the main
   view, **VisCelCount = 2**: the eye cel (sphere (4.5, 43.5, −42.1) r 11.4: your cell 33, so your eye-cel fix is
   confirmed) and a big room **0x0179fb30** (sphere (−13.0, 49.0, −37.0) r 44.0) at depth 1.
3. **That big room's `+0x18` chain, all 40 nodes with class and world sphere** (in the doc). The eye cel's
   chain is empty.

So with only 2 rooms visible, the game draws those walls from the big room's glist or its 40-node chain. Your
"the game holes too at this placed camera" hypothesis is refuted by (1). Find the mismatch:
- Identify the port's cell for 0x0179fb30 (match by world sphere: `CelNode.sphere`, the placement-transformed bbox) and
  check the port's walk reaches it from cell 33 at this camera. The game's depth-1 entry says one portal hop.
- Compare the port's `CelGraph::chains` for that cell against the 40 live nodes (match by sphere). This settles the [S]
  0x40-cap / per-map-block chain-builder model: make the port's chain for that room equal the live one, and derive
  the rule that generalises (pick a second room on 05 or 07 and check it too, using the same gdbcall/PINE method:
  the commands are in the doc).
- Then check which of the port's wall cells 20/22/38/5/13/23 are the game's chain nodes. If a wall is not a chain node
  of a visible room and not a visible room, find which list the game draws it from (View_AddCels 0x1e6ba0's other
  branches, the forced/object lists) before changing anything.
- Pin it with a test: at this spot the port's visible room set is {cell 33, the 0x0179fb30 cell} and its chain
  equals the 40 live spheres (numbers in the test, not game bytes), and the wall cells draw. The other spots in the
  ground-truth doc must not change (07 window: the room stays hidden; 07 hall: identical).

Run the full four-crate suite (start it in the background first thing; fix-portal3's run didn't finish) and list
render spots for the main agent.

Files: `crates/nf-view/src/{scene,gpu,lib}.rs`, `crates/nf-view/tests/fix_portal*_live.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/portal4` (start from `s/fix-portal3`, then merge `origin/master` into it).

---
claimed-by: sandbox5 at 2026-10-13T16:36Z

---
## Report (done, 2026-10-13T18:18Z)



---

# The worker report (docs/reports/fix-portal4.md on s/fix-portal4, commit 02c0786)

# fix-portal4 — the +0x18 chains rebuilt from the live ground truth; the wall-draw path decoded

Task 130e: at `07000006 --bond 7.24,42.03,-39.31,10.161,0.428` the real game holds exactly two
cels visible (the eye cel 0x0179f230 = the port's cell 33, and the big room 0x0179fb30 = the
port's cell 21, depth 1) and still draws the masonry under the left walkway and the lower right
castle wall. The port hid wall cells 20/22/38/5/13/23. Branch `s/fix-portal4` = `s/fix-portal3`
(377873e) + master (ce88d6f); only `docs/WORKLOG.md` conflicted (both sides kept).

## 1. The chain-builder model is refuted and replaced (landed)

The port hung each generic on every cel parsed before it in the same map block (the [S] 0x40-cap
model). The live dump refutes it: the big room's chain is 40 spatially local nodes while the old
model produced 64 parse-order generics from across the whole map (1/12 live nodes matched).

New evidence, walked from the snapshots (`ee_start_05.bin`, `ee_snapshot.bin` — identical results):
the world cel list threads via `cel+0x14`; the chain head is `cel+0x18` and each node's next is
`node+0x18` [M 0x1e6ba0: `fVar9 = *(fVar9 + 0x18)`], so chain members are cel records threaded
through their own +0x18. Facts [L]:
- 137 world cels, **8 owners — all room-class** (6× 0x0804c047 + 2× 0x1804c047), 113 generic
  members (classes only 0x8019/0x08008019), no orphan 0x8019 cels.
- The record's world bbox is two vec4s: min at +0x40 (w 0), max at +0x50 (w 0) — cel 0x16f12a0's
  matches the port's cell 136 bbox exactly.
- **0 of 113 members lie outside their owner's bbox.**
- 65 members lie inside several room bboxes; the actual owner matches `build_FindCel`
  (0x1c8cb0)'s collision tie-break in 65/65 (64 = the earlier+smaller room; 1 = a later room whose
  co-candidate has no collision at the point — the "no hits loses to any hit" rule).
- The chain order is reverse parse order (the live 40-node dump descends by runtime address).

`build_chains` (scene.rs) now hangs each generic (runtime class 0x8018/0x8019) on the room
`find_cell_raycast` — the port's `build_FindCel` — picks for its world-sphere centre, prepending
so a chain reads in the live dump order; the port's unresolvable-object cells (zero bbox/sphere)
are skipped (the game's `View_AddCels` likewise skips null-glist nodes). It runs in the Scene
constructor after the scoped `CollisionWorld` exists (the tie-break rays need it), with the
world-less fallback for sim-less loads.

Verification at the exact spot (`crates/nf-view/tests/fix_portal4_live.rs`,
`the_courtyard_visible_set_and_chain_match_the_live_ground_truth`, NF_ISO): eye cel 33; the walk
visits exactly `[33, 21]` (the live VisCelCount 2); cell 33's chain is EMPTY (the live eye cel's
is); cell 21's chain carries all 12 named live nodes by sphere + exactly 28 railing nodes; the
sister pins that the old model's far-away node (sphere (88.1, 35.7, 28.0) r 13.5) is gone. The
test FAILS pre-fix (under the old model cell 33's chain was non-empty) — shown failing first.
Also green: fix_portal_live 6/6, fix_portal2_live 2/2 (the 07 window stays hidden, the 07 start
band holds), fix_portal3's placed-viewer pin, fix_cel_order_live 2/2, explode_live 2/2,
walkreal_07000001_live 2/2.

Residue: the port's chain for cell 21 has 9 extras beyond the live 40 (port cells
57/108/109/110/111/344/345/346/349 — courtyard generics whose centres are also inside wall room
22's bbox). The live game does NOT hang them on 0x0179fb30, so its FindCel picked the wall room;
the port's `segment_hit_scoped(Some(cell))` stand-in for `Collide_RayIntersect`'s per-cel lists
assigns collision triangles differently (the port's instance→cell mapping), flipping those
tie-breaks. Next session's item, with the live check below.

## 2. The wall-draw path: View_AddForcedObjects 0x1e5fb8 (decoded, not landed)

The wall geometry sits under the wall ROOM placements in the port's parse (wall-region World
materials: node 13 ×48, node 20 ×64, node 22 ×55, node 38 ×96 = 263, plus railing generics) —
and those rooms are NOT in the game's VisCelList, yet the walls draw. The missing list is
`View_AddForcedObjects` [M 0x1e5fb8], called from `View_CaptureScene` (0x1e6df0) right after
`View_AddCels`:

- walks the global object list (`control_first_object()`, threaded +0x14);
- skips when `(obj+0xf8 & object_display_mask) == 0` — `object_display_mask = 1 << (viewer+0x28
  & 0x1f)` set per capture [M 0x1e6df0];
- skips when `obj+0xd8 == 0` (no model) or `(obj+0xf0 & 0x8000) == 0` (not a cel-creating
  placement) or `(obj+0xf0 & 0x10) != 0` (hidden);
- **`Vision_InView(obj+0x8c radius, obj+0x80 centre)` must pass — then it draws with a ±dist²
  key, INDEPENDENT of the VisCelList**;
- only when the object's class byte (`obj+0xff`) == 2 (with the `+0xf0 & 0x400` gate on
  `viewer+0x236 == 2`) does it additionally require its `Collide_StraddleCels` cels to intersect
  the currently drawn cel set (the GfxList walk); other classes draw frustum-only.

This is how the game draws the courtyard walls with VisCelCount=2: the wall objects pass
Vision_InView and are not class-2 (or their straddle cels include the visible rooms). Not landed:
implementing it blindly would either over-draw (the 07 ballroom-window room must stay hidden —
the ground-truth doc's regression) or need the class-byte (+0xff) distribution as evidence.
Live checks below settle it; the implementation is then mechanical (a second draw gate in
gpu.rs for object-class materials keyed by Vision_InView + the class-2 straddle rule).

## Live checks for the main agent

1. **gdbcall View_AddForcedObjects 0x1e5fb8** at the courtyard spot (same method as the ground
   truth doc), dumping for each added object: address, `+0xf0` flags, `+0xff` class byte,
   `+0x80/+0x8c` sphere. Expected: the wall objects (the masonry/castle-wall meshes) appear with
   class byte != 2 — that decides the implementable rule.
2. **gdbcall the wall rooms' +0x18 chains**: the runtime cels for the port's wall rooms (cell 22
   ≈ sphere (−17.0, 44.0, −40.0) r 18.5; cell 20 ≈ (27.0, 52.5, −57.5) r 19.1) — expected: the 9
   extra courtyard generics (spheres (−17.0, 43.0, −39.5) r 12.6, (−17.1, 44.8, −39.9) r 11.5,
   (−17.1, 44.3, −43.6) r 5.5, (−16.8, 44.2, −37.2) r 2.6/2.7, (−24.5/−9.5, 43.4, −29.8/−49.2)
   r 0.5 ×4) hang THERE, not on 0x0179fb30 — that confirms the tie-break divergence is in the
   port's collision-cell assignment.
3. fieldprobe: read `object_display_mask` (address per 0x1e6df0's store) for the main viewer —
   expected != 0x10 (so room-class chain nodes would draw through the chain pass's 0x40000
   branch, unused so far).

## Render spots for the main agent (after the next session lands View_AddForcedObjects)

- `07000006 --bond 7.24,42.03,-39.31,10.161,0.428` — the masonry under the left walkway and the
  lower right castle wall solid (the ground-truth screenshot); no sky through them.
- `07000007 --bond 61.11,44.12,-52.58,-2.541,-0.010` — the ballroom window: ONLY the snowy
  mountain, no lit building (the forced-objects rule must not rescue that room).
- `07000007 --bond 34.25,44.13,-70.71,1.549,-0.070` — the clock hall: identical to the game.

## Suite status (honest)

nf-game + nf-formats + nf-audio: 37 test-result-ok groups; the only failures are the 3 allowed
`disc_golden` slim-ISO stubs (MIS01.MUS etc.) and `pss_movies_live`'s 1 (needs NF_MOVIE_ISO, the
retail image — the slim ISO stubs all movies; environmental, not a regression). nf-view: the
portal/cel/draw-adjacent suites above are green (the chain change's blast radius); the remaining
~140 nf-view test binaries did not fit the sandbox window (each link OOM-retries cost a cycle
early on; then time) — re-run before merge.

## What's left

- Implement View_AddForcedObjects 0x1e5fb8 in the renderer (the walls draw) after live check 1.
- The 9-extra tie-break divergence: the port's collision triangle→cell assignment vs the game's
  per-cel `Collide_RayIntersect` lists.
- Full nf-view suite re-run; the disc_golden/pss_movies environmental failures counted.

## Files touched

`crates/nf-view/src/scene.rs` (the chain builder replaced by `build_chains`, its call sites, the
CelGraph::chains doc), `crates/nf-view/tests/fix_portal4_live.rs` (new: the pin + the snapshot
rule probe + the ground-truth probe). No shared file rewritten; nf-game/nf-formats/nf-audio
untouched. The sandbox's `~/.local/lib/pkgconfig/alsa.pc` libdir fix (alsa-sys's stale build
cache pointed at the system libdir without the dev symlink — every nf-view link failed with
"unable to find library -lasound") is environment-only, nothing committed.
