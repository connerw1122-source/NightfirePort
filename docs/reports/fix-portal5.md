# fix-portal5 — the courtyard walls belong to the visible rooms: the draw mechanism decoded, the own-cell attribution fixed

Task 130e. At `07000006 --bond 7.24,42.03,-39.31,10.161,0.428` the real game holds **VisCelCount = 2**
(the eye cel 0x0179f230 = the port's cell 33 at depth 0, and 0x0179fb30 = the port's cell 21 at depth 1)
and still draws the courtyard walls. fix-portal4's forced-objects hypothesis was refuted live [L ground
truth §3], and its census ("World materials node 13 ×48, 20 ×64, 22 ×55, 38 ×96") attributed the
wall-region materials to hidden ROOM cells. Branch `s/fix-portal5` = origin/s/fix-portal4 (02c0786) +
origin/master (aba19b0); only `docs/WORKLOG.md` conflicted (both sides kept append-only). Branch-name
note: the task header/rules say `s/fix-portal5` (twice), the tail said `s/portal5`; neither existed on
origin — `s/fix-portal5` created and used throughout.

## 1. The game's complete draw mechanism (decoded this session)

- **`vision_generate_display_list` 0x1e8848** [M]: after `Vision_Portal_Recurse`, the VisCelList
  (0x34b3b0, 8-byte {cel, u16 depth} entries, dedup + insert by `Vision_AddCelToDraw` 0x1e8908, cap 0x40)
  is `QuickSort`ed by depth and **threaded through `cel+0x20`** into the per-frame list whose head is the
  global `GfxList._4_4_` (0x2d6274, cleared by `View_CaptureScene` 0x1e6df0 each frame).
- **`View_AddCels` 0x1e6ba0** [M, disassembled] walks exactly that +0x20 list (the visible cels,
  depth-sorted) and per visible cel: pushes the cel's own glist with a **list-0 key (−90000 + 10·k)**
  gated only by glist draw-flag 0x10, then walks the cel's `+0x18` chain pushing each node's glist with
  **±dist² keys** (list 1/2 by glist flag bit 0x1/0x2000) gated by `Vision_InView` (0x1e89e0) and the
  0x40000/display-mask check, skipping 0x8018 nodes while the viewer's cel is 0xc023-class.
- **`View_AddObjects` 0x1e62d8(cel, viewer)** [M, disassembled — the path fix-portal4's report didn't
  reach]: called for EVERY visible cel AND every visible chain node; walks **the cel's own object list**
  (head `cel+0x8`, threaded `obj+0x8`; `obj+0x20` = the owner cel, filled by
  `control_link_object_to_cel` 0x1346b8) and adds every object passing {not ((obj+0xfa & 0x110)==0 &&
  obj+0xd8==0), `obj+0xf8 & object_display_mask`, **glist flags not 0x8010**, `Vision_InView`} with ±dist²
  keys — cel-scoped. `View_AddForcedObjects` 0x1e5fb8 is its complement: it draws only glist-flag-**0x8000**
  objects from the global list; 0x1e62d8 skips them. So a model either draws through its cel
  (0x8000) or through the owner cel's object list (not 0x8000), plus the forced sweep.
- **The world list threads through `cel+0x14`** (head = `*(glb_world)` — `glb_world` 0x30d0f8 points at a
  4-byte world record, `build_alloc_world` 0x1c93c8 allocates just that; `build_alloc_cel` 0x1c93f8
  prepends). The +0x14 order is ASCENDING allocation/parse order [L ee_start_05.bin: 137 cels ascending
  by address]; `build_link_objects_to_rooms` (below) walks it in that order and prepends to the chain,
  which reproduces the live chain's descending-address order [L ground truth].

## 2. The cel → glist assignment is explicit and the port's matches the game exactly [L]

`parsemap_block_map_data_static` 0x1d1998: each record {u16 model ref, s32 hash_or_−1, u32 flags
(0x8000 = creates cel, &0xffff = class), 6 floats, prop list} resolves its model through the
`hashtable_getitem` / `pCelList + ref*0x50` (the per-model 0x50-byte glist records; the snapshot shows
static cels SHARING glist records per model) and copies **the glist's local sphere (glist+0x10..0x1c),
local bbox (+0x20..0x40) and draw flags (+0x44)** into the cel (+0x80..0x8c, +0x40..0x60, +0x94);
`parseentity_fixup_entity` 0x1d00c0 normalizes the class word (0xc047 → 0x804c047 etc.). There is NO
spatial attribution at load — the placement names the model.

