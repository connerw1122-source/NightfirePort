## Your task: `level-09-interactables-2`, finish the 09 interactables (HELD at merge)

`s/level-09-interactables` (709a, 45c5dc5) was held: its 19-line `crates/nf-game/src/drone.rs` change breaks four
tests that pass on origin/master (bisected — the three branches merged before it leave them green):
`ai_live::alarm_pulse_reaches_only_the_drones_updated_first` (ai_live.rs:376 "drone 66 never saw the pulse", left 112),
`fix_cutscenes2_live::the_drake_flythrough_hides_the_live_party_and_restores_it`,
`spawner_kill_live::the_wall_hug_scene_holds_the_gate_pair_and_swaps_the_pairs_at_its_end`,
`kimiko_actors_live::the_drake_flythrough_draw_set_matches_the_script_with_no_duplicates`.
The change reaches every level's drone update, not just 09. Merge the newest origin/master, scope your change to what
the game does (cite it), keep your census/interact tests, and make all four pass.

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

**Branch:** `s/level-09-interactables-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-17T18:06Z
