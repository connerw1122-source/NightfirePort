## Your task: `p2k-fire-07`, the P2K never fires on 07000007 after the disguise lock

Found by level-07-interactables (707a, interact_07000007_live `the_party_vase_breaks_on_a_p2k_round`, left
`#[ignore]`): after the hall walk-in releases the weapon lock (ch 0x48, the P2K auto-drawn — weapon 2 out),
holding fire through `Sim::step_bond` for 100 ticks never spawns a shot (`last_shot` stays `None`). If the
real player can't shoot on 07 after the lock lifts, that is a game-breaking bug on the level. Find the fire gate
that holds it (suspects from the worker: the weapon-lock auto-draw's switch state vs the fire path's lockout
[M Player_Weapon 0x1a24c8:242-275]), compare with the game, fix it, and un-ignore the vase test (the vase,
statue #109 and the window #298 must break on the real path). Sister: the shot is still refused while the lock
channel holds.



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

**Branch:** `s/p2k-fire-07`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-17T15:59Z

---
## Report (done, 2026-10-17T16:42Z)

