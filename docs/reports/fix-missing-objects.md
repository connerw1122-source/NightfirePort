# fix-missing-objects — report

Branch `s/fix-missing-objects` (from origin/master c9e1582). Time box: this session only (sandbox
until ~10:05 UTC; started 08:06). Everything below cites the decompile/docs; the port's behaviour is
pinned by NF_ISO tests through the real build path.

## What was wrong (two real bugs, one root cause chain)

The fountain, the guard-room furniture (table/bed/chair/wall map/clock), the lockers and — disc-wide
— every 0700000a office prop, 07's books, 09's lights and more were **orphan generics**: their cel
nodes (placement flag words 0x8019, `cel_flags_word`) never got a chain owner at load, and
`View_AddCels` 0x1e6ba0 draws a generic ONLY from its owner room's `+0x18` chain under
`Vision_InView` (fix-portal5 §1). No owner → the node never enters the game-mask → gpu.rs's World
cull drops every material at that node. The water patch drew because `Object_Pondwater` rides the
water path, not the static glist.

1. **`CollisionWorld::build_scoped` attributed each instance's collision with a
   smallest-volume-containing-bbox rule** (nf-game/src/world.rs). The guard-room floor landed in the
   table's tiny bbox (its centre sat inside it), the courtyard floor in the fountain's bbox — so at
   `build_chains` time every candidate room's scoped bucket read empty and the owner pick found no
   hits. The game's per-hit cel word (`HITDATA+0x54`, the input of `Collide_SphereIntersect`
   0x1ec940's candidate-scoped query) carries the object's **own placement cel**:
   `parsemap_block_map_data_static` 0x1d1998 creates one runtime cel per placement whose glist IS
   the placement's object. The instances are the cel placements in parse order and the scope graph's
   cells are `parse_cel_graph`'s per-placement cells in the same order, so the k-th pushed instance
   IS the k-th cell. **Fix**: `build_scoped` keys the k-th pushed run to cell k. `attribute_scope`'s
   post-chain re-key (the game's final attribution) is unchanged.
2. **`link_owner_room` dropped a candidate whose scoped bucket was empty** (`continue` on `None`),
   but the game's read is "hits whose +0x54 cel is the candidate, or 0/unmapped, intersect;
   **everything else reads 1e8**; strict `<` keeps the first candidate in parse order"
   (fix-portal5 §4). A candidate with no geometry still participates at distance 1e8, so with no
   candidate hitting, the FIRST containing room wins — the game leaves a generic unchained only
   when no room passes the radius+bbox gates. **Fix**: `unwrap_or(1.0e8)` instead of `continue`
   (crates/nf-view/src/scene.rs).

After both fixes the fountain, the guard-room furniture and the lockers chain to their rooms and
draw (fountain: owner room 27, all 96 batches kept at the user's spot; table/bed/chair/map/clock:
owner room 7; locker: 15/15 materials at the lockers spot). The disc-wide census (below) is clean.

## The 0c pylon — not reproducible on this tree

At the exact user spot (`--bond 14.22,16.28,-139.54,0.986`), driving the REAL camera path
(`Bond::stand_at` → tick → `head()` = [14.22, 16.965, -139.54]), the port resolves eye cell 91; all
four pylon parts (`pylon`/`pylon_back`/`pylon_left`/`pylon_right`, placements 160–163, world-baked
at x 44–60, z −115..−106, translation (0,0,0)) and `pylonshadow` ride owner room 77's chain, pass
`Vision_InView`, and every one of their 14 World materials survives the mask and the frustum (probe
6, `probe_missing_pylon.rs`, kept). The same held BEFORE this session's fixes (owner Some(77)
pre-fix), so the 2026-10-13 render either predates the portal6-era merges or hit a state this
sandbox cannot reproduce. What I tried: the hand-built frustum at the literal spot, the real
stand_at+head camera, both with and without the fixes; all four probes show the pylon drawn. The
report does not guess a fix; `the_0c_pylon_is_in_the_drawn_set_at_the_user_spot` pins the current
good state as a regression guard. The user's "wires" have no undrawn placements on 0c (census:
0 no-draw-path) — the pylon's cables live inside the pylon mesh.

## The census (every on-foot archive, the task's counts)

