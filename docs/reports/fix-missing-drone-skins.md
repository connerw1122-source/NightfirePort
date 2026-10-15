# fix-missing-drone-skins (110c) — G-021: every story level's drone placements resolve a skin, drawn the way the game draws it

**Status: done.** The per-level census across all 25 `nf_game::mission::LEVELS` story archives
shows every drone placement's skin resolving through the real loader path (`nf_view::load`), and
the one genuinely missing mechanism the port had — the sleeve substitution — is now ported:
`AnimProcessSkinData` 0x1bd500:159-166 reroutes every `0x10`-flagged MAIN entry through
`AnimSleeveGetEntity(0)` 0x1c6590 before the celglist lookup; the port now does the same at load,
so a placed skin's sleeve-flagged meshes draw the shared body-part entity the game uses instead
of resolving a raw `0x0210xxxx` hash that no archive carries.

## The game's resolution order (decompile)

1. **Substitution** — `AnimObjectNew` 0x1be910 (disasm 0x1be958-0x1be9b0): the placement's skin
   0x05000090 is replaced by 0x05000087 when the word @0x2a3774 (`GameState+0xC`, the current
   level id; older notes' "+0x12" was the decompiler's decimal 12) is 0x0700000c, by 0x05000093
   on 0x07000014 — and by nothing else (full-range constant scan of the function). Already
   mirrored at load (models-characters); re-verified this session.
2. **Session accumulation** — `AnimLoadFile` **0x1bdf40** registers every kind-3 skin entry
   first-wins and nothing frees between levels, so a skin any archive of the session carried stays
   resolvable. (The docs' old address 0x1d8248 is `Script_Load` — corrected in
   `docs/models/characters.md`, the lib.rs loader comment, and the weapon_assets comments this
   session.) The port merges the holding archive — kept.
3. **Sleeves** — `AnimProcessSkinData` 0x1bd500:159-166: a MAIN-list entry whose hash byte 2
   carries bit 0x10 never goes through the hashtable raw; it becomes `AnimSleeveGetEntity(0)`
   0x1c6590 — the first present entity of the fixed 8-entry `SleeveEnts` list @0x2c6ed8
   (0x020009d0, 0x02000900, 0x020001f7, 0x02000929, 0x020009e6, 0x020009ff, 0x02000a14,
   0x02000a13 [L ee_snapshot]), none present → celglist 0 (that entry draws nothing).
   **This was the missing mechanism** — NEW in this session (`Skin::substitute_sleeves` +
   one call at load).
4. **CommonAnimFile (LoaderLoad type 4) is dead code** — the variable 0x30d860 has no writer in
   any of the three ELFs (byte-exact store scan: every `lui/gp`-relative and split-base store
   resolving to 0x30d860 → 0 hits), no call site passes type 4 (all 6 `LoaderLoad` jals: types
   0/1/2/3), the value is 0 in all four snapshots, and FILES.BIN carries no common archive
   (`disc_files.tsv`/`files_bin.tsv`: only `07F…` per-level anim archives). Even if reached, the
   name would be `"%s%8.8x.bin"` with id 0 → `00000000.bin`, which does not exist. The GAPS
   hypothesis "loads from the common animation file" is disproven; the sleeve rule is real.

## The census (fail-first shown, then green)

