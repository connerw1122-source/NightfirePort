# fix-missing-drone-skins2 — finishing held 110c (`s/fix-missing-drone-skins`)

Branch: **s/fix-missing-drone-skins2** = origin/s/fix-missing-drone-skins (22ec9c87) merged with
origin/master (8401cab3; the WORKLOG conflict resolved both-kept, the branch's last; the lib.rs
loader block kept **master's braces** — `{ let mm = level.main_map(); } let mm = level.main_map();`
— per the merge note, with the branch's comment fixes inside).

## The four hold points, resolved

**1. The `0700000e.bin: not in FILES.BIN` panic — does not reproduce on the current test.**
`census()` opens the raw archive through `open_archive`, which returns `None` for an archive absent
from FILES.BIN *before* `nf_view::load` is reached; the test prints
`[0700000e.bin] no archive in FILES.BIN — not a story level, skipped` and moves on. The 10:47 UTC
run on this tree skipped exactly 0700000e/0f/10 and passed. (`nf_game::mission::LEVELS` still
lists them; the skip — with the reason in the test's comment — is the fix.)

**2. `sleeve entries 0` on every level — the census's BEFORE column measured the wrong set.**
The column summed the flagged entries over the *placed drone skins only*; they genuinely carry
none. The observable cases are the archive's OTHER character skins: a disc-wide scan of all 118
FILES.BIN archives counts **627 `0x10`-flagged MAIN entries across 33 archives** (the 23 story
archives + the MP arenas 07000023-29/41/46/4b), every one at `main[0]` with a `0x0210xxxx` hash
that no archive carries raw (`raw present 0` everywhere). 10-31 per story archive (07000005: 15,
0700000b: 17, 07000015/16: 18 each …). The flag test itself is correct [M
`AnimProcessSkinData` 0x1bd500 (ACTION.ELF/001bd500): the MAIN loop reads the entry's byte at
offset +2 (`pbVar2 = pAnimData + 2`) and tests bit `0x10` — i.e. `(hash >> 16) & 0x10`, exactly
the port's `Skin::is_sleeve`]. The census now counts ALL archive skins in the sleeve columns, and
a new assert pins that the PLACED skins stay clean (the drone render path never hits the rule).
**Fail-first re-verified on this tree**: with the `substitute_sleeves` call disabled in
`nf_view::load`, the census fails on the first level — `07000005.bin: 15 sleeve-flagged main
entries the loaded skins still carry raw` — and is green with the fix (`left after 0` on every
level). The substitution mechanism is real, observable, and now pinned by the golden.

**3. The report's placeholders — filled.** The 110c report (`docs/reports/fix-missing-drone-skins.md`)
now carries the real per-level table (placements / missing-skin-before / unresolved-after / sleeve
counts), the fail-first line from this tree's run, and the totals below. No PENDING markers remain.

**4. 0700000c's 7 missing skins — named, sourced, draw-confirmed.** The placements wanting
0x05000090 (absent from 0700000c's own archive, which has 0x05000087 [D]):
#262 (6.95, 1.10, −182.45), #263 (6.55, 12.31, −224.56), #265 (0.71, 12.33, −228.02),
#310 (36.36, 12.15, −208.81), #333 (47.12, 9.79, −148.56), #338 (−5.37, 1.08, −184.76),
#464 (60.67, 9.79, −146.87). Source after load: `AnimObjectNew` 0x1be910 (disasm
0x1be958-0x1be9b0) substitutes 0x05000090 → **0x05000087, 0700000c's own archive's skin**, when
`GameState+0xC` @0x2a3774 == 0x0700000c; the port mirrors it at load keyed by the placement's
hash. That they draw: the census's after-load check per placement requires the skin data + a
non-empty skeleton + a built character in `scene.characters` (the renderer's draw list) carrying
that hash — all 7 pass (`unresolved after load 0`). (07000014's 26 missing resolve the same way:
0x05000090 ×15 via the 0x07000014 → 0x05000093 substitution arm, 0x050000b1 ×11 via the session
merge from 07000015/16's archives [M 0x1bdf40's first-wins accumulation].)

## What changed this session (the branch's files only)

- `crates/nf-view/tests/drone_skins_all_levels_live.rs` — the sleeve columns measure ALL archive
  skins (with the where-the-cases-live comment) + the placed-skins-stay-clean assert.
- `crates/nf-view/src/lib.rs` — **no behavior change**: master's loader-block braces restored
  (the merge note); the sleeve call and its comment are the branch's, unchanged.
- `docs/reports/fix-missing-drone-skins.md` — the real numbers (above), placeholders gone.
- The scratch probe (the disc-wide sleeve scan) was run and deleted, not committed.

## How it was verified

- The census on the final tree: **passed, all 21 archives with drone placements** (≈21 s) with the
  corrected columns; the nf-formats unit sister `sleeve_substitution_reroutes_only_flagged_entries`
  green (nf-formats lib 64/64).
- Fail-first: the census with the substitution call disabled FAILS on 07000005 (15 raw entries),
  green re-enabled — shown in this session, on this tree.
- The affected suites re-run: weapon_assets_live 5, models_characters_live 6, cutscene_models_live 4,
  models_weapons_live 4, models_misc_live 10, gpu_skin_live 3, models_world_live 3 (+7 ignored) —
  all green.

## Full-suite total (final tree, foreground chunks — background jobs die between tool calls here)

**1275 passed, 6 failed, 66 ignored** across 327 test binaries. The 6: the 3 `disc_golden`
(allowed on the slim ISO) + `fix_0c_pylon_live` ×2 + `probe_pylon_render` ×1 — the same set
prior sessions verified pre-existing on master (the 1227/6/66 baseline). Five binaries wall-cut
at 90 s in the chunks re-ran green solo (campaign_flow_live 5, fix_guard_weapons_live 3,
levels_sweep_live 3, walkreal_07000001_live 2, weapon_matrix_live 6 — 328 s, its own comment says
the suite gives it 1200 s). The `nf_view` bin target (0 unit tests; its main opens a window)
is not a suite member and was excluded, as the runner does.

## What's left

- The main agent renders the result (non-visual task — the live checks are in the 110c report:
  the 0700000c guard-skin variant, the SleeveEnts[0] substitution on any sleeve-carrying skin,
  the `AnimSleeveGetEntity` breakpoint at 0x1c6590).

## ntfy failed

(none — notification sent at finish)
