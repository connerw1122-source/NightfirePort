## Your task: `context-hints`, the context hint and the orange 007 ring

PCSX2 at two spots (main agent, 2026-10-13): standing at a usable thing, the real game shows (1) a hint bar at the top,
e.g. "Hint: Use Watch Laser" in front of the 07000008 safe (`--bond 155.19,43.12,-113.38,1.57,0`), and (2) the HUD
ring bottom-left turns into an ORANGE "007" icon (at the safe, and in front of the 05 grapple point at
`--bond 16.5,29.6,58.6,-2.01,0.24`: the Bond-moves/context indicator). The port shows neither (its ring stays the green
radar). Find the game's context system (the third icon / `ThirdIcon_Update` 0x196e80 and `BLData+0x95F` context that
fix-08-walls traced, the hint text source in the language bank, the HUD's ring swap and its sprite) and port: the hint
bar with the game's text, the ring icon per context (grapple, laser target, use, ...), shown and hidden when the game does.
Tests: at the two spots the context and the hint label id equal the game's (numbers/label ids only); away from them,
none; sister assertions.

Files: `crates/nf-view/src/hud.rs`, `crates/nf-game/src/player.rs` (the context), `docs/systems/hud.md`.

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

**Branch:** `s/context-hints`.

---
claimed-by: sandbox2 at 2026-10-14T05:59Z

---
## Report (done, 2026-10-14T08:42Z)

Closed by the driver: the report is on `s/fix-context-hints` (docs/reports/fix-context-hints.md); the worker didn't run finish.sh.
