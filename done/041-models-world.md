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


---
## Report (done, retry with content)

# models-world — report

Branch `s/models-world` (from `origin/master` d93772b). Task: every level's world geometry and
placed static objects on the 27 on-foot levels.

## What's done

**The inventory is complete and pinned.** Every placement of every story level resolves to an
object — 0 unresolved across all 27 codes (07000001–0700004b). Every glist, leaf, batch and
texture REF of every map decodes. The per-level counts (maps, placements, statics drawn, cel-pass
skips, leaves, batches, vertices, triangles, texture-REF anomalies, non-finite UVs) are pinned in
`crates/nf-view/golden/models_world.tsv` and asserted by
`models_world_live::story_levels_match_the_model_inventory_golden` (numbers only, tamper sister
included). The full per-level table and the placement-type status are in `docs/models/world.md`.

**G-018 closed (per-object GS state).** Measured over every DIRECT A+D write of all 27 levels: the
PS2_GFX packets carry only ALPHA_1 (0x42), TEST_1 (0x47) and ZBUF_1 (0x4e) — no CLAMP (0x08), no
TEX1 (0x14) anywhere, so filtering/wrapping are global texture-upload state and the port's fixed
sampler is not a per-object mismatch. Two real defects were found and fixed:

1. ALPHA_1 0x58 ((Cs−0)·**Ad**+Cd) and 0x64 ((Cs−Cd)·FIX0+Cd) — 66 batches, only on 07000041's
   `EnvModTest_*`/`polySurface2` objects — were drawn as the normal blend (opaque overlays). The
   port now implements blend 4 (additive gated by destination alpha) and blend 3 (keep-destination;
   the GS still depth-tests/writes and alpha-tests). Nothing patches the FIX field in the game's
   load/draw path (psiCreateEntityGfx 0x1171b0, DrawThisBox 0x1230d8, CacheAndFillNewGlistTextures
   0x11f180 all re-checked), so the disc values are final. Every fix cites the game address; the
   test `draw_state_matches_the_decoded_gs_writes` fails against the old mapping (its sister
   asserts exactly that) and runs the mapping over the real 07000005/07000041 batches.
2. TEST_1 0x5000b (GEQUAL 0 = everything passes) and ATST ALWAYS were mapped to "alpha ≥ 1/128",
   rejecting a=0 texels and skipping their depth write. Now mapped to no alpha test.

