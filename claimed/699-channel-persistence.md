## Your task: `channel-persistence`, switch channels must persist until written (every level's buttons and doors)

Found by task 750q (docs/reports/cutscene-guard-models.md, "Open port bug"): a switch-channel value written
from outside the channel's own owners is cleared by some port system within one tick — `Channels::set(0x33, 1)`
then one `step_bond` reads it back 0 — where the game's `switch_channels` array (0x26fc90) persists until
something writes it again [M: find every writer]. The 0700004a lift's ch-0x22 stand-in (nf-game nis.rs ~1300)
has the same shape. Buttons, switches, lifts, doors and objectives on every level communicate through these
channels, so this can break level progression anywhere.

1. Find which port system clears or recomputes channels each tick (a mux/AND/OR recompute writing every
   channel, a reset in the switch tick, a per-frame clear) and what the game actually does per channel kind
   (muxes recompute their own OUT channel only; triggers write theirs on fire) [M].
2. Fix it so only the game's writers write; a test that sets a channel nobody owns and checks it holds for
   600 ticks, plus a census across all on-foot levels: for each level, which channels the port rewrites each
   tick vs the game's writer set.
3. Re-run the level walk tests and the lift/door tests; report any that change.

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

**Branch:** `s/channel-persistence`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-17T14:51Z
