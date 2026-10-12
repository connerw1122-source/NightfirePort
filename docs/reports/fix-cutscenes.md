# fix-cutscenes — the party Drake cut-scene, the gondola intro, and the full cut-scene survey

Branch: `s/fix-cutscenes` (from master e020db6).

## What's done

**Both broken cut-scenes are fixed.** Their scripts were never in the level archives: each level's type-0xC
entries register (script hash, flag) pairs, and the game streams a **loadable pack** file named with the
level id's (flag+7) nibble — flag 1 → `078000NN.bin`, 2 → `079000NN.bin`, 3 → `07A000NN.bin`
(`SP_LoadScript` 0x1dbc08 → `isLoadable` 0x1d69a0 → `LoadableLoad` 0x1d69e8 → `LoaderLoad` 0x1d4a00;
`LoaderProcess` 0x1d4880 case 0xC). Measured on the disc: 07000007's records (0x06000072, 2) and
(0x06000073, 1) map to `07900007.bin` / `07800007.bin`, which contain exactly those scripts plus their
animation sequences; the same holds on 01/02/08/09/0b/0d/11/13/14/16/1b/4a. The port only ever loaded the
level archive, so the ScriptPlayers deleted themselves on their first trigger and the scenes never played.

- **07000007 (the party):** Bond walks in → TouchOnce #356 raises ch 0x15 → SP #419 plays 0x06000072 — the
  3000-tick fly-through with the camera sweep and every guest animated (the "Bond first sees Drake"
  scene). TouchOnce #412 raises ch 0x48 → SP #420 plays 0x06000073 — the control-taking cut-scene
  (3 × player-disable/hand-back cycles).
- **07000008 (the gondola level):** Bond spawns ~3.5 m from TouchOnce #237 → ch 0x49 → SP #294
  ("Trigger_1stNIS") plays 0x06000077 from `07800008.bin` (3880 ticks) — the cut-scene that starts almost
  immediately.

The fix reads the type-0xC records in `nf_view::load_with_carry` and merges each pack's scripts and
sequences at level load. Two subtleties measured on the way: the game deletes its animation hashtable
before streaming a pack, so pack sequences must never join the level's per-skeleton idle pool (they go
under a sentinel skeleton key that only the by-hash map reads) — appending them there changed guard
animations; and the level-start intros on 07000001/07000002 now play like the game's (verified against the
game's own snapshot: SP state 2, clock 42, length 2120 at level start), so the affected tests skip the
intro with the game's own skip mechanism.

## What's verified and how

- `crates/nf-view/tests/fix_cutscenes_live.rs`: 3 tests drive the real path (`nf_view::load`, Bond's
  capsule into the real trigger triangles, channel, ScriptPlayer) and failed before the fix ("SP deleted
  itself on its trigger"); the fourth (the 07000007 urn ScriptPlayer, mode-64 preload) pins the priming.
  The Drake test also checks the cut-scene's camera object and that its animated guests advance frames.
- The party/gondola chains were read end-to-end on the disc (placements, trigger volumes, channels, SP
  params, script contents) and are documented with addresses in `docs/systems/levels.md` §5.
- Full suite on the slim ISO: nf-formats, nf-audio, nf-game and nf-view all green except the 3 allowed
  `disc_golden` failures (nf-view alone: 106 passed, 0 failed).

## The survey (docs/systems/levels.md §5)

Every level-start and intro cut-scene in the story levels, and which ones start in the port:

- **Loadable-pack scenes** (§5.1): 14 SPs on 12 levels — all play now (before the fix they all deleted
  themselves at their first trigger; the mode-1 ones — 07000001 #277, 07000002 #400, 0700000d #377,
  07000013 #65, 0700001b #147 — deleted at load).
- **Archive-resident level-start scenes** (§5.2): already played before this task (e.g. 07000005's truck).
- **Mode-64 preloads** (§5.3): primed at start, play on a trigger (the 07000008 cable-car ride, the
  0700000d crane, …). All archive-resident, already worked.
- **The intros** (§5.4): `ResetMap_Load` (0x1d6360) creates one ScriptPlayer per level from a movie-code
  table (7 on-foot levels: 01, 05, 09, 0c, 11, 14, 1b) — only when the SUBTITLES option is on. All 14
  scripts in `07000500.bin` are single op-0x1E text streams: subtitle timings for the intro MOVIE, which
  the port doesn't play (no PSS player) — that gap is the movie player's, not the ScriptPlayer's.
- One dead reference: 07000005 #71 waits for 0x06000074, but no pack for it exists on the disc — the real
  game cannot stream it either.

## What's left

- `SP_Hit` proxy trigger bits (bullet 0x2, neighbours 8/0x10): 07000007's urn-break cut-scene (SP #312,
  mask 2, correctly primed) still can't fire — that's the last cut-scene kind the port can't start.
- The `GameFlow_PushState(12.0, 0xD, 0xFF)` streaming loading screen (cosmetic; the port loads eagerly).
- The 07000002 C2-class drone dump, the 07000005 torso-twist, the 07000006 stationary guards and the
  searchlight alarm path — drone.rs/searchlight.rs are outside this task's file list; not attempted.
- The pack sequence merge is eager (at level load) where the game streams lazily at first use — same
  content, no gameplay or RNG-order impact; noted in `docs/systems/script-player.md`.

## Files

`crates/nf-view/src/lib.rs` (the level-start path — the fix); `crates/nf-view/tests/fix_cutscenes_live.rs`
(new); minimal test-only adaptations in `crates/nf-view/tests/{fix_player2_live,fix_player3_live,
fix_ladder_live,render2_live}.rs` (skip the now-playing intros with the game's own skip; the raptor-drop
take loop approaches from the open sides); docs (`levels.md` §5, `script-player.md` port status, GAPS
G-031, port-map core-loop `SP_LoadScript` row, WORKLOG). `nf-game` and `nf-formats` are untouched.
