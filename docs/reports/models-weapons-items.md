# Report: models-weapons-items

Branch `s/models-weapons-items` (from master 2b030c5; head a46b936). Lane: every weapon's
first-person and third-person model, every gadget's models, every pickup's model, dropped weapons
and the projectiles. The golden lives in `crates/nf-view/tests/models_weapons_live.rs` (4 tests,
NF_ISO); the lane inventory with the per-level table and render spots is
`docs/models/weapons-items.md`; the WORKLOG entry is `models-weapons-items`.

## What's done

1. **The projectile bodies — a whole model class that drew nothing.** The weapon field table
   missed `weapon_data+0x5C`: `Bullet_update` (0x12e7b0) state 0, disassembly 0x12e8f0-0x12e900,
   sets the flying bullet's drawn mesh from `weapon+0x5C` (`hashtable_set_object_to_entity_gfx` +
   `View_SetDrawInAllViews`; `0xFFFFFFFF`/0 = none — those bullets show the 0x02000123 trail).
   Measured [L `ee_start_05.bin`]: frag 0x35 -> 0x02000726, stun 0x36 -> 0x0200071f, mine 0x37 ->
   0x02000a6e, rocket 0x3A -> 0x0200074f, darts 0x3B..0x40 -> 0x020007b3, Q-Worm 0x46 -> 0x0200025f.
   The port drew no projectile at all. Fixed: `Grenade`/`Bullet` carry the mesh; the scene pools
   Effect slots (8 grenades, 32 drone bullets, 8 of Bond's bullets; one instance per slot x distinct
   resolving mesh) posed in `frame_state` at `Mat_Align2Dir(dir, up (0,1,0) [L @0x2d6210], fallback
   (1,0,0) @0x2d6220)` — `Bullet_init` 0x12c5a0:185 aligns the bullet matrix to its direction and
   the per-tick tail (0x12f1ac-0x12f1e4) moves `obj+0x30..0x38` only. Documented in
   `docs/atlas/weapon-data.md` + `docs/formats/weapon-data.md`.

2. **The session object table — Tower-2's 64 flash-less guards.** The flash meshes 0x02000261 /
   0x0200044a and the SMG's held mesh 0x0200025c are on NO archive from 07000011 on [D scan], yet
   07000011-16's guards fire those weapons: the game's map-object hashtable accumulates every
   archive loaded in the session (`hashtable_delete_hashtype`'s only callers are the cut-scene pack
   loader 0x1d69e8/0x1d6ab0 [M decompile grep]). `load_impl` now merges every missing weapon-model
   hash (the +0x80/+0x4C/+0x5C trio and the view-model sleeves) from the latest earlier story
   archive that holds it (the story order is `ResetMap_LevelCode2Img` 0x1d5258's case order — the
   level codes ascend in mission order [docs/systems/levels.md section 1]). Maps append (no existing
   (map, object) index shifts) and `by_hash` stays first-wins: every local resolution unchanged.

3. **Dev view models (folded from 190-fix-dev2).** With `--dev`, the missing first-person skins
   (+0xDC) merge from their holding archives (a skin-hash -> archive index built once over every
   type-3 entry on the disc), with their maps/skeletons/sequences and the weapons' kind-5 anim
   scripts; `Scene::build` then gives every selectable weapon a view model and the panel catalog
   reports none missing. Normal play is untouched (pinned by the sister assertion).

4. **The per-level golden** (numbers only): armed drones, their held guns and flash meshes, the
   placed pickup models, and the projectile meshes of every firable weapon (drone param-9 + param-11
   grenade weapons + pickup ids + the level's cold loadout) resolve through the level archive OR the
   session union. Placed pickups resolve on all 20 story levels (the O2 armour-plate class is clean
   level-wide).

5. **The shipped game's own artifacts** (measured, pinned in the golden, not "fixed" against the
   game): the stun grenade flies invisible on 07000001/03 (no archive the session has loaded by
   those levels holds 0x0200071f [D]; `Bullet_update` skips the draw on a hashtable miss [M]); and
   07000004/16's SMG guards draw unarmed (skin 0x0500001e has no datum 4 [D];
   `AnimDatumSetEntity` 0x1c7848 no-ops on a missing datum id [M]).

