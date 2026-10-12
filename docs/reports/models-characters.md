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
