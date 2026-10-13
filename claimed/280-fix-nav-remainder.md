## Your task: `nav-remainder`, guard navigation remainder

GAPS G-027's open items: GoToGoalPosition 0x63 with `SetGoalToCloserPosition` / `SetGoalToDroneWithLosToOpponent`; the
"drone ahead" flags `+0x31C` (state 0x5E); obstacle avoidance; the door/kick/action nodes (0x81–0x84); EnemyMission (kind-8
paths); `PatrolTalk`'s RNG use; the boundary push-out collision. The user: "All of the AI in this level is bugged as hell" (0c).

Order: the door/kick/action nodes first (NPCs and doors are a repeated complaint), then GoToGoalPosition, then the rest.
For each: an NF_ISO test on a level where it happens (find one by placement/state census), compared with a trace when one
exists (`tests/traces/*.trace`, the ai_traces_live harness).

Files: `crates/nf-game/src/drone.rs` (nav arms), the nav module, docs/systems/drone-navigation.md.

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

**Branch:** `s/nav-remainder`.

---
claimed-by: sandbox1 at 2026-10-13T20:14Z
