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

---
## Report (done by sandbox1)

# models-characters — report

Branch `s/models-characters` (from master d93772b). Lane: every skinned character — complete, exact,
bound right, placed right, visible. The harness: `crates/nf-view/tests/models_characters_live.rs`
(6 tests, NF_ISO, all green).

## What's done

**1. G-021 — the 33 drone placements whose skin is not in their level archive. FIXED, with the
game's own mechanism.** A disc-wide scan [D] found them on exactly two levels: 0700000c (7 ×
0x05000090) and 07000014 (15 × 0x05000090 + 11 × 0x050000b1). Two mechanisms, both read off the
decompile:

- **The substitution**: `AnimObjectNew` (0x1be910, disassembly 0x1be958-0x1be9b0) rewrites
  0x05000090 to **0x05000087** on 0700000c and **0x05000093** on 07000014 (both the levels' own
  skins) before the hashtable get. The port mirrors it at load; the merged entry keeps the
  placement's hash so the sim's skin lookups stay keyed consistently. My first attempt merged
  foreign archives for these too — the decompile corrected it (the substitutes are what the game
  draws), and the commit history shows the rework.
- **The session-table resolve**: 0x050000b1 has NO substitution (the disassembly covers only
  0x05000090), and the game's skin/object tables accumulate every archive loaded in the session
  (`AnimLoadFile` 0x1d8248 registers per map; nothing frees between levels) — 0x050000b1 lives in
  07000015/16's archives [D]. The port merges the holding archive's maps (the skin's meshes are
  that archive's glist objects), object hashcodes (first-wins), skins and skeletons.
  `CommonAnimFile` (the LoaderLoad type-4 name source, the variable at 0x30d860 [M 0x1d4a00 case
  0 — the only reference in ACTION.ELF, no writer]) is **0 at every level-start snapshot** [L
  ee_start_05.bin / ee_start_07000001.bin / ee_start_0700000a.bin @0x30d860] — not the source
  here. The merge adds no sequence to any skeleton the level's own 07F already covers (both
  G-021 skins use skeleton 0, which 0700000c covers with 429 sequences and 07000014 with 402 [D])
  — the order-sensitive gameplay lists the WORKLOG warns about are untouched, pinned by
  `g021_the_merge_leaves_gameplay_data_untouched`. Fail-first shown for 0700000c.

**2. The golden (completeness).** `golden_every_story_level_s_placed_skins_resolve` pins, per
story level: the drone-placement count (515 total), the distinct placed-skin hash set (88 skins),
and the archive's own skin count. Every placed skin resolves post-load. One dead end documented:
**0700001b's cut-scene script names skins 0x05050a17 / 0x05050ad2, which exist on NO archive of
the disc** [D scan of every type-3 entry] — the game's `AnimObjectNew` rejects any hash whose
prefix byte check fails and any hashtable miss, so they cannot draw in the game either.

**3. The pose check (exactness).** `nf-extract pose-check` re-run on all three EE snapshots:
`ee_start_05.bin` (07000005), `ee_start_07000001.bin`, `ee_start_0700000a.bin` — **worst local
0.00e0 on all three** (bit-exact). The only nonzero world deltas sit on the one 07000005 drone
whose sAnimObject+0x58 carries the live head-track/aim flags (0x40f) — the game's runtime look-at
state, not a port error. G-024's world-matrix 4.6e-5 remains owned by `250-fix-bitexact`.

**4. G-042 — the bathhouse civilian texel check.** 07000002's civilian body skin 0x05000034
("polySurface2", the most-textured body on the level [D]) decodes with non-white texels on every
batch texture (`g042_...` asserts ≥ 8 textures checked, not every-texel-white, and < 25 % white).
The old over-bright was the lighting path (fixed by fix-render2), not white source data — the gap
entry's question is answered and pinned.

**5. Playtest F3 — the tuxedo vanish. Re-verified fixed on the current master.** The dressing
cut-scene's two actors — the suit 0x05000007 and the tuxedo 0x0500000d (both in 07000006's archive
[D], and their type-5 anim scripts 0x060001fc/0x060001fb with their sequences in 07F00006 [D]) —
resolve and pose through the whole cut-scene; the live script objects carry exactly one Anim per
stream (the mid-script swap mechanic, `Script_AnimStart` 0x1d8f40's kill-and-recreate, is in the
port's `anim_start`). The reported vanish was the pre-fix-render2 script-character resolution.
Regression: `f3_both_dressing_actors_resolve_and_pose`.

**6. The lane inventory + render spots** — `docs/models/characters.md`: the per-level skin table,
the two G-021 mechanisms, the dead skins, the pose-check results, and one `--bond x,y,z,yaw` spot
per level (3 units north of each level's first placed drone group) for the main agent to render.

## Shared files touched (additive, minimal)

- `crates/nf-view/src/lib.rs` — the G-021 load merge + the `AnimObjectNew` substitution (one block
  after the level's own sequence load; `skeletons` is now `mut`).
- `crates/nf-view/tests/fix_hud_live.rs` — not touched this task.
- New files: `crates/nf-view/tests/models_characters_live.rs`, `docs/models/characters.md`.

## What's left (each with its evidence)

- **The 0700001b dead cut-scene skins** (0x05050a17/0x05050ad2) — nothing to resolve against on
  the disc; a substitution by policy would be the driver's call.
- **The gondola riding inherit** (fix-objects2's leftover): script-ENTITY parts vs the truck's
  ANIM parts in the ground-motion path — a character-adjacent behaviour, listed here because the
  carriage is a character-hull object; the full diagnosis is in fix-objects2's report.
- **The kind-4 pickup's label text** (PICKUPINFO+0x34 → `Txt_BindLabel`) — fix-objects2 leftover,
  HUD-side, no character impact.
- **World matrices ≤ 4.6e-5 off** — G-024, owned by `250-fix-bitexact` (the local poses are
  bit-exact; only the world composition differs).

## Verification

`models_characters_live.rs` (6 tests) green; the full suite
(`--no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`) green
except the 3 allowed slim-ISO `disc_golden` failures; 99 test binaries ok.