Verified on the 05 start snapshot (`ee_start_05.bin`): **all 137 runtime cels' glists match the port's
parsed objects by model header (local sphere + radius + bbox + flags), 137/137 with 0 unmatched, and
every runtime ROOM cel's game model equals the port's room placement's resolved object** (Room_Sky,
Room_Wall_G10, Room_Courtyard_Top_I, … each side identical). The port's placement→model attribution is
NOT the divergence (`probe_portal5_glist_model_match` in fix_portal5_live.rs).

## 3. The real divergence #1: the own-cell key misalignment (fixed)

`Scene::build` fed its positional `own_i` (advanced only for non-room instances) from an `own_cells`
vec built over EVERY resolved placement — rooms and the retained-out classes (0x2a/0x22/0xd4)
included — so after the first room every non-room placement read an **earlier placement's cell**.
Measured on 07000006: **379 of 765 non-room instances misaligned** (placement 45 keyed under cell 5, 44
under u32::MAX, …). The wall-region generics keyed under hidden ROOM cells 13/20/22/38 (the census the
task quoted) while their true cells hang on room 21's chain. Fix: `own_cells` is now built with exactly
the walk's filters (unresolved, retained-out, room-class dropped; plus `cels_only`'s trailing 0x20/0xe2
extension appended). Post-fix audit: **0 of 765 misaligned**.

## 4. The real divergence #2: the chain-owner rule (fixed)

