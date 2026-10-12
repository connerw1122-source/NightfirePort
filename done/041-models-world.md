## Your task: `models-world`, every level's world geometry and placed static objects

Lane: the room/cel geometry and every static placed object on all 27 on-foot levels (`docs/systems/levels.md`):
walls, floors, terrain, props, decorations (0x47), breakables in their intact state (0x20), SimpleScript props (0xE2),
doors, sky objects, and the shadow and decal geometry baked into levels.

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
- **G-018:** the per-object GS state (alpha blend/test, CLAMP, filtering from the first CNT's `DIRECT` A+D writes) and
  animated textures. Audit every object's state against the port's `Draw` and fix the mismatches.
- **G-014:** the batch semantics in the VU1 microcode (ACTION.ELF `.vutext` 0x241c80): signed V4-8 = normal + no-kick
  flag, UV use. Confirm from the microcode, or measure on the disc that the port's reading holds for every batch.
- The cel-pass flag 0x10 skips ("18 skipped by the cel pass" on 07000005): check that the game skips them too.
- The main agent moved alpha-shaped list-0 textures after the opaque draws (`gpu::alpha_shaped`, R2). Check that
  stand-in against the real draw order (`View_DrawCelGlist` 0x1e6538 then `View_DrawObjList` 0x1e69a0), and replace it
  with the cel/object split if you can build it.

Files: `crates/nf-formats/src/{ps2gfx,level,map,texture}.rs`, `crates/nf-view/src/scene.rs` (the world build),
`crates/nf-view/tests/models_world_live.rs`, `docs/models/world.md`.

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

**Branch:** `s/models-world`.

---
claimed-by: sandbox3 at 2026-10-12T14:34Z

---
## Report (done, 2026-10-12T16:35Z)

