## Your task: `mp-bots-data`, multiplayer bots, data first: BOT_vars, the bot records and the Bot states

Read `docs/atlas/multiplayer.md` §4 first. Groundwork for phase 4, independent of the match core:
1. Lift the per-bot config records (0x12 bytes each at 0x2DEEAA: aggression/aim/HP/speed/trait/team/character) as a data
   table with a golden test.
2. Map `BOT_vars_t` (0x780 bytes, the 4-slot array at 0x26D660) field by field in `docs/fields/bot-vars.md`, the way
   `docs/fields/` did for the player and actors.
3. Inventory the ~55 `NDrone2_DSTATE_Bot*` states (0xC3..0xF9): for each, the address, what it does, and which existing
   drone states/helpers it reuses (the port's drone.rs), so phase 4 is a list of arms to add. Port `BotGlobal` 0x16e060's
   dispatch skeleton behind a feature that nothing calls yet, with a unit test of the dispatch.

Files: a new `crates/nf-game/src/mp/bots.rs`, `docs/fields/bot-vars.md`, docs/atlas/multiplayer.md (status).

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

**Branch:** `s/mp-bots-data`.

---
claimed-by: sandbox7 at 2026-10-15T06:47Z

---
## Report (done, 2026-10-15T08:51Z)

Closed by the driver: the report is on `s/mp-bots-data` (docs/reports/mp-bots-data.md); the worker didn't run finish.sh.
