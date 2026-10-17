## Your task: `helicopter-2`, finish the helicopter (HELD at merge)

`s/helicopter` (task 750h) was held at the 2026-10-17 batch merge: merged onto master it breaks two tests that
pass without it (bisected to that merge alone):
- `nf-game` `guard_reaction_live::sight_to_first_shot_for_the_three_sighting_paths` (panics at
  guard_reaction_live.rs:216) — the guards' sight-to-first-shot timing, which is measured;
- `nf-view` `fix_weapon_sounds_live::every_weapon_on_07000005_draws_fires_and_reloads_with_sounds` —
  "0x3a: none of its missing sounds [1093, 1094] ever played; 0x41: none of its missing sounds [639] ever played".
07000005 carries two copters (map50 #241, #242), so the copter's own drone.rs / sound / RNG use is the first
suspect: a copter that draws from the shared RNG, takes sound voices or ticks in a different order changes the
guards' timing and the weapon sounds. Start from `origin/s/helicopter` merged with the newest origin/master,
make both tests pass alongside your own tests, and never loosen a measured test.

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

**Branch:** `s/helicopter-2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
