## Your task: `models-characters`, every character model: Bond, guards, civilians, allies, bosses, cut-scene actors

Lane: every skinned character on the disc. Every skin each level places or spawns, every cut-scene's actors, Bond's
third-person and tuxedo variants, heads and accessories, and every skin variant's textures.

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
- **G-021:** 33 drone placements whose skin isn't in their level's archive. Load the common animation file
  (`CommonAnimFile`, LoaderLoad type 4) and the other archives, and the sleeve substitutes (`AnimSleeveGetEntity`).
  Every one of them must draw and animate.
- **Skinning exactness:** the two-bone blend and the palette (`gpu_skin_live` pins GPU = CPU). Check the CPU pose
  against the game's with `nf-extract pose-check` on every EE snapshot (`work/ee_start_*.bin`), and extend it to more
  skins and frames. G-024: the world matrices are up to 4.6e-5 off; note it, `250-fix-bitexact` owns it.
- **Tint and ambient:** the 0x800 TWEAK / ambient multiplier per character (the user saw over-bright guns and
  characters, R1). Check every skin's colour multiplier against the game's.
- **Playtest F3:** Bond's model vanishes when he changes into the tuxedo (the dressing cut-scene on 07000006; the
  third-person skin 0x050000d, `Player_Init` 0x1910f8:96-107). Fix the skin swap.
- **The 07000002 bathhouse civilian texel check (G-042)**, and any character drawn in T-pose, with missing parts or
  with the wrong texture on any level: sweep every character on every level for an unposed or unresolved state.

Files: `crates/nf-formats/src/{character,ps2gfx}.rs`, `crates/nf-view/src/scene.rs` (the character build),
`crates/nf-view/tests/models_characters_live.rs`, `docs/models/characters.md`.

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

**Branch:** `s/models-characters`.

---
claimed-by: sandbox1 at 2026-10-12T14:48Z
