## Your task: `fix-cel-order`, the static draw order must be the game's cel-chain order

**The main agent's measurement (2026-10-13, master after fix-render-artifacts):** on 0700000c the baked
`pylonshadow` decal (material 3380, texture 002d:003b, `Draw { pass 1, blend 0 normal, zwrite true }`) has chain
ordinal **68**, and the ground room under it (materials 2818-2838, chain **88**) draws AFTER it. List 0 sorts statics
by `−90000 + 10·chain` (models-world, `Material::chain` = placement-parse order). So the decal darkens the empty
background (the fog clear colour) and its depth write rejects the ground: the user's **pale lattice** on 0c (the
real game shows a dark lattice: main agent's PCSX2 reference, 2026-10-12). fix-render-artifacts' cull-radius fix is
right but does not change this; the static draw state of the decal is right too.

The game's order is the cel chain: `View_AddCels` 0x1e6ba0 gives the root cel (`GfxList+4`) −90000, then walks
`cel+0x18` adding +10 per cel. models-world assumed that chain is "≈ placement-parse order". On 0c it can't be. But
a plain reversal would break 07000007's R2 decals (`statics_draw_in_parse_order_and_decals_after_their_walls`, the
user-verified fix), so the real chain order is neither: find it.

**Do:**
1. Measure the real chain: walk `GfxList+4 → +0x18 …` in a real EE snapshot (`work/ee_start_05.bin`, and the main
   agent can save one on 0700000c for you: ask) and record each cel's glist/object in order. Then find the code that
   links cels into `+0x18` at load (`parsemap_create_dynamic_objects` 0x1d04f0 / the cel creators; prepend or
   append, and in which order the placements are visited).
2. Port the exact order into `Material::chain`, and check 0c's pylon shadow draws after its ground and 07000007's
   decals still draw after their walls (both pinned as tests; render spots: `0700000c --bond
   14.22,16.29,-147.02,0.973,-0.300` and the 07 R2 spots in the existing test).

Files: `crates/nf-view/src/scene.rs` (`Material::chain`, the group build), `crates/nf-formats/src/level.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/fix-cel-order`.