**The real list-0 draw order (the R2 "navy blocks" hole's root cause).** Decompile re-read:
`View_DrawObjList` 0x1e69a0 sorts one mixed array of cel and object entries; `View_AddCels`
0x1e6ba0 pushes every runtime cel's primary glist into list 0 with the synthetic key
−90000+10k in cel-chain (≈ placement-parse) order — the push has no list choice; only
`View_AddObjects` 0x1e62d8's objects (and the never-occurring +0x18 LOD chain records) carry
distance keys. So list 0 = all visible statics in placement-parse order, then dynamic objects
front-to-back. The port sorted everything by its own centre distance, so a baked shadow decal with
a nearer centre could draw before the wall it lies on and its depth write rejected the wall — the
symptom fix-render4's `alpha_shaped` reorder stood in for. The port now uses the game's key exactly
(`Material::chain` in scene.rs; gpu.rs sorts pass 1 by −90000+10·chain then dist²); the
alpha_shaped hack is retired (the classifier stays, still pinned by `decal_order_live`);
glist-flag-0x1 statics draw in list 0 (primary push has no list choice — only dynamic objects pick
the alpha list by flags). `statics_draw_in_parse_order_and_decals_after_their_walls` pins it on
07000007 (statics-first, parse order, decals after an overlapping opaque static; sisters assert the
predicate).

**Also measured and documented** (`docs/models/world.md`):

- The cel-pass skip (glist flags bit 0x10) is the game's own — `View_AddCels` never pushes a record
  with flags & 0x10 (decompile re-read). The golden pins each level's skip count.
- LOD blocks (map block 0x1f, `parsemap_block_LOD` 0x1d1f10, the switch-channel glist swap in
  `control_movement_object_handler` 0x1336f8) never occur on the disc (0 blocks in 2,230 maps) —
  no hidden swap-variant models exist.
- The one out-of-range texture REF (0700000d `WallHug_Blocker1_Long`, index 185 ≥ 185 textures)
  sits on a flag-0x10 blocker the game never draws — no port defect.
- UV semantics confirmed on the disc (G-014's measurement): 10,966,959 coords in UV range; the 303
  outliers are U-022's garbage floats (no ST·Q tail).
- The +0x30 field exceeds the decoded triangle count on 690 leaves (per-leaf U-020 view).
- `--bond x,y,z,yaw,pitch` eyeball spots for every model class (door, breakable, SimpleScript prop,
  sky, baked decal/cutout, env-map) per level: docs/models/world.md §5 (generated by the
  `bond_spots` probe from the real placements).
- Every placement type the port does not draw is listed per level in docs/models/world.md §6 for
  the lanes that own them (0x31 fuse boxes up to 129 on 0700001b, 0x28 sensors, 0xd8 rigid bodies,
  0x3b DynamicObjects, 0xea/0xeb touch triggers, 0xf2 emitters, …). Nothing was fixed outside my
  lane.

## How it was verified

- `story_levels_match_the_model_inventory_golden`: parses all 27 story levels through the real
  `Level::parse` path, asserts 0 unresolved placements and diffs the counts against the golden
  (sister: a one-character tamper is caught).
- `draw_state_matches_the_decoded_gs_writes`: the `Draw::from_gs` unit table (six ALPHA shapes ×
  TEST/ZBUF) + every real batch of 07000005 and 07000041 maps to a representable state, with the
  0x58/0x64 batches present on the disc (the sisters fail against the old mappings).
- `statics_draw_in_parse_order_and_decals_after_their_walls`: the renderer's list-0 sort replicated
  over 07000007's real materials (statics first, parse order; the R2 decal ordering).
- `uv_range_histogram`: the G-014 disc measurement over all 27 levels.
- The survey/probe tests (`models_survey`, `probe_lod_blocks`, `probe_data_defects`,
  `probe_blocker_object`, `probe_unusual_alpha_states`, `bond_spots`) are opt-in tools (NF_SURVEY /
  --ignored) left in the tree for the next sessions.
- Full suite before push: `NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game
  -p nf-formats -p nf-audio -p nf-view -- --test-threads=1` — 97 test binaries ok; the only
  failures are the 3 allowed `disc_golden` full-disc manifest/bigf goldens on the slim ISO.
- No GPU in this sandbox, so the visual check of the two render changes (07000041 env-map object;
  flag-0x1 statics that moved from the alpha list to list 0, e.g. 07000005's pine trees) is owed to
  the main agent's render pass — docs/models/world.md §8 lists the spots.

## Shared files touched (additive, minimal)

- `crates/nf-view/src/scene.rs`: `Draw::from_gs` rewritten (documented, pub for tests);
  `Material.chain` added; the World pass/order wiring.
- `crates/nf-view/src/gpu.rs`: blend 3/4 pipelines; the list-0 sort (the `late`/alpha_shaped hack
  removed).
- `crates/nf-formats/src/map.rs`: the 0x1f LOD block-id const (parse-side only; nothing consumes it
  — the block never occurs).
- Docs: `docs/formats/ps2gfx.md` (the GS-state section), `docs/systems/render.md` (the draw-order
  section), `docs/GAPS.md` (G-018 closed, G-014 measured), `docs/WORKLOG.md` (the entry), plus the
  new `docs/models/world.md`, `crates/nf-view/golden/models_world.tsv`,
  `crates/nf-view/tests/models_world_live.rs`.

## What's left

- G-014's microcode reading itself (no VU disassembler in this sandbox; `.vutext` located at
  ACTION.ELF 0x241c80, file offset 0x13bb80, 0x2880 bytes — the disc-side measurement now stands in
  for the batch semantics; see docs/models/world.md §8).
- The other model lanes' undrawn meshed placement types (docs/models/world.md §6).
- The render eyeball of §5's spots (main agent).
