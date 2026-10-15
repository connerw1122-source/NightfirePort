# fix-bullet-child-list — the searchlight bullet box's exact path (U-041)

Branch `s/fix-bullet-child-list` (cut from master 404d7dc1, merged with origin/master before the
final push). Session 13:36–15:15 UTC 2026-10-15, sandbox window honoured. Files:
`crates/nf-game/src/searchlight.rs`, `crates/nf-game/src/drone.rs` (shared hooks, below),
`docs/systems/searchlight.md` (new), `docs/UNKNOWNS.md` (U-041), `docs/GAPS.md` (G-033 row),
`docs/WORKLOG.md`, `crates/nf-view/tests/fix_bullet_child_list_live.rs`.

## What's done

1. **U-041 answered, with addresses** (docs/systems/searchlight.md, the new systems page):
   the head's `obj+0xD0` entries' float at `+0x08` is **the bullet's damage** — the collider's
   `HITTEST+0x90` = `weapon_definition+0xC` [M `Bullet_update` 0x12e7b0:391] — and **the writer is
   `Collide_Update` 0x1ea398:160-234's mirror phase**: per frame every object's list is cleared
   [:25-53], the collider picks candidates (`Collide_PickObj` 0x1eae80, gated on
   `obj+0xD8 != 0 && *(obj+0xD8)+8 != 0` — parsed collision data — with the `obj+0x80/0x8c` sphere
   broadphase), `Collide_Intersect` 0x1ebaa8 copies the object's `obj+0x90` matrix and
   `Intersect_RayGeom` 0x10a8c8 ray-tests the COLLDATA box tree down to 0x40-byte triangles, the
   survivors are sorted by `+0x0C` and each is mirrored into the HIT object's list (`+0x08` =
   damage, `+0x58` = the bullet, `+0x51 |= 1`), stopping after the first non-pass-through contact
   [:226-229]. The reader census: `Searchlight_Update` 0x1cb900's **kind-1 gate** (:88-90) is the
   ONLY reader — class 0x22's collision handler is 0 [L `control_funcs` 0x26f890 in
   ee_snapshot.bin] — so a mount-only hit never breaks the light; the walk (LAB_001cc1d4) breaks
   on a class-5 entry with damage > 0.0 (state 2, psiLights deleted, `Control_NextLOD`, sound
   0xE6). The searchlight's collision volume is the head object's own collision cels
   (`OBJECT_SearchLight1`, 130 triangles, map block 0x2E — the game's `obj+0xD8` celglist), not
   the gfx box.
2. **The stand-in replaced** (`searchlight.rs::Searchlights::bullet_check`): the head part's
   parsed collision triangles (`Part::coll_tris`, fed from `MapObject::collision` at create) in
   the part's current matrix frame, with the game's two gates — the segment truncated at the
   tick's stop (`drone.rs`'s new `bullet_path_stops`, pushed per bullet in
   `update_player_bullets` = the port's form of the mirror walk's stop) and the damage > 0 check
   read off the weapon record via the existing `bullet_path_weapons` (`weapon_definition+0xC`).
   The break block (state 2, parts hidden, spot light deleted, 0xE6) is the game's, unchanged.
   The old box walk (head AND mount placement gfx boxes) is gone — it was also frame-mismatched:
   it tested a current-frame segment against placement-frame box coords, so on 07000006's
   sweeping heads it missed even square-on housing shots.
