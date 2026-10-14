# fix-weapon-assets (100a): the disc-wide weapon asset index + loader

Branch `s/fix-weapon-assets` (built on `origin/master` @ 810c109, merged mid-task). Task 100a of
the four 100x weapon tasks: the assets-and-animations foundation so `--give` / the F2 dev menu can
hand Bond ANY weapon on ANY level and it draws, animates and fires with its own model, flash and
projectile meshes.

## What was wrong (reproduced before fixing)

`Scene::build`'s first-person section (scene.rs "The first-person gun models") builds a view model
only when the weapon's fp skin (`weapon_data+0xDC`) is in the CURRENT level's skin table, and the
same hold for the sleeve, the skeleton, the `07F` sequences and the kind-5 view scripts. The test
`merge_resolves_every_asset_on_07000005` (written first, against the pre-fix behaviour) showed
07000005 lacks **53 weapon ids** — including 0x34 (frag grenade), 0x40 and 0x6f — whose fp skins,
meshes, sequences and scripts simply didn't resolve on that level. A `--give` of one of them drew
nothing.

## What was built

- **`crates/nf-view/src/weapon_assets.rs` (new)** —
  - `DiscIndex::for_iso`: one pass over every `.bin` archive in FILES.BIN, tabulating skin hash →
    archive (kind-3 `05` entries) and kind-5 script hash → archive (`Script::parse`'s hashcode,
    `06` entries). **Cached** as a small text file in the temp dir
    (`nf-weapon-index-<ISO size>-<FILES.BIN-table SHA-1[0..12]>.txt`); no game bytes, not in the
    repo. Cold scan ~0.3 s, cached load ~0.01 s (measured in `weapon_index_covers_the_disc`).
  - `Wanted::{None, Ids, All}` + `merge_weapon_assets`: pulls the wanted weapons' fp skins from
    their owning archives into the level's tables — skins, the maps holding their `main`/`extra`
    meshes (textures ride the map), the sleeve set, the skeletons, the paired `07F` sequences and
    the kind-5 view scripts (+0xA0 idle / +0xA4 reload / +0xA8/+0xAC reload-start/end / +0xB0/+0xB4
    fire / +0xB8/+0xBC / +0xC0 draw / +0xC4 holster / +0xC8 variant / +0xCC/+0xD0/+0xD4 idle-alert,
    per `nf_formats::weapon`) — with a script top-up pass from the index when a wanted script sits
    outside the opened archives. First-wins everywhere, appended maps never shift an index, so
    every resolution the level already had is untouched (asserted per-key in the test).
  - This generalizes the existing `--dev`-only `merge_dev_view_models` (folded 190-fix-dev2), which
    it replaces; `--dev` keeps the lend-everything behaviour, a normal load merges exactly the
    given ids. A safety guard the dev merge lacked: appended `07F` sequences never land on a
    skeleton any character placement animates (the per-skeleton lists are order-sensitive gameplay
    data — the same reason the loadable-pack path uses the `skeleton | 0x80` sentinel).
- **`crates/nf-view/src/lib.rs` (hooks, additive)** —
  - `load_weapons` / `load_with_carry_weapons` (new pub fns) → `load_impl(.., give: &[u8])`.
  - `run()` parses `--give` ids before the load and calls `load_weapons` (the merge runs before
    `Scene::build`; measured: +0.07 s for one weapon on 07000005 with a warm index, ~0.4 s total
    load — eager-all not needed).
  - The dev menu / browser panel give paths (`poll_panel`, `apply_menu_action`): when the given
    weapon's assets are absent, the App answers with a **lazy same-level warm reload**
    (`load_with_carry_weapons`) that merges the weapon's assets, re-places Bond where he stood and
    re-issues the switch. One hitch per newly-given weapon (the game never gives cross-level
    weapons; this replaces the missing pieces of the dev affordance).
  - weapon_data is built once per load now (the merge consumes the same table the DroneContext
    does; `--dev` used to run the EE initialiser twice).
- **`crates/nf-view/src/scene.rs`: no changes needed** — the view-model builder, muzzle-flash
  attachment (datum 0, colour +0x56/55/54) and projectile pooling resolve the merged weapons on
  their own once the tables hold them.
