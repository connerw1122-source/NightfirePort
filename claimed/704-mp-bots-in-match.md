## Your task: `mp-bots-in-match` — bots actually play a local MP match

A local MP Quick Game now loads its arena with 3 bots (the menu repair, 2026-10-15). Prove the bots play.
Headless, from the menus' `mp_match_settings` through `load_mp_carry`, tick 3600 frames of the arena and check each
bot:
- it moves its route (positions change);
- it acquires the human and other bots and fires;
- it takes damage and dies;
- it respawns per the game's rules;
- the scores go up.

Use the MP core (`nf_game::mp`, the bots module) and the game's bot AI [M BOT_update and friends]. Fix anything
inert: bots that never spawn, never move, or never fire. Table the per-bot stats after 3600 ticks. Sister: with bots
disabled, only the human is in the match.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mp-bots-in-match`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-16T16:31Z
