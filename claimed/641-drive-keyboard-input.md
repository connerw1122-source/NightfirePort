## Your task: `drive-keyboard-input`, the driving missions on keyboard and mouse

The user plays with **keyboard and mouse only** (no pad). The driving missions' input path (110w,
crates/nf-game/src/input/driving.rs + docs/systems/driving-input.md) is the game's pad mapping. Design and port the PC
mapping onto the game's driving actions: steer (A/D or ←/→ with the game's own steering ramp applied to a digital
input — find how the game ramps a d-pad steer, if it does), throttle (W), brake/reverse (S), handbrake (Space),
fire (left mouse), cycle weapon (wheel/Q), use gadget (right mouse or E), camera view (C), look back (hold Tab or
middle mouse) — every driving action the game has — keeping the game's semantics (holds vs presses, repeat
rates). Expose the bindings as a table the main agent can show (key, the game's action label id from USATxt), next to
the on-foot table `PC_CONTROLS` in crates/nf-view/src/menu.rs. Tests: each key reaches the game's action with the
game's ramp/edge semantics (headless, numbers only). No pad work.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/drive-keyboard-input`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T01:09Z
