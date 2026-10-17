## Your task: `frag-throw-timing2`, finish frag-throw-timing (HELD at merge)

`s/frag-throw-timing` (task 741) was held at the 2026-10-16 batch merge: on top of the batch it fails
`nf-view` `fix_grenade_types_live::the_stun_grenade_whites_bond_out_at_the_games_time_and_alpha` (alpha 0.673 vs the
formula's 0.758 / 0.379 on the observed geometry — tolerance 0.08) [M 0x12dfb0]. The test reconstructs the blast
point from the last grenade position it saw and Bond's look; with the new release timing the grenade is still
moving at the pop, so the reconstruction drifts. Do NOT loosen the tolerance. Either expose what the game used
(the blast position, the eye point and the aim yaw the flash took — e.g. the sim records them where it calls
`flash_amount`) and make the test assert against exactly those, or show the timing change is wrong. Start from
`origin/s/frag-throw-timing` merged with the newest origin/master; keep its own frag_throw_timing_live green.

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

**Branch:** `s/frag-throw-timing2`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-17T03:10Z

---
## Report (done, 2026-10-17T03:34Z)

Closed by the driver: the report is on `s/frag-throw-timing2` (docs/reports/frag-throw-timing2.md); the worker didn't run finish.sh.
