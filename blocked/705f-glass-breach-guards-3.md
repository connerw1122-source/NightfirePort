## Your task: `glass-breach-guards-3`, finish the 08 glass-breach guards (HELD at merge again)

`s/glass-breach-guards-2` (705c, 0e5fbd7) was held: merged onto origin/master it breaks
`interact_07000001_live::bullets_shatter_the_glass_and_vases_and_the_sister_stays_whole` (interact_07000001_live.rs:171
"unshot scenery stays whole (the sister)") — bisected to this branch (master alone passes). Your guard glass-breach path
is breaking 01's breakables that nobody shot. Note origin/master now draws breakables through the posed door/SS slots and
hides dead ones (`Break_Kill` 0x12b970); use `crate::breakable` (`Drones::breaks`), not a second system. Scope the
guard breach to the game's rule [M Break_Update 0x12bfd8: class-0x02 contacts damage only on 0x7000001 kind 2], make
that sister pass alongside your own tests.

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

**Branch:** `s/glass-breach-guards-3`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-17T18:42Z

---
## Report (blocked, 2026-10-17T19:47Z)

