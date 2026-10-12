## Your task: `models-weapons-items`, every weapon, gadget, pickup and projectile model

Lane: every weapon's first-person view model (with Bond's hands and sleeves) and third-person held model, every
gadget's models, every pickup's model (weapons, ammo, armour, keys, 007 bonuses, mission items), dropped weapons, and
projectiles (grenades, rockets, darts, the Q-Worm).

**The user's priority for today:** every model in the game ported, and ported correctly, before they get home tonight.
Four tasks split the work: `041-models-world`, `042-models-characters`, `043-models-weapons-items`, `044-models-misc`. Stay in
your own lane, and list anything found in another lane in your report rather than fixing it.

**What "correct" means here, and how to prove it without a GPU** (the main agent renders; you can't):
- **Complete:** every model the game can show on that level resolves, with no placement, glist, leaf, batch, texture
  REF or skin skipped or unresolved. Count them per level and pin the counts in a golden (numbers and hashes only).
- **Exact:** decoded geometry matches the disc data. Cross-check the vertex, triangle and batch counts against the
  VIF unpack totals of each packet. Where an independent tool exists (the `work/nfre` nightfire-re dumps, facts only),
  compare counts as sets. UVs, vertex colours and normals in range; no NaNs; the bounds match the glist's own.
- **Bound right:** each batch's texture, palette, alpha/blend/test/depth state (the leaf's GS A+D writes) and
  animated-texture frames are the ones the game uses. Compare the port's per-batch draw state with the decoded GS
  writes, per object.
- **Placed right:** world matrices from the placements (bit-exact where the game's maths is ported), LOD choice,
  attachment datums for held and attached items.
- **Visible:** for every class of model, write a list of `--bond x,y,z,yaw,pitch` spots (one per level where it
  appears) into `docs/models/<lane>.md` so the main agent can render and eyeball each one today.

Fix every defect you find, each with its evidence and an NF_ISO test (fail first, with a sister assertion). Write the
model inventory for your lane into `docs/models/<lane>.md`: every model, per level, its status (ported / fixed / still
wrong + why).

**Shared files** (`scene.rs`, `gpu.rs`, `lib.rs`, `ps2gfx.rs`, `level.rs`): additive, minimal, listed in your report.
Other model workers edit nearby, so pull `master` before you start and before every push.

**Known open items in this lane:**
- **Attachments:** each weapon's datum positions (muzzle, hand), the flash mesh, and the held model's datum on guards
  (`DroneAnim_EventFunc` weapon swaps, G-023).
- **The user's reports:** "torn" guns, an armour plate pickup whose model didn't draw (O2, fixed on 07000006; check every
  level), and guns and characters over-bright (R1, the TWEAK).
- **Pickup models per kind and per level:** the pickup table on every level, each resolved to its model, rotating and
  bobbing as the game does (`Pickup_Update`).

**Folded in from the old task `190-fix-dev2` (do it as part of this task):**

The dev panel (`crates/nf-view/src/panel.rs`, http://127.0.0.1:7007 with `--dev`) can give any weapon, but most show "no 1st-person model here": each level archive only ships the first-person skins of its own weapons. The user wants every weapon to have its first-person model on every level.
1. At startup, scan every level archive on the disc (FILES.BIN) for first-person weapon skins (`weapon_data+0xDC`, per weapon, after `Player_Init`'s overrides) and their sleeve meshes, and build an index: skin hash → archive.
2. When a level loads with `--dev`, load the missing first-person skins (and their textures, skeletons and sequences) from the other archives and add a view model for every weapon, the same way `Scene::build` builds them. GPU skinning (`Scene::skin_layout`) must include them.
3. The panel's catalog then shows a model for every weapon. Test: on 07000005, every selectable weapon has a view model.

This is dev mode only. Normal play must be unchanged: no extra archives loaded without `--dev`.

Files: `crates/nf-view/src/{lib,scene,panel}.rs`, and the archive helpers in `crates/nf-formats`.

Files: `crates/nf-view/src/{lib,scene,panel}.rs`, `crates/nf-formats` (the archive index),
`crates/nf-view/tests/models_weapons_live.rs`, `docs/models/weapons-items.md`.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/models-weapons-items`.

---
claimed-by: sandbox1 at 2026-10-12T15:47Z

---
## Report (done, 2026-10-12T17:37Z)

