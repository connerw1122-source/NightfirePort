## Your task: `fe-gaps-g048-2`, finish fe-gaps-g048 (HELD at merge)

`s/fe-gaps-g048` (task 757) was held at the 2026-10-17 batch: merged onto master it breaks 22 front-end
tests that pass without it (bisected to its merge alone) — among them `briefing_plays_live`,
`mp_menu_flow` (4), `memcard_save_flow` (4), `save_progress_roundtrip` (2), `pause_results_flows_live`,
`mission_unlock_flow`, `front_end_chain_live`, `movie_playback_flow`, `menu_music_flow`, `dossier_pages`,
`mp_host_match_live`, `mp_match_end_live`, `net_e2e_loopback_live` (2), `drive_hud`. The whole front-end
chain (menus → briefing → level → results → save) moved. Merge `origin/s/fe-gaps-g048` with the newest
origin/master, find the change that moves the chain, decide from the decompile which side is right [M], and
land the G-048 items without breaking those flows. Fix a test only with decompile evidence; the briefing test
pins the user's own bug (the briefing must play).

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

**Branch:** `s/fe-gaps-g048-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