- **`crates/nf-view/tests/weapon_assets_live.rs` (new, 5 NF_ISO tests)** — index + cache; the
  surgical merge resolving EVERY asset class for ALL 53 cross-level weapons on 07000005 (main
  meshes — except the sleeve entries, which `AnimSleeveGetEntity(0)` substitutes per
  `AnimProcessSkinData` 0x1bd500:159-166 — extras, skeletons, sequences, all view scripts, +0x48
  flash, +0x80 third-person, +0x5C projectile) with per-key by_hash/skin unchanged assertions;
  draw → fire → reload through `Sim::step_bond` for a 3-id sample (equip per `Player_EquipWeapon`
  0x1a8a58 + the `--give` switch path, then the switch machine's +0xC4/+0xC0 scripts); the
  native-carry sister (07000001: plain vs merged loads identical — skins, meshes, datums, scripts,
  view-model slots, no characters added); the no-model list.

## Measured (the task's "measure; report the times")

- Index: cold ~0.3 s once per ISO, then cached (temp file keyed by ISO size + FILES.BIN SHA-1).
- Surgical merge (1 weapon, warm index): **0.07 s** — lazy-vs-eager debate settled: eager-all is
  also cheap (the 53-weapon merge on 07000005: 2 archives, 28 skins, 50 maps, 160 sequences, 364
  scripts, 0.07 s), so `--give` at startup stays simple; the dev-menu give keeps the lazy reload
  for mid-game.
- Full load 07000005 + 53 weapons: 0.43 s headless.

## Suite

Full suite on the final tree (merged with origin/master), serial, fresh binaries in bounded
batches (the sandbox kills detached processes): **752 passed, 3 failed — exactly the 3 allowed
`disc_golden` tests on the slim ISO (bigf header/tables/directory golden) — 27 ignored**, over
nf-view (172 files, 549 passed / 0 failed), nf-game (32 files, 116 / 0), nf-formats + nf-audio
(87 / 3).

## Data notes (task item: ids without a first-person skin anywhere)

18 ids have no fp model (`+0xDC == 0`): 0x39 and the 0x47+ gadget/consumable tail (57, 71–73,
95–105, 107, 112, 113 — the last are past the selectable range). Every gun-class id models. The
full list with reasons prints in `ids_without_a_first_person_model`.

## Quirk found on the way (for 100c)

`Inventory::equip_weapon` marks `owned[base(id)]` (record +0x02), while the Sim's input switch gate
reads `owned[id]` — a base-variant give through `input.switch_to` is refused; `--give` bypasses it
with the direct `gun.switch_request` write [M lib.rs:2385]. Worth a look in 100c.

## Shared-file changes (for the merge)

- `lib.rs`: the hooks above + the replaced dev merge fn; the App ctor union-merged with master's
  `devwin`/`audio_paused` fields.
- `scene.rs`, `drone.rs`, `player.rs`: untouched.
- `docs/WORKLOG.md`: my entry appended (union-merged with master's).

## Live checks for the main agent (nf-view renders, numbers passed here)

- `nf-view 07000005.bin --give 0x40 --shot /tmp/qclaw.png --bond <a rooftops spot>` — the Q-Claw's
  first-person model drawn with its sleeve (pre-fix: invisible).
- `nf-view 07000005.bin --give 0x34 --shot /tmp/frag.png` — the frag grenade drawn; firing shows
  the throw arm (the fire script +0xB0 now loads).
- `nf-view 07000005.bin --give 0x0e --shot /tmp/gun.png` — a cross-level gun; muzzle flash on
  datum 0 with the record's colour when firing.
- `nf-view 07000001.bin --give 0x02 --shot /tmp/p2k.png` — the native path unchanged.
- In PCSX2: give the Q-Claw on 07000005 and fire at a grapple point — the game shows the claw's
  own view model + hook projectile (the port now merges the same assets).

## What's left (for 100b/c/d)

- Sounds ride the level's bank — already fully loaded per level (nf-game::sound::level_bank); the
  per-weapon id remap exists (scene.rs `remap_op1_sound`); 100b owns the per-id mapping work.
- Fire behaviour/effects (impact types per weapon) are 100c; the projectile meshes now resolve.
- 100d's all-weapons × all-levels sweep can build on `weapon_assets_live.rs`'s cross_level_ids.
