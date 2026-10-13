# fix-swim-vent5 — the third-icon scope (E at the 05 gate door creeps instead of opening it)

Branch `s/fix-swim-vent5` (from `s/fix-swim-vent4` c463614 + master 3d33a27 merged; WORKLOG
keep-both). Task `140e-fix-swim-vent5`.

## What was wrong

The four merged-suite reds (`level_end_live::route_b_on_foot_gate_door_loads_level_on_e`,
`level_carry_live::{castle_door_route_spawns_at_the_matched_entry,
use_key_still_activates_gated_triggers_and_load_level}`,
`walk_07000005_live::cold_start_reaches_the_castle_door_and_exits_to_the_castle`) all share one
symptom: E at 07000005's castle-gate door (the 0x1D LOAD_LEVEL trigger at (30.0, 38.84, 38.2))
entered the crawl instead of opening the door.

Reproduced headless before any change (`crates/nf-view/tests/fix_swim_vent5_live.rs`, run against
the pre-fix tree):

- `the gate door must read no icon [L PCSX2 2026-10-13] (got 6)`
- `E at the gate door must not crawl (substate 7)` — substate 7 is the crawl, the user's exact
  playtest symptom.

## The fixes (crates/nf-view/src/scene.rs, two areas)

1. **The creep arm's gate is the icon context ALONE.** Both `Player_Activate` sites in the Sim
   (the use-key block and the second activation site) dropped the proximity fallback
   (`|| best.is_some()` / `|| cbest.is_some()`). The game's arm is
   `if (BL+0x95f == 6)` [M `Player_Activate` 0x1a5170:66] — the squared-1000 scan over class-0x2B
   objects [M 0x1a5170:66-97] runs only inside that gate. At the gate door the second vent's
   creepwall IS inside the scan (≈24.1 units, squared ≈579 < 1000), which is why the fallback
   crawled; the game reads +0x95F = 0xFF there, so it never does [L].

2. **The third-icon scope is the icon's own straddle volume, not the shared cel.** The old loop
   wrote an enabled icon's type when the icon's cel equalled Bond's (`same_cel`) or within 4
   units. 07000005's courtyard defeats that: the port's 19 room-cel bboxes put the gate door AND
   the second vent's ctx-6 icons in the same cel 1, so the door read 6 permanently. The game's
   scope, from the decompile: an object whose collision volume straddles a cel leaves a hitdata
   node on every occupant of that cel with node+0x58 = the straddler [M `Collide_Update`
   0x1ea398:160-233; nodes whose query distance is 1e8 (a miss) are freed, 0x1ea398:123-139], and
   the icon writes its type when a node's +0x58 is the class-3 player, gated by its enable
   channel (obj+0xf4, set at create when param 1 == 0) and its 4-tick refresh counter
   [M `ThirdIcon_Update` 0x196e80:15-27, 36-53; `ThirdIcon_Create` 0x197070:21-32]; the tick
   starts at 0xFF [M `Player_CollisionHandler` 0x1949a8:22]. The straddle volume is the object's
   world sphere [M `Controls_StraddleTest` 0x134778:30-38 — the HITTEST from position + world
   sphere; `Control_BuildWorldSph` 0x133070].

   **The port's stand-in and its limit, stated plainly:** the port cannot resolve that chain —
   its cel set has no per-cel geometry for these courtyard floors (probe: the scoped buckets of
   cels 0/1 return no hit under the gate door, the second vent, or their icons, while the
   unscoped world does), and `build_FindCel`'s ray tie-break [M 0x1c8cb0:163-232] cannot be
   ported onto data the port does not have. So the scope ports as the icon's placement sphere
   (the "its own radius or volume" half of the game's gate), with the radius taken from the live
   measurements rather than guessed: [L PCSX2 2026-10-13] +0x95F reads 0xFF at the gate door
   (30.0, 38.84, 37.70 — 24.1 from the nearest ctx-6 icon) and the level start (104.95, 25.12,
   108.83 — ~94), while the vent crawl spot (3.9 from ctx-6 icon[0]) reads 6 and the south-ledge
   wire spot (1.8 from ctx-3 icon[5]) reads 3. The window is (crawl must apply, door must not) =
   roughly [3.9, 24.1); the port uses 12.0 (squared 144), the middle. The pinned test holds the
   measured values; a later celglist-sphere parse can replace the constant.

## Verification

- `fix_swim_vent5_live` (new, NF_ISO): the two game-pinned spots read 0xFF, the crawl spot reads
  6, and E at the door stays SS_WALK with a sister asserting the scan range is genuinely met
  (nearest wall squared 579 < 1000 — only the context keeps E off it). Both tests shown failing
  first (see above), green after.
- The four reds, re-run on the fixed tree: `level_end_live` 5/5, `level_carry_live` 8/8,
  `walk_07000005_live` 2/2 (incl. `cold_start_reaches_the_castle_door_and_exits_to_the_castle`).
- Sisters kept green: `fix_swim_vent_live` 2/2 (the vent crawl still enters substate 7, carries
  the wall and exits — the crawl spot is inside the ported scope), `fix_wire2_live` 3/3 (the
  ctx-3 jump-grab path intact).
- Full four-crate suite: see the count below (run at ~20:17 UTC; on the slim ISO only the 3
  `disc_golden` tests and `pss_movies_live` may fail).

## What's left / live checks for the main agent

- The 12.0 radius is a measured stand-in. **Live check 1:** with `gdbwatch.py` on an icon
  object's +0x8c (world-sphere radius, `Control_BuildWorldSph` 0x133070 writes `glist+0x1c ×
  |obj+0xe8|`) at 07000005's first vent, report the value — one reading replaces the constant.
- **Live check 2:** confirm the icon's obj+0xd0 chain at the gate door is empty (no hitdata
  nodes) while Bond stands there 40 frames, and non-empty at the vent entry — that pins the
  cel-chain direction (player→icon hitdata) vs any icon-side query.
- The two degenerate ctx-2 icons at the origin (icon[2]/icon[6], placement ~(0, 0.06, 0)) are
  inert under the sphere scope (no reachable spot within 12 on the tested routes); the game may
  link them to no cel at all (the port's `find_cel` returns None for them) — harmless either way.
- No RNG draws added; no wall-clock in Sim; shared files untouched beyond scene.rs.

## Suite count

**781 passed, 3 failed** — the only failures are exactly the 3 sanctioned `disc_golden` tests on
the slim ISO (`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_manifest`,
`directory_tree_matches_the_golden_manifest`); no other failure anywhere.

The run could not go through one `cargo test` invocation (two detached runs were killed between
tool calls), so it was executed as foreground chunks that share the same command line and
`--test-threads=1`, covering every target: nf-game + nf-formats + nf-audio 325/3 (chunk A), nf-view's
83 non-walk binaries 328/0 (B1), the 19 early walks + level/levels_sweep binaries 66/0 (B2a), the 45
later walks + walkreal binaries 62/0 (B2b) — 147 of 147 nf-view test binaries ran. The four
previously-failing families all green inside these chunks: `level_end_live` 5/5, `level_carry_live`
8/8, `walk_07000005_live` 2/2 (incl. `cold_start_reaches_the_castle_door_and_exits_to_the_castle`),
plus the sisters `fix_swim_vent_live` 2/2, `fix_wire2_live` 3/3 and the new `fix_swim_vent5_live`
2/2. (Baseline before this branch: 764 passed, 4 failed — the four "E at the gate door" reds.)