`fix_missing_objects_live.rs::census_no_unchained_generics_and_no_unexplained_skips_on_any_level`
runs `Scene::build`'s exact sequence (parse → `build_scoped` → `build_chains` → `attribute_scope`)
per archive and asserts: **0 unchained meshed generics** and **0 no-draw-path mesh placements** on
all 21 on-foot archives (07000001–0700001b + 07000041/43/46/48/49/4b; the driving missions
07000023-29 and the shared packs are out of scope per docs/systems/levels.md §7). Generics per
level: 01:103, 02:200, 03:239, 04:40, 05:113, 06:322, 07:287, 08:92, 09:160, 0a:474, 0b:169,
0c:273, 0d:260, 11:137, 12:438, 13:48, 14:242, 15:324, 16:106, 1b:22 — 0 unchained everywhere.
Explained skips per the game's own rules: the glist not-drawn bit 0x10 (triggers/AI markers) and
the collision-only bit 0x1000 (BlockingPoly/PlungeTrigger), plus the documented no-draw classes
(0x29 switch, 0x31 fusebox — obj+0xD8 = 0 [M 001adcc0/001b0448]; 0x27 GT; 0x32 rainbox; 0x38/0x39
+ 0xe4 missile visuals; 0x3f creep walls; 0xf2 emitter; 0x21 ripples; 0x4e lights; 0xdf snow;
0x24/0x2d the player's own object; 0xd9/0xe8/0x26/0x28 controllers; 0x17/0x23/0xff map-block data).

**Found gap (reported, not fixed here)**: kind 0xea/0xeb trigger objects whose glist lacks the
not-drawn bits — `Trigger_Create` 0x1b1720:95 sets obj+0xF0 = glist+0x44 (docs/fields/glist.md), so
the game DRAWS them: 0700000a's `laser`/`laserbeam` tripwires (flags 0x0/0x645) and 07000012's
`lft_liftswitch`/`sh_shutter_switch` (flags 0x0). Also still un-ported: the 0x2f helicopter
(Copter_Create 0x17d340, docs/systems/world-objects.md "Missing"). Both are follow-up tasks.

## How it was verified

- **Fail-first**: `the_named_objects_are_in_the_drawn_set_at_the_user_spots` failed on the pre-fix
  tree (probe 4, 08:3x UTC: fountain/pondwater/furniture/locker nodes `owner None`, `node_vis
  false`, 0/N materials visible) and passes now; `the_0c_pylon_is_in_the_drawn_set_at_the_user_spot`
  pins the pylon state; the census failed on 06 (orphan generics) and on the 1e8-rule gap (0a
  office furniture) before its fixes. Sister assertions: the lamp post keeps owner room 34 (the fix
  must not re-home others), the pondwater stays chained, every named node's materials survive both
  the mask and `Vision_InView`, the pylon's 12 draws all kept.
- **Neighbour re-runs green on the fixed tree**: the whole portal chain (portal_live 4,
  portal_eyecel_live 1, fix_portal2..6_live, portal_sweep_live 3, walkreal_celprobe_live 2),
  nf-game's unit tests (the world.rs scope tests included), fix_missing_objects_live 3.
- Full suite: `tools/test/suite.py` could not survive this sandbox in the background (nohup dies
  between tool calls — same as the previous two sessions); foreground chunks were still running at
  the deadline.nf-game + nf-formats + the nf-view portal/object/census chunks above are green;
  the levels_sweep is the one file the next session should re-run in full.

## Shared-file deltas (small, additive)

- `crates/nf-view/src/scene.rs`: `scope_graph` fn `fn` → `pub fn` (the census drives the exact
  build path); `link_owner_room`'s `continue`-on-None → the game's 1e8 miss distance + doc comment.
  No other lines touched.
- `crates/nf-game/src/world.rs`: `build_scoped`'s obj_cell rule (own-cel ordinal) + the doc
  comment. The unit test at world.rs:1014 hand-builds its buckets and needed no change.

## What's left

- The 0xea/0xeb trigger-mesh draws (the 0a lasers, the 12 lift switches) — the game draws them;
  needs a sweep like the ladder/grapple ones plus the trigger lane's say-so.
- The 0x2f helicopter (Copter_Create) — pre-existing gap.
- Re-run the full suite (the levels_sweep especially) on the merged tree.
- Live checks for the main agent (nf-view): `--bond 21.12,39.04,-0.48,8.683` (06) — the fountain
  basin/tiers/fish statues now draw; `--bond 33.50,41.13,-17.90,9.158` and
  `--bond 31.94,41.13,-17.36,10.756` — bed/desk/chair/wall map/clock and the lockers; PCSX2 ground
  truth for both. `--bond 14.22,16.28,-139.54,0.986` (0c) — please re-render: this tree draws the
  pylon in every headless probe; if the live render still hides it, the divergence is in the GPU
  frame path, not the mask (list what you see — the probes say node-vis true for 116–119/74).
  Visual work needs your render before merge (lesson 6).

## ntfy

Sent after this report (see the WORKLOG tail for the result line if it failed).
