## Your task: `models-misc`, every remaining model: vehicles, effects, broken pieces, front-end and HUD models

Lane: everything not in the other three lanes:
- **Vehicles and big movers** in the on-foot levels: the 05 truck, the 08 gondola and cable car, the helicopters
  (`Copter_Create`), the cars (0xda), lifts and platforms;
- **Effect meshes:** the explosion visual (script 0x6000052, `Explode_Create`), the muzzle-flash meshes' variants,
  smoke and gas meshes (`Gas_Init`), coronas and glows (0xFC), searchlight beams and spots (the user saw "white blobs" at
  the heads, a "harsh beam", O5), the sky's lightning;
- **Broken states:** every breakable's broken pieces (`Break_Create` debris), destroyed doors and objects (`Destroy`);
- **Front-end and HUD models:** the menu backgrounds and any 3D models the front end or the pause menu draws;
- **Cut-scene props:** objects that exist only in scripts (`Script_CreateEntity`).

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

Files: as needed per item, keeping to new files where possible; `crates/nf-view/tests/models_misc_live.rs`,
`docs/models/misc.md`.

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

**Branch:** `s/models-misc`.

---
claimed-by: sandbox3 at 2026-10-12T16:36Z
