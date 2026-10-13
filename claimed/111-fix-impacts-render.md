## Your task: `impacts-render`, draw the impact effects with the game's own meshes

Read `docs/reports/fix-impacts.md` first (merged at f38cfac). The sim side of the impact path is ported (decals,
debris, casings, puffs), and `Sim::effect_quads` (crates/nf-view/src/scene.rs) is drawn in `snow_for`
(crates/nf-view/src/lib.rs) as **flat-colour quads**, a placeholder. Replace it with the game's own draw:

1. **Decals:** the material's decal mesh from `EffectInfo+4` (hashes 0x02000122 glass, 0x02000126/127 stone/metal,
   0x02000128 wood, 0x02000262 soft-generic, ICE 0x020006fd), oriented in the surface plane, depth-tested,
   alpha-blended, no depth write. Find the game's decal draw (the `Effect_*` list walk and its draw call) and its
   orientation/scale, and match its blend/z state from the decompile.
2. **Casings:** the `ammo_data+4` mesh instances at the casing pose (pos + the spin the sim integrates).
3. **Debris:** the `EffectInfo+0x74` meshes at the particle pose/scale/alpha.
4. Write `docs/atlas/effect.md` (owed by fix-impacts: the impact path, its addresses, the port's [S] stand-ins), and
   update GAPS G-041 / G-025 to the state after your commit.

Prove it with an NF_ISO test through the real load path (the instance list `effect_quads` is replaced by carries
the right mesh hash per material and the casing hash for the P2K), plus a sister, and, if the sandbox has a GPU,
`nf-view --shot` at `07000005` after firing at a wall. List render spots for the main agent in the report.

Files: `crates/nf-view/src/{scene,lib}.rs`, the nf-view renderer files the draw needs, `docs/atlas/effect.md`,
`docs/GAPS.md` (G-041/G-025 rows only).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/impacts-render`.

---
claimed-by: sandbox6 at 2026-10-13T12:30Z
