## Your task: `objective-banner`, the objective banner text and timing, the fail message

From the user's playtests (`dist`-side results, 2026-10-12): on 07000007 the objective banner and its sound fire the moment the
Kimiko cut-scene starts instead of after it ends ("obj doesnt wait for kimiko cutscene and plays the sound immediately");
banners say a generic "NEW OBJECTIVE" instead of the objective's own text; the cut-scene fail message is off-centre.

1. **Timing:** find what holds the HUD objective pane while a script player owns the camera (`HUD_*`, `Mission_*`,
   `Objective_*`, the PlayerDisable/cut-scene flags in BLData) and port it, so the announce and its sound wait for the
   cut-scene's hand-back. Test: 07000007's Kimiko cut-scene through the real trigger; no banner and no 0x… sound until the
   PlayerEnable tick, then exactly one.
2. **Text:** the banner's body text comes from the objective's string id in the mission table (`docs/systems/mission.md`,
   USATxt.dat via `Lang`). Show the real line for each objective of 07000005 and 07000007 in a test (string ids, not text).
3. **Fail message:** centre it the way the game lays it out (the pane's x/justify fields), with a layout test on its rect.

Files: `crates/nf-game/src/mission.rs`, the HUD module(s) in nf-view, `crates/nf-view/src/scene.rs` (hooks).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing** (start it early, in the background, so it finishes inside your session):
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count. If your change adds `Rand_*` draws, seed-pinned
  tests may move: re-pick them by the WORKLOG 0072 procedure (`NF_TEST_SEED` search against the same live expectations).
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/objective-banner`.

---
claimed-by: sandbox2 at 2026-10-13T16:08Z

---
## Report (done, 2026-10-13T17:33Z)