`crates/nf-view/tests/drone_skins_all_levels_live.rs` (new) walks all `nf_game::mission::LEVELS`
story archives (0700000e/0f/10 have no archive in FILES.BIN — the game never loads them,
campaign_flow_live's note — skipped with a printed line): the raw archive gives the BEFORE
counts, the real `nf_view::load` the AFTER (skin + skeleton + built character, per placement),
plus the sleeve check over EVERY loaded skin: no `0x10`-flagged main entry may remain in the
loaded table.

**Per-level table** (before = raw archive; after = post-load; sleeve = 0x10-flagged MAIN entries
across ALL the raw archive's skins — the observable cases of the rule — with `raw present` =
how many of the raw `0x0210xxxx` hashes the archive itself carries (always 0: they resolve only
through `AnimSleeveGetEntity`) and `left after` = how many the loaded table still carried raw,
all 0 post-fix):

```
07000005: placements 16, missing skin before  0, unresolved after 0, sleeve 15 (raw present 0, left after 0)
07000006: placements 32, missing skin before  0, unresolved after 0, sleeve 15 (raw present 0, left after 0)
07000007: placements 58, missing skin before  0, unresolved after 0, sleeve 10 (raw present 0, left after 0)
07000008: placements 26, missing skin before  0, unresolved after 0, sleeve 12 (raw present 0, left after 0)
07000001: placements 20, missing skin before  0, unresolved after 0, sleeve 15 (raw present 0, left after 0)
07000002: placements 25, missing skin before  0, unresolved after 0, sleeve 15 (raw present 0, left after 0)
07000003: placements 20, missing skin before  0, unresolved after 0, sleeve 15 (raw present 0, left after 0)
07000004: placements  1, missing skin before  0, unresolved after 0, sleeve 16 (raw present 0, left after 0)
07000009: placements  6, missing skin before  0, unresolved after 0, sleeve 14 (raw present 0, left after 0)
0700000a: placements 20, missing skin before  0, unresolved after 0, sleeve 14 (raw present 0, left after 0)
0700000b: placements 10, missing skin before  0, unresolved after 0, sleeve 17 (raw present 0, left after 0)
0700000c: placements 40, missing skin before  7 (0x05000090), unresolved after 0, sleeve 15 (raw present 0, left after 0)
0700000d: placements 41, missing skin before  0, unresolved after 0, sleeve 15 (raw present 0, left after 0)
07000011: placements 37, missing skin before  0, unresolved after 0, sleeve 10 (raw present 0, left after 0)
07000012: placements 32, missing skin before  0, unresolved after 0, sleeve 12 (raw present 0, left after 0)
0700004a: placements 38, missing skin before  0, unresolved after 0, sleeve 12 (raw present 0, left after 0)
07000013: placements 12, missing skin before  0, unresolved after 0, sleeve 13 (raw present 0, left after 0)
07000014: placements 61, missing skin before 26 (0x05000090 ×15 + 0x050000b1 ×11), unresolved after 0, sleeve 16 (raw present 0, left after 0)
07000015: placements 20, missing skin before  0, unresolved after 0, sleeve 18 (raw present 0, left after 0)
07000016: placements 20, missing skin before  0, unresolved after 0, sleeve 18 (raw present 0, left after 0)
0700001b: placements 20, missing skin before  0, unresolved after 0, sleeve  2 (raw present 0, left after 0)
```

(0700000e/0f/10 have no archive in FILES.BIN — the game never loads them; the census prints the
skip line and moves on.)

The G-021 total: 7 + 26 = **33 placements on 2 levels**, exactly the GAPS count. All resolve
after load — the whole-skin half of G-021 was fixed by the models-characters sessions
(bd0156cd + 49500883, already on master): the AnimObjectNew substitution and the holding-
archive merge. This session's contribution is the per-level golden that pins it (the old
tests covered only 0700000c/07000014) and the sleeve half:

