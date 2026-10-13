# Report: research-multiplayer

**Branch** `s/research-multiplayer` (from origin/master 30b98a5). **Done** — the multiplayer atlas
is written, every arena on the disc is pinned by a test, and the phased port plan is in the atlas.

## What's done

1. **`docs/atlas/multiplayer.md`** — the full atlas, every fact tagged:
   - **Arenas**: MP codes 0x07000021–0x29 + 0x0700004c group in `ResetMap_LevelCode2Img` 0x1d5258
     [M]; the on-foot ISO ships **8 arenas** (0x07000023…29, 0x0700004b) [D]. Placement types:
     0x25 = `MP_RegisterSpawnPoint` 0x184f20 (64-entry SpawnPoints, 0x30-byte records), 0x26 =
     `MP_RegisterMPObject` 0x185990 (dispatch on celglist+0x2c into the per-mode arrays: flags,
     bases, uplinks, demo/prot sites, GoldenEye sets, blueprint, hill), 0xf0 = pickups + 
     `MP_RegisterPickup` 0x189d80 into MPpickups (64 × 0xa0). No campaign drones anywhere [D].
   - **MPSettings @0x2a47a0**: the full field map (per-player records 8×0x30, scenario word +0x420,
     humans +0x428, bots +0x432, frag/time limits +0x412/+0x416, weapon set +0x436, spawn policy
     +0x448, radar +0x452…) [M] and the live match state MPGame @0x2a4980 [M].
   - **The 13 game modes**: the scenario-ID table (deathmatch 0x1, TDM 0x20000002, CTF 0x20000004,
     demo 0x20000040, protection 0x20000080, blueprint 0x20000100, GoldenEye 0x20000200, assassin
     0x400, KOTH 0x40000800/0x60001000, uplink 0x60000008, plus 0x10 (death-limit FFA) and 0x20) —
     each with its rules/scoring/win-condition from `MP_CheckForEndCondition` 0x184268,
     `MP_SortOutWhoWon` 0x1848d0 and the per-mode `MP_*Update` functions [M].
   - **The bots**: an MP bot is a campaign Drone object (0xD20 instance) with a 0x780-byte
     `BOT_vars_t` in the 4-slot array @0x26D660; per-bot config = 0x12-byte records @0x2DEEAA
     (aggression/aim/HP/speed/trait/team/character); the ~55-state `NDrone2_DSTATE_Bot*` family
     (ids 0xC3..0xF9) + `NDrone2_DSTATE_BotGlobal` 0x16e060; the goal layer (pickGoal 0x126b88,
     gotoGoal 0x1278a8, processGoals 0x127c78) over the shared nav graph; combat (states 0xEB/0xD4-DF,
     move chance from aggression, aim error bursts, trait-based target choice + 16-slot history);
     per-bot difficulty (no global MP difficulty exists); exactly what's shared with campaign drones
     and what's bot-only [M].
   - **Split screen**: 11-slot viewer array `glb_viewer` @0x2d88b0 (0x240 viewer_tag), rects per
     player count (2P side-by-side 256×448 or stacked 512×224 per DrawInfo @0x2a37c0; 3–4P
     quadrants 256×224), GS scissor from the viewer rect (`psiUseCamera` 0x123700), per-object u16
     view mask at obj+0xF8, the viewer-5 full-screen HUD-overlay mechanism with per-player pane
     sets (`MPPaneList` @0x2be9e0, `HUD_FixMP` 0x19b110), radar (`MP_GetRadarObjects` 0x189840 +
     `HUD_RadarUpdate` 0x19f6e8), and pad→player routing (`MapPlayerToSlot` 0x10cc48, multitap
     slots, `psiInput_MapInputs` 0x10cd80) [M].
   - **Reuse + phased plan** (§6 of the atlas): 6 phases (MP core harness → deathmatch end-to-end →
     split screen → the bots → the other modes → radar/pickup-sets), each with its touched-function
     count and dependencies.
2. **`crates/nf-game/tests/mp_arenas_live.rs`** — the NF_ISO test: loads all 8 arenas with the
   current parser and pins (placements, objects, drones, pickups, MP spawns, MP objects) per arena,
   asserts no campaign drones + spawns/pickups present, with a sister can-fail assertion. Probe run
   measured, then pinned; **green**.

## How it was verified

- The mode-ID constants were re-checked first-hand in the decompile (MP_Update/CheckForEndCondition/
  SortOutWhoWon/PlayerKilled/Flag/KOH/Uplink/BluePrint/GoldenEye comparisons vs `MPSettings._420_4_`)
  before going in the atlas [M].
- The test ran against the real ISO: `cargo test --offline --release -p nf-game --test
  mp_arenas_live` — 1 passed. All 8 arenas parse cleanly; counts are in the atlas table [D].
- Suite (full four crates, `--test-threads=1`, slim ISO): **620 passed in the main run + 376 more
  nf-view fast tests after a chunked re-run, 0 unexpected failures**; the only failures are the 3
  allowed `disc_golden` tests (slim-ISO BIGF quirk, pre-existing). The 52 walk/walkreal opt-in
  nf-view binaries (unchanged master code, green at the walkreal-c merge) were not re-run in this
  sandbox for time — no nf-view source changed in this task.

## Independent audit

The three research areas (arenas+modes, bots, split-screen) were researched by three parallel
subagents from the decompile; the key load-bearing claims (scenario word offset +0x420 and the mode
constants, the team bit 29 derivation in `MP_Init`, bot slots 4..7 + the 0x2DEEAA records in
`MP_Start`, viewer rect offsets, `BLData+0x94E` = viewer index) were re-derived by the main agent
from the decompile C before being tagged [M]. Open items that need the ELF or a live check are
listed in the atlas (the 0x2F3DE0 jump table, the 0x10/0x20 scenario names, the time-limit units).

## Live checks for the main agent

Listed at the end of `docs/atlas/multiplayer.md` §6 — five concrete watches: MPSettings+0x420/+0x428
in a real match, MPSettings+0x424 after selecting Arena 1 (expect 0x07000023), the first SpawnPoint
record at 0x2a7360 at load, `glb_viewer[0..3]` rects in a 2P match, and BOT_vars+0x45 goal-type
writes during play.

## What's left (for a follow-up task)

- Dump the `validateRoute` jump table @0x2F3DE0 and the state-name table @0x26F39D from the ELF to
  name the 12 route thunks and ~55 bot states.
- Resolve the 0x10 / 0x20 scenario names (menu string tables or the main agent's PCSX2 menu dump)
  and the +0x416 time-limit unit (minutes vs seconds).
- The atlas's per-mode detail (exact flag-carrier state machines, uplink RGB semantics) is one level
  shallower than the port will want; the scratch notes from this session had them, a follow-up
  should re-derive the deep per-mode flows into `docs/atlas/` files when the port phase starts.
- No game code changed in this task (per the brief); the plan's phase 1 is ready to start from the
  atlas.

## ntfy

Sent (see the finish steps' output; if all three channels failed the exact errors would be here —
they did not).