3. **Tests** (`crates/nf-view/tests/fix_bullet_child_list_live.rs`, NF_ISO, 07000006, through
   `nf_view::load` + `step_bond(.., trigger = true, ..)`):
   - `a_mount_only_shot_never_breaks_the_light` — **PASSES post-fix**: a burst through the mount
     pole below the head's collision bbox (no head triangle can be touched — the disc's own
     bbox) leaves the head in state 0; every flight ray is verified against the parsed cels.
   - `a_head_box_corner_shot_that_misses_the_cels_does_not_break` — **PASSES post-fix**: a ray
     through the old stand-in's box that misses every head cel does not break the light (the old
     predicate is reproduced in the test and shown to catch the ray class).
   - `a_lamp_housing_shot_breaks_the_light` — **UNRESOLDED premise** (see What's left).
   - **Fail-first shown**: with the old code restored (stash) all three failed — the housing shot
     did NOT break (the stand-in's frame mismatch), and the sisters' old-code behavior was the
     box walk documented above.

## How it was verified

- The two passing live tests above (through the real input path; the flight rays are read off
  `bullet_paths` and checked against the disc-parsed collision triangles — the game's cels).
- The pre-fix full suite (started in the first minutes, on the pre-fix tree): **1316 passed /
  6 failed / 66 ignored (54 min)** — the 3 `disc_golden` tests + the no-GPU pylon set, the
  accepted slim-ISO state.
- Post-fix: `fix_bullet_child_list_live` 2 passed / 1 ignored (the reason in the attribute) on
  the final merged tree. The full four-crate rerun does not fit the 2-hour window after the
  mid-session merge (see What's left).

## What's left

- **The registered-housing-shot live test's aim premise** (`#[ignore]`d). The burst aims through
  the real fire path, but the muzzle rides the gun's anim pose: measured spawn offsets from
  `bond.pos` across runs: (+0.108, +1.032), (+0.675, +0.874), (+0.675, +0.935) — stable WITHIN a
  run (dbg3: five consecutive ticks identical), different ACROSS runs/poses, so a per-burst
  compensation still leaves ±0.06 residuals and the burst's rays crossed no cel even aimed at the
  cels' own tri centroids (the housing shell may be open at the centroid too). Within the window
  the registered-shot demonstration did not materialize; the fix's hit path is the same
  `seg_tri`-on-`coll_tris` code the sisters exercise. Next session: settle Bond on the ground
  first (a stable gun pose), or drive the registered shot from a fire script; then un-ignore.
- The background `nf-game`-only run on the final tree was starved by the interactive test runs'
  cargo lock and produced no totals before the window closed; the pre-fix full-suite baseline
  above is the recorded state. The final tree's deltas are `drone.rs`/`searchlight.rs` + the new
  test only.
- The full four-crate suite on the final tree (the window cut it; the pre-fix baseline and the
  nf-game run bound the risk — only `drone.rs`/`searchlight.rs` changed).
- G-035's `Door_ReactToHit` exact proxy-list path is the same collide machinery, still open.

## Shared files touched (small, additive, listed)

- `crates/nf-game/src/drone.rs` — the `Drones` struct gains `bullet_path_stops: Vec<f32>` on its
  own lines at the end (lesson 4), its constructor line in the lesson-4 block, a `.clear()` next
  to the other path clears, one `stop_t` push after the `(best, tw)` match in
  `update_player_bullets` (min of the world/drone stop fractions, inclusive), the searchlight
  `create` call gains the collision-closure argument, and the `update` call passes the two new
  slices. No behavior of any other system changed; `bullet_paths`/`bullet_path_weapons` are
  untouched.
- `docs/WORKLOG.md` — this task's entry appended (merged with origin/master's, both kept).

## Live checks for the main agent

- PCSX2, 07000006 courtyard: (1) shoot a lamp housing square-on — the light breaks (droop, glass
  0xE6, the spot goes out); (2) shoot the mount pole just below the housing — the light must NOT
  break (the kind-1 gate); (3) if either differs, a `gdbwatch` on the head's `obj+0xD0`
  (e.g. head 016e95a0's address on 07000005) during the shot shows whether the mirror node
  arrived (the writer side) vs the walk (the reader side).
- nf-view renders: after the break, `--bond 4.8,42.1,-16.0,-2.2,0` and
  `7.1,48.6,-23.6,-1.4,0.2` (07000006 courtyard) — the head parts hidden, the spot's light gone;
  a mount-shot control render shows the light still sweeping.

## ntfy

(sent at finish; see the finish log if it failed)