## How it was verified

- `models_weapons_live.rs`, all NF_ISO, `--test-threads=1`, green on the final head:
  - `golden_weapon_models_resolve_on_every_story_level` — the 20 story levels, counts pinned via
    `nf_formats::golden::diff_tsv` (numbers and the four documented miss entries only).
  - `a_thrown_grenade_draws_its_weapon_mesh` — a frag thrown on 07000005 through the real input
    path (`panel::apply` give -> `Input.switch_to` draw -> `step_bond` trigger pull); the mesh
    attachment tracks the grenade within 1 unit; when no grenade flies the slot collapses to the
    FAR point (sister).
  - `tower_2_guards_get_their_muzzle_flashes_from_the_session` — the merged meshes land in the
    sim's model table and >= 32 flash attachments exist on 07000011 (pre-fix: 0).
  - `dev_gives_every_selectable_weapon_a_view_model_on_07000005` — every selectable weapon has a
    view model, the GPU skin layout covers every character, the panel catalog shows no "no
    1st-person model here"; without `--dev` the L96A1 stays view-model-less (sister).
- **Fail-first shown for every fix** by neutralizing exactly the fix (the draw branch / the session
  merge / the dev merge) and re-running: each test failed at its named assertion, then passed
  restored.
- Full suite (`cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
  -p nf-audio -p nf-view -- --test-threads=1`): 102 targets ok, 472+ tests passed; the only
  failures are the 3 allowed slim-ISO `disc_golden` tests. Master was merged twice mid-lane
  (models-characters' G-021 merge, then models-world/fix-finale-blockers) with both sides' logic
  kept; the lane tests and `models_characters_live` re-ran green after each merge.

## What's left (in this lane)

- **The "torn guns" user report** has no playtest entry to reproduce from (nothing in
  `docs/playtests/2026-10-11.md` names it); the model-side causes it could be (a held mesh missing
  its flash, a drop drawing at another pickup's slot) are covered by tests. Needs a fresh
  screenshot with the level and spot named to chase further — I did not guess a fix.
- **R1 over-bright on guns**: the 0x800 TWEAK root cause is fix-render2's (characters.md G-042
  pinned the source texels as non-white); nothing in the gun path adds brightness. Watch the
  section-4 render spots.
- **Grenade body orientation**: `Mat_Align2Dir(dir)` mirrors `Bullet_init` [M]; whether the mesh's
  long axis is +x or -x is eyeball-only — the render spots' throw check settles it.
- **The gadget functions** (grapple/decryptor/laser) are another lane's function work
  (docs/port-map/gadgets.md); their MODELS (FP skins, the Q-Worm projectile mesh) resolve and draw.

## Shared files touched (additive, minimal)

- `crates/nf-view/src/lib.rs` — the two merges in `load_impl` + `load_dev`/`load_with_carry_dev`
  wrappers (the G-021 block from the characters lane is untouched; the weapon-model merge runs
  after it).
- `crates/nf-view/src/scene.rs` — `Attachment.projectile`, the projectile slot pool, the
  `frame_state` posing branch, the grenade push's `mesh:` (master's `chain` parameter threading
  merged around it).
- `crates/nf-formats/src/weapon.rs` — the `projectile_mesh` accessor.
- `crates/nf-game/src/{bullet,grenade,drone}.rs` — the `mesh`/`weapon` fields on the projectile
  structs + their spawn sites; the bullet.rs/grenade.rs test literals updated.
- Test files with grenade literals gained the `mesh:` field (walk_07000001/04/14, fix_actors,
  fix_finale).
