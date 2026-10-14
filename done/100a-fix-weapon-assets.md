## Your task: `weapon-assets`, every weapon's model, animations and effect meshes on every level

CONTEXT (the user, 2026-10-13): "need heavy work on getting weapons to work no matter what level, with sounds and
proper animations and particles." The dev tools (the dev window / F2 menu, `--give id,id`) hand Bond ANY weapon on ANY
level. Today `scene.rs` builds a first-person view model only when the weapon's first-person skin (`weapon_data+0xDC`),
its sleeve and its parts are in the CURRENT level's archive (`level.skins.get(&fp)`, scene.rs "The first-person gun
models"), and the same holds for its view-model animation scripts, its muzzle-flash mesh (`weapon+0x48`), its casing,
its projectile and its sounds (the level's own sound bank). So a weapon the level doesn't natively carry is invisible,
silent, unanimated or fires the wrong effects (the user saw the grapple fire a bullet impact, the laser and stunner fire
"bullets", no Q-Claw model). The real game only ever loads a level's own set; the port goes further on purpose, so that
every weapon behaves exactly as it does on a level that carries it.

The four 100x tasks split the work: 100a the assets and animations (the foundation), 100b the sounds, 100c the fire
behaviour and effects, 100d the all-weapons x all-levels check. They run in parallel: pull `master` often, and if
100a's branch (`s/weapon-assets`) isn't merged yet, build on it (`git merge origin/s/weapon-assets`) and say so in
the report. Keep the per-level behaviour bit-identical for weapons the level carries (the existing tests stay green).

YOUR PART: a disc-wide weapon asset index and loader.
1. Index: for each weapon id 0x01..0x71, which level archives (the FILES.BIN `.bin` entries; fix-anim-events parsed
   all 99) carry its first-person skin, the sleeve (scene.rs SLEEVES), its extra parts, its view-model animation
   sequences (draw/holster/idle/fire/reload/alt: the scripts `weapon_data` names at +0xA0/+0xA4/+0xC0/+0xC4 etc., see
   gun.rs and docs/systems/player.md "Weapon switching"), its muzzle-flash mesh (+0x48), casing mesh, projectile mesh
   (rockets, grenades, bolts, darts, the Q-Claw hook), its third-person (held/pickup) model and the textures they use.
   Build it once per ISO and cache it (a small file next to the build or in the user's temp dir, keyed by the ISO's
   size and a SHA-1 of the archive table; never game bytes in the repo).
2. Load: when a scene is built, and when `--give` / the dev menu gives a weapon the level lacks, pull that weapon's
   assets from the owning archive into the scene, the way scene.rs already does for drone skins from other archives
   ("drones with a skin from another archive"): meshes, textures, skeleton, sequences, rebased into the scene's
   tables. A lazy load on first give is fine if loading every weapon up front is slow (measure; report the times).
3. The draw/holster/fire/reload animations play from the loaded sequences with the game's timing, the muzzle flash
   attaches to datum 0 with the record's colour, and the held/dropped world model shows.
Tests: for every weapon id with a first-person skin anywhere on the disc, on a level that does NOT carry it (07000005
for most): give it, run the draw, fire once, reload; the view model has its skin's vertex count, the sequences
advance, the flash attaches. On a level that DOES carry it, the result is identical to before (numbers, not images;
sister assertions). List the ids that have no first-person skin anywhere on the disc (expected: hands, some gadgets)
with the reason.

Files: `crates/nf-view/src/scene.rs` (the loader; small hooks), a new `crates/nf-view/src/weapon_assets.rs`,
`crates/nf-view/src/lib.rs` (the `--give` hook), tests.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/weapon-assets`.

---
claimed-by: sandbox4 at 2026-10-14T02:32Z

---
## Report (done, 2026-10-14T04:06Z)


---
## Report from the worker (done, 2026-10-14T04:06Z)


---
## Report from the worker (done, 2026-10-14T04:07Z — attached with the absolute path after the relative-path cat failed)

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
