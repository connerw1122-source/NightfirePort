## Your task: `mp-bots-states`, the bot state machine (multiplayer phase 4, after 401)

401 (`mp-bots-data`, merged) ported the data layer: the per-bot config records, the `default_bot_stats` presets, the bot
state table 0xC3..0xF9 and the `BotGlobal` dispatch skeleton behind the `mp-bots` feature (`crates/nf-game/src/mp/bots.rs`,
`docs/fields/bot-vars.md`, `docs/atlas/multiplayer.md`).

Port the states the skeleton dispatches to, in the atlas's order of importance: the global state (0xC5 `BotGlobal`), the
move-to-goal / roam states, the weapon pickup visit (the MP_PICKUP+0x80 visit floats), and the attack state (target
choice, the fire decision from the stats block). For each state: decompile address, the `BOT_vars_t` fields it reads and
writes (`bot-vars.md`), its transitions, and its RNG calls in order. Keep everything behind `mp-bots`; the SP sim must not
change (the full suite stays green, byte-identical SP traces).

Test (NF_ISO, an MP level's archive — see `docs/atlas/multiplayer.md` for which levels carry MP spawn points): a match
with one bot. The bot leaves its spawn, reaches a pickup, and takes a shot at a stationary player within the
decompile's timing bounds. Sister: a bot with the "stand" preset doesn't move.

Files: `crates/nf-game/src/mp/bots.rs`, `docs/fields/bot-vars.md`, `docs/atlas/multiplayer.md`, the new test.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).


**Branch:** `s/mp-bots-states`.

---
claimed-by: sandbox2 at 2026-10-15T12:53Z
