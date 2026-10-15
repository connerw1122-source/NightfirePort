## Your task: `mp-core`, multiplayer phase 1: the match core on one arena

Read `docs/atlas/multiplayer.md` (research-multiplayer) first; this is its §6 phase 1. Port MP_Init / MP_Start / MP_Update,
MPSettings (0x2a47a0) and MPGame (0x2a4980) as Rust structs with the game's field offsets in comments, spawn-point
registration (placement 0x25 → `MP_RegisterSpawnPoint` 0x184f20, the 64-entry table) and selection (the spawn policy at
MPSettings+0x448), and the MPObject registration (0x26 → `MP_RegisterMPObject` 0x185990, stored, not yet driven).

Deliverable: `nf-view <iso> 07000023 --mp` loads Arena 1, spawns Bond at a spawn point chosen by the game's policy, and
runs the match clock (MPGame+0x400). Bots are not in this phase.
Tests: spawn-point tables for all 8 arenas against the placements (counts, positions); the spawn choice sequence from a seed
matches the decompile's arithmetic; the clock after N ticks.

Files: a new `crates/nf-game/src/mp/` module, `crates/nf-view/src/lib.rs` (the flag, minimal), docs/atlas/multiplayer.md (status).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/mp-core`.

---
claimed-by: sandbox4 at 2026-10-15T06:28Z
