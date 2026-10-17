**Base:** merge `origin/master` first and check it contains `crates/nf-view/tests/briefing_plays_live.rs` (the
front-end audit + test fixes landed with it, 2026-10-16 evening). If it does not, fetch again before you start: the old
master still carried 28 tests that contradicted the decompile.

## Your task: `drone-skins-g021`, the 33 drone placements without a skin

docs/GAPS.md G-021: 33 drone placements have no skin in their own archive. The game loads the common animation file
(`CommonAnimFile`, LoaderLoad type 4) and other archives; some placements take a sleeve substitution. Find, per level,
which placements still render skinless or fall back (`nf-extract` / a test listing them), trace the game's lookup for
each (`LoaderLoad` 0x1d4a00 type 4, the skin hashtable get with its per-level substitutions [M]), and port the missing
sources. Tests: the per-level count of skinless placements drops to the game's (0, or the cited exceptions), with
the skin hash each placement resolves to; ISO-gated, numbers/ids only.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drone-skins-g021`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-17T07:08Z

---
## Report (done, 2026-10-17T08:43Z)


---
## Report from the worker (done, 2026-10-17T08:43Z)

# drone-skins-g021 — G-021: the 33 drone placements without a skin in their own archive

**Status: done.** G-021 is resolved by tracing the game's whole skin path to the decompile and
scanning the disc; the port now resolves skins exactly the way the game's session does, the two
old tests that pinned the pre-gate port behaviour are re-pinned to the game with addresses, and a
per-level census pins every placement's resolved hash. The headline correction to the GAPS
hypothesis: **the 33 do not all resolve — 22 resolve through the substitution, 11 (0x050000b1 on
07000014) resolve for NO fresh story session and the game itself renders them skinless.**

## The trace (full chain in docs/systems/drone-skins.md)

- `LoaderLoad` 0x1d4a00 opens a hash-named .bin container and walks its sub-file directory;
  `LoaderProcess` 0x1d4880 case 3 hands each type-3 sub-file to `AnimLoadFile` 0x1bdf40, which
  registers it **first-wins** (`hashtable_additem` only when `hashtable_getitem` 0x1ca5b0
  misses) and parses it into the 0x40-byte skin record via `AnimProcessSkinData` 0x1bd500 [M].
- **Nothing frees skins**: the only hashtable deletions are `LoadableLoad`/`LoadableReload`
  0x1d69e8/0x1d6ab0's `hashtable_delete_hashtype(0x4000000)` — 0x1ca520 compares each entry's
  hash **top byte** to the argument's, so only 0x04 sequences die — and
  `Script_FreeFromHashTable` 0x1daec0's single scripts [M]. The session table is therefore the
  union of every container loaded since boot: the front-end (0x07000048) plus each story level's
  own container in campaign order.
- `CommonAnimFile` (LoaderLoad type 4) stays dead code (no writer, no type-4 call site, no such
  archive — fix-missing-drone-skins' store scan stands).
- `NDrone2_DefaultInit` 0x14b300 puts the placement param 0 in `+0xe0`; the **captain swap**
  :536-555 rewrites it for kinds 1/2/3/6/7 (property 9/5/8 at the difficulty `GameState+0x28` —
  the decompiler's `_40_4_` names are decimal, hex +0x28, 0x2a3790); `AnimObjectNew` 0x1be910
  :33-43 substitutes 0x05000090 → 0x05000087 (0x0700000c) / 0x05000093 (0x07000014); :44-46 does
  the get; **a miss returns 0, the caller discards it (:563-565) and no character meshes are
  created** — the placement renders skinless [M].

## The disc [D]

Scan of all 99 FILES.BIN containers' type-3 sub-files (probe kept out of the repo; reproducible
with `nf-extract bin-list`): 0x05000090 is carried by 07000049/0d/15/16; 0x050000b1 by
**07000015/16 only**. Story order …, 13, **14**, 15, 16 … — at 07000014's load no carrier has
loaded, so:

| placements | game's resolution |
|---|---|
| 0700000c ×7 wanting 0x05000090 | substitution → 0x05000087 (own archive) — resolve |
| 07000014 ×15 wanting 0x05000090 | substitution → 0x05000093 (own archive) — resolve |
| 07000014 ×11 wanting 0x050000b1 | **no session carrier — the game renders them skinless**, except kind-6/7 captains (swap → 0x05000093/0x050000b2, both in the own archive) |

The census (`drone_skins_all_levels_live.rs`) runs the real loader on all 21 story archives: at
the direct-load difficulty the b1 set creates 5 drones, all 5 skinless, 0 captains (6 of the 11
placements sit above the param-2 gate and create no drone); every other level resolves every
drone. The sleeve rule (AnimProcessSkinData :159-166) stays 0-flagged-raw everywhere — no placed
drone skin carries a sleeve entry, as fix-missing-drone-skins found.

## The port changes

- `crates/nf-view/src/lib.rs` — the G-021 merge is **session-gated**: a foreign archive is a
  source only if the session could have loaded it (the front-end 07000048 or a story level
  before the current one, `nf_game::mission::LEVELS` order). 07000014 merges nothing for
  0x050000b1 any more; the index keeps all carriers per hash so a later-session carrier can
  never outrank an earlier one.
- `crates/nf-game/src/drone.rs` — `captain_skin_of_kind` (one source for the 0x14b300:536-555
  kind→skin table, now used by `Drone::captain_setup`) and pure `captain_skin` (the same rule
  from placement params + property tables + difficulty — no drone creation, so no RNG shift).
- `crates/nf-view/src/scene.rs` — `Scene::build`'s character pass resolves a drone placement
  through the **post-captain-swap** hash: a swapped captain's instances are rebuilt from the
  swapped skin's mains and the raw skin's dropped; a swapped skin the table lacks draws nothing
  (the game's miss). Visible deltas: 07000002's two kind-1 captains now build 0x05000038's
  meshes (0x05000038 is in 07000002's own archive [D]); 07000001's captains likewise; 07000014's
  b1 meshes are gone.
- `drone_skins_all_levels_live.rs` — rewritten census: per-level raw-missing sets, per-drone
  resolved hashes (printed with `--nocapture`), the skinless set, the substitution content
  equality (skeleton + mains vs the variant entry), the sleeve rule.
- `models_characters_live.rs` — **the two old G-021 tests re-pinned to the game** (the TEST was
  wrong): `g021_the_airbase_s_foreign_guards_resolve` now asserts 0x050000b1 does NOT resolve on
  07000014 and no character draws for it (the session gate + `AnimObjectNew` 0x1be910:44-46 +
  `NDrone2_DefaultInit` 0x14b300:563-565 — addresses in the test header), and the golden's
  post-load pass asserts the same. The old expectation ("the foreign archive merge resolves
  0x050000b1") pinned the pre-gate port, not the game.
- `load_scene_equality.rs` — scene digests re-pinned (the test's own [S port-engineering]
  pattern; 05 and 0c byte-identical, 02/14/01 moved exactly by the two mechanisms above); the
  assert now collects all levels' drift in one failure and prints each `got` line for the next
  re-pin.

## How it was verified

- Census `drone_skins_all_levels_live` — green on the final merged tree (1 passed, 16.3 s).
- models_characters_live 6, cutscene_models_live 4, models_misc_live 10, models_weapons_live 4,
  models_world_live 3, gpu_skin_live 3, weapon_assets_live 5, nf-formats lib 79, the
  07000002/14/15/16 walkers 10 — all green.
- `load_scene_equality` — green after the re-pin (1 passed).
- The fast runner (`tools/test/suite.py`) on the final merged tree: **278 of 496 binaries in
  three foreground chunks** (this sandbox kills background jobs between tool calls, so the run
  could not finish inside the window): zero failures except
  `load_scene_equality::the_cut_costs_build_the_same_scene` (re-pinned above, now green) and
  `probe_pylon_render::probe_render_pylon_only` — a wgpu `adapter: NotFound` (the sandbox has no
  GPU; environmental, unrelated to the change). The models/menu/census binaries the change could
  affect all ran green in the chunks or in the targeted lists above.
- Pushed in five checkpoints (WORKLOG start → gate+captains → test re-pins → docs → digest
  re-pin), then merged `origin/master` d628f01f and re-ran the census green.

## What's left

- The main agent renders the result (non-visual task; the visible deltas are 07000002's/01's
  captains in their captain skins and 07000014's b1 guards no longer drawn).

## Live checks for the main agent

1. PCSX2, 07000014 (airbase) fresh boot, story order: at the b1 placements (the census's
   skinless rows give the translations) the guards should show **no body** — AI still runs and
   shoots. `--bond` spots: use the census table's first b1 placement translation.
2. `gdbwatch.py` on `hashtable_getitem` 0x1ca5b0 during the 07000014 load: confirm the 0x050000b1
   lookups return 0 (and the substituted 0x05000093/0x05000087 lookups return non-zero).
3. 07000002 at difficulty ≥ 2 (or 1 — the census load swapped them at 1): the two kind-1 captains
   should wear the 0x05000038 skin, not their placement skins 0x05000039/0x0500003a.
4. A captain's held weapon: the sim derives it from the RAW skin (`Drone::new`'s `weapon_id`
   before `captain_setup`), the draw now derives it from the swapped skin — check in PCSX2 which
   the game shows for 07000002's captains (untraced here; the port's two sides disagree by
   construction, one of them should be aligned once observed).

## Usage

not visible to me — this session exposes no token, cost or context-window figures.