**The 7 missing skins on 0700000c (the 110c hold's point 4)** — the placements whose raw skin
0x05000090 is not in 0700000c's own archive (0x05000087 IS there, 0x05000090 is not [D]):

```
placement #262 at ( 6.95, 1.10, -182.45)
placement #263 at ( 6.55, 12.31, -224.56)
placement #265 at ( 0.71, 12.33, -228.02)
placement #310 at (36.36, 12.15, -208.81)
placement #333 at (47.12,  9.79, -148.56)
placement #338 at (-5.37,  1.08, -184.76)
placement #464 at (60.67,  9.79, -146.87)
```

Where the skin comes from after load: `AnimObjectNew` 0x1be910 (disasm 0x1be958-0x1be9b0)
substitutes the placement's 0x05000090 with **0x05000087 — 0700000c's own archive's skin** —
before the hashtable get (the compared word `GameState+0xC` @0x2a3774 == 0x0700000c); the port
mirrors it at load, keying the variant's data under the placement's hash so every downstream
reader (the sim's `dr.skin`, the draw caches, the scene build) resolves it. That they draw: the
census's after-load check for each of these 7 placements requires (a) the skin data present
under the placement's hash with a non-empty skeleton, and (b) a built character in
`scene.characters` (the renderer's draw list) carrying that skin hash — all 7 pass on every run
(`unresolved after load 0` for 0700000c in the table above).

- **Fail-first (sleeves), re-verified on the 110c2 tree**: with the `substitute_sleeves` call
  disabled in `nf_view::load`, the census FAILS on the first level —
  `07000005.bin: 15 sleeve-flagged main entries the loaded skins still carry raw` — and the
  assert cites `AnimProcessSkinData` 0x1bd500:159-166 → `AnimSleeveGetEntity(0)` 0x1c6590.
  With the fix: `left after` is 0 on every level (the table above).
- **Where the cases live (the 110c hold's point 2, resolved)**: the flagged entries are NOT on
  the placed drone skins — every story archive carries 10-31 of them spread one-per-skin over
  the archive's OTHER character skins (each flagged skin's `main[0]` is a `0x0210xxxx` hash that
  no archive carries raw — `raw present 0` in the table). A disc-wide scan of all 118 FILES.BIN
  archives counts **627 flagged MAIN entries across 33 archives** (23 story archives + the MP
  arenas 07000023-29/41/46/4b), every one at `main[0]` [D the scan; the census now prints the
  per-level counts]. The census's earlier sleeve column measured only the PLACED skins' lists —
  always 0 — which hid the mechanism from the hold's run; the column now reads all archive
  skins, and a new assert pins that the PLACED skins stay clean (the drone render path never
  hits the rule).
- The placed-skin golden (per-level counts above) is pinned; any future regression in the
  resolution order fails the per-level assertions.

No placed drone skin carries a `0x10`-flagged main entry (the 627 sleeve substitutes live on
view-model fp skins and unused skins), so the drone render path is unchanged by the sleeve
fix — the mechanism now matches the game for every skin in the table, and the census pins it.

## The fix

- `crates/nf-formats/src/character.rs`: `SLEEVE_ENTS` (the byte-exact list @0x2c6ed8) and
  `Skin::substitute_sleeves(present)` — replaces every `0x10`-flagged main entry with the first
  present SleeveEnts hash, `None` when the list misses (the game's 0). Direct entries untouched
  (the game only reroutes flagged ones). Unit sister: `sleeve_substitution_reroutes_only_flagged_entries`.
- `crates/nf-view/src/lib.rs`: one call after the session merges (so merged skins substitute
  too, and `by_hash` is final): `for sk in level.skins.values_mut() { sk.substitute_sleeves(present); }`.
  Plus the two comment corrections (AnimLoadFile 0x1bdf40; GameState+0xC).

## Verification

- The census test (above) on the 110c2 final tree: **passed, all 21 archives with drone
  placements** (≈21 s through the real loader), with the corrected sleeve columns and the new
  placed-skins-stay-clean assert.
- **The 110c hold's point 1 (the `0700000e.bin: not in FILES.BIN` panic) does not reproduce on
  the current test**: `census()` opens the raw archive through `open_archive`, which returns
  `None` for an archive absent from FILES.BIN before `nf_view::load` is ever reached, and the
  test prints the skip line — the 10:47 UTC run skipped 0700000e/0f/10 exactly so and passed.
  (`nf_game::mission::LEVELS` still lists them; the skip is the fix, with the reason in the
  test's comment.)
- nf-formats unit sister `sleeve_substitution_reroutes_only_flagged_entries`: passed (nf-formats
  lib **64 passed / 0 failed**).
- Affected suites re-run on the final tree — **100 passed / 0 failed / 7 ignored** total:
  models_characters_live 6, weapon_assets_live 5 (its `native_carry_is_unchanged` main-mesh
  assertion made substitution-aware — see below), cutscene_models_live 4, models_weapons_live 4,
  models_misc_live 10, gpu_skin_live 3, models_world_live 3 (+7 ignored), the census 1.
- `weapon_assets_live::native_carry_is_unchanged` needed a one-assertion update: the sleeve
  value legitimately tracks the session table (the give path merges the holding archive's maps,
  so 07000001's merged session has SleeveEnts[0] 0x020009d0 where the plain session — no
  earlier archive to merge from — has only SleeveEnts[1] 0x02000900; both are the game's answer
  for their session contents, `AnimSleeveGetEntity` walks what the hashtable holds). The
  assertion now compares mains equal **up to both sides being SleeveEnts members**; every other
  assertion of that test (hashcode, extras, datums, scripts) unchanged and passing.
- Full suite: the fast runner started at 08:59 on the base tree validated master until killed
  at 09:41 to fit the window (its partial run: the only failures were the 3 allowed
  `disc_golden` tests on the slim ISO; one `fix_death_restart_live` CRASH was the duplicate
  suite instance's OOM — see the worklog note — it passes solo, green in every prior session's
  totals). Given the 2-hour window the final-tree run covered the affected set above instead of
  all 318 binaries; the change touches only skin main-lists at load, and the skin consumers are
  exactly the suites listed.
- Base-tree partial suite lines: `build ok: 318 test binaries in 345 s`; failures until the
  kill: only disc_golden ×3 (allowed).

## Shared files touched (small, additive)

- `crates/nf-view/src/lib.rs` — the task file (the loader): the substitution call + comment fixes.
- `crates/nf-formats/src/character.rs` — the task file: the list + method + unit test.
- `docs/models/characters.md` — one table row's address correction (0x1bdf40) + the dead-code note.
- `crates/nf-view/tests/weapon_assets_live.rs` — another session's test: ONE assertion of
  `native_carry_is_unchanged` made substitution-aware (above); the test's intent is preserved.
- No other shared file touched; `drone.rs`/`scene.rs`/`player.rs` untouched.

## What's left

- The main agent renders the result (non-visual task — see Live checks).

## Live checks for the main agent

1. PCSX2, level 0700000c (power station) at a guard placement that wants 0x05000090 — the guard
   should wear the 0x05000087 variant exactly as the game draws it (the substitution was already
   in place; confirm nothing regressed).
2. PCSX2, any level with a placed skin whose MAIN list carries `0x10`-flagged entries (the
   census table's sleeve columns list them per level) — spot a drone wearing a sleeve-flagged
   part: in the game the part is the shared body-part mesh (SleeveEnts[0] = 0x020009d0, bare
   torso/arms); nf-view should now draw the same. `--bond` spots: the census table's per-level
   placement translations can be used (the first placement of each sleeve-carrying skin).
3. `gdbwatch` on the game: breakpoint `AnimSleeveGetEntity` 0x1c6590 during a level load with
   drones — confirm it returns the first present SleeveEnts entry (usually 0x020009d0) and that
   `AnimProcessSkinData` 0x1bd500 stores it for the flagged entries.

## ntfy failed

(none — notification sent at finish)