The game's load-time linker is **`build_link_objects_to_rooms` 0x1c9150** [M], not the runtime
`build_FindCel` 0x1c8cb0 fix-portal4 borrowed: for each generic (short class 0x8017/0x8018/0x8019) it
walks the +0x14 world list, considers every **0xC047/0xC023 room** with `generic.radius <= room.radius`
whose bbox contains the generic's sphere centre, and picks the room whose **`Collide_SphereIntersect`
(0x1ec940) hit is closest** — a containing sphere (radius 4001) scoped to the candidate cel (hits whose
+0x54 cel is the candidate, or 0/unmapped, intersect; everything else reads 1e8); strict `<` keeps the
first candidate in parse order. It writes `generic+0x1c` = owner and prepends `generic` to
`owner+0x18`. The port now implements exactly this (`build_chains` → `link_owner_room`, with
`CollisionWorld::closest_point_scoped` — new public method in nf-game/src/world.rs, minimal shared-file
touch — computing the closest point over the cell's own scoped runs + the unmapped `always` runs, the
same cel-0 semantics 0x1ec940's walk has). `find_cell_raycast` (the runtime FindCel rule) stays for the
eye-cel picker and as the world-less fallback.

## 5. The courtyard pin (RED first, then GREEN)

`crates/nf-view/tests/fix_portal5_live.rs` `the_courtyard_walls_draw_with_the_walk_on` (NF_ISO):
- the walk visits exactly `[33, 21]` (VisCelCount = 2); cell 33's chain empty;
- cell 21's chain carries the live 40 (the 12 named nodes by sphere + exactly 28 railing nodes);
- **cells 33 and 21 draw with ALL their World materials** — the wall masonry and castle wall are
  Room_Courtyards_G's own 171-batch model (cell 21, bbox (−40,42,−71)..(14,56,−3)) and the eye room's
  45 — that is the game's wall draw at VisCelCount = 2;
- no wall ROOM cell (5/13/19/20/22/23/38) leaks into the visible set;
- **attribution pin**: the courtyard generic cells (39/40/41/42/43, the heli parts 57/344/345/346) own
  node-keyed World material counts equal to their placement objects' batch counts.
RED before the fix (pre-fix tree, both fixes stashed): `cell 40's placement object must own its 1
batches as node-keyed materials (got 4)` — FAILED. GREEN after.

Probes in the same file (log-only): `probe_portal5_glist_model_match` (§2), `probe_portal5_wall_attribution`
(the wall-region census by node, the wall cells' identities — Room_OuterWall_B5/Room_Tunnel_DA/
Room_Tunnel_G9/Room_Courtyard_Top_I/Room_Courtyards_G4/Room_Courtyard_Lower_G8/Room_Wall_G10 — the
chain(21) dump with LIVE marks, the own-cell alignment audit), `probe_portal5_big_room_model` (cell 21 =
"Room_Courtyards_G" 171 batches; cell 33 = 45).

## 6. The 9 extra chain nodes on cell 21 — identified, owner still divergent

The port's chain(21) = the live 40 + **9 extras, now identified by object**: the parked helicopter's
statics — 57 "OBJECT_HeliPadLight_1" r 12.6 (48 mats), 345 "Object_Helicopter" r 11.5 (118),
344 "OBJECT_HelicopterCabin" r 2.7 (38), 346 "OBJECT_HelicopterWindows" r 2.6 (3), 349
"BLOCKINGPOLY_HelicopterRear" r 5.5 (0x10-flagged blocker, 0 mats) — plus four r 0.5 railing dots
(108-111). Their spheres (−17.0, 43.0..44.8, −37.2..−43.6) are NOT among the live 40, so the game's
linker did NOT hang them on 0x0179fb30; both candidate rooms 21 and 22 contain the centres, so the
tie-break decides — and the tie-break reads the **per-cel collision assignment**, which the port still
models differently (see Left). Note the game must still render the helicopter somehow at this spot
(it sits in the frustum) — most plausibly through room 22's chain or the cel+0x8 object lists; the
live check below settles it, and the port's current over-draw (the heli via chain(21)) may well match
the picture even if the owner cel differs.

## Live checks for the main agent

1. **gdbcall `build_link_objects_to_rooms` 0x1c9150** (or the +0x1c owner word of each heli cel) at
   the courtyard spot, for the heli statics' runtime cels: expected `+0x1c` = 0x0179fb30's neighbour
   room (the wall room 0x0179f??0 the port's cell 22 corresponds to) or 0 — that decides whether the
   port's chain(21) extras are an over-draw or correct-by-another-path.
2. **gdbcall View_AddObjects 0x1e62d8** at the same spot for cel 0x0179fb30's +0x8 object list:
   the list's members (addresses + obj+0xd8 glist) — expected: the heli parts as linked objects
   (they would then draw through 21 legitimately and the port should model the link, not the chain).
3. fieldprobe the heli: is the helicopter VISIBLE in the PCSX2 courtyard screenshot at
   `7.24,42.03,-39.31,10.161,0.428`? (The port now draws it via chain(21).)

## Render spots for the main agent

- `07000006 --bond 7.24,42.03,-39.31,10.161,0.428` — the masonry under the left walkway and the lower
  right castle wall solid (the ground-truth screenshot); no sky through them; no wall-room geometry
  missing that the game shows.
- `07000006 --bond 34.25,44.13,-70.71,1.549,-0.070`-equivalent courtyard sweep (the alignment fix
  touched every level's node keys — any spot of 06 confirms generics still draw).
- `07000007 --bond 61.11,44.12,-52.58,-2.541,-0.010` — the ballroom window: ONLY the snowy mountain
  (no lit building; the fix must not rescue that room).
- `07000007 --bond 34.25,44.13,-70.71,1.549,-0.070` — the clock hall: identical to the game.
- `07000005` start view — the alignment fix's broadest blast radius (every non-room placement's node
  key changed): the courtyard/estate should render as before.

## What's left

- The per-cel **collision assignment** (the residue fix-portal4 named): the game links each collision
  object to the cel `build_LinkToRoom` 0x1c9680/`build_FindCel` picks (`obj+0x20` owner; statics'
  collision rides their own cel's glist), while the port's `CollisionWorld::build_scoped` maps
  instances by smallest-containing-bbox and excludes the linked dynamics (doors etc.) from the scoped
  world. That difference, not the linker rule, is what still flips the 9 heli tie-breaks.
- The suite run (below) and the render verification (no GPU here).

## Suite status (honest)

See the final numbers appended below before merge; slim-ISO allowances: the 3 `disc_golden` stubs and
`pss_movies_live` without NF_MOVIE_ISO.

## Files touched

`crates/nf-view/src/scene.rs` (own_cells alignment + build_chains/link_owner_room + docs),
`crates/nf-game/src/world.rs` (`closest_point_scoped`, minimal), `crates/nf-view/tests/fix_portal5_live.rs`
(new: the pin + 3 probes). Shared-file notes: world.rs addition listed here per the rules; WORKLOG.md
append-only entry.
