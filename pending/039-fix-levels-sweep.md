Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-levels-sweep`, a health check of all 36 on-foot levels

For each on-foot level in `docs/systems/levels.md`, run headless:
- load, 1800 ticks of a scripted walk along the AI path graph, then render one `--frame --shot` on a GPU-less pass (counts only);
- record: panics, unresolved textures, placements of unhandled types, drones stuck in unported states (state ids with no handler), scripts that never start, and pickups that don't resolve a model.

Write the table into `docs/systems/levels.md` (level, then each issue count), and fix every crash you find. For each other issue class, list the unported game function behind it, so later tasks can target it.

Add an NF_ISO smoke test: every level loads and runs 600 ticks without a panic.

Files: `crates/nf-view/tests/levels_sweep_live.rs`; fixes wherever the crashes are, minimal.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Also: objective channels with no writer (from the merges of fix-mission and map-castle/map-mayhew).** For every level, list each objective's condition and enable channels (`nf_game::mission::LEVELS`) and find their writers on the disc (placements, multiplexers, script events, drone death channels, pickups, doors, locks). Known gaps: **07000005's first objective "Breach the castle walls" completes on ch 0x46** (record `46 00 01 45` at +0x0C [L ee_start_05.bin 0x2a3e34]) and its enable ch 0x45; no writer has been found for either. 07000002/03's exits are key-gated on ch 0x4C / 0x52, and 07000008's boarding and ride channels 0x4A/0x4B come from the cable-car pack script (not parsed). Port the writers you can prove, and put the rest in the table with the game function that would write them.

**Branch:** `s/fix-levels-sweep`.
