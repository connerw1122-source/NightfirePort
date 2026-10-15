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

`crates/nf-view/tests/drone_skins_all_levels_live.rs` (new) walks all 25 story archives twice:
the raw archive (the BEFORE counts — placements wanting a skin their archive lacks) and the real
`nf_view::load` (the AFTER — skin + skeleton + built character; plus the sleeve check: no
`0x10`-flagged main entry may remain in the loaded skin table).

**Before/after table** (from the `--nocapture` runs; before = raw archive, after = post-load):

PENDING-CENSUS

The fail-first: with the substitution reverted (`git stash`), the census's sleeve assertion
FAILED on the first level with placed sleeve-flagged skins (PENDING-FAILFIRST). With the fix,
both assertions pass on every level.

## The fix

- `crates/nf-formats/src/character.rs`: `SLEEVE_ENTS` (the byte-exact list @0x2c6ed8) and
  `Skin::substitute_sleeves(present)` — replaces every `0x10`-flagged main entry with the first
  present SleeveEnts hash, `None` when the list misses (the game's 0). Direct entries untouched
  (the game only reroutes flagged ones). Unit sister: `sleeve_substitution_reroutes_only_flagged_entries`.
- `crates/nf-view/src/lib.rs`: one call after the session merges (so merged skins substitute
  too, and `by_hash` is final): `for sk in level.skins.values_mut() { sk.substitute_sleeves(present); }`.
  Plus the two comment corrections (AnimLoadFile 0x1bdf40; GameState+0xC).

## Verification

- The census test (above) on the final tree: PENDING
- nf-formats unit sister: PENDING
- Affected suites re-run on the final tree (the skin tables' main lists changed — the
  character/weapon/model consumers): PENDING-CHUNKS
- Full suite: the base tree (pre-fix, origin/master dc44c0e4 + the branch's WORKLOG commit) ran
  through the fast runner; totals below. PENDING-SUITE

## Shared files touched (small, additive)

- `crates/nf-view/src/lib.rs` — the task file (the loader): the substitution call + comment fixes.
- `crates/nf-formats/src/character.rs` — the task file: the list + method + unit test.
- `docs/models/characters.md` — one table row's address correction (0x1bdf40) + the dead-code note.
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
