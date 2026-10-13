## Your task: `fix-wallhug-freeze`, the 07000005 wall-hug plate must not freeze the NPCs

**Real play contradicts the port** (main agent + the user, 2026-10-12; `tests/traces/05-gate-swap.trace` on master,
07000005 played from the start, the gate guards left alive, god mode):
- frame 174328: the player steps on the ledge plate #301 → ch 0x43. The user saw **no cut-scene**, only the shimmy.
- frame 174329: the spawned gate pair is removed AND the two placed guards behind the gate activate (state 1 → 4).
- **No NPC freeze:** the two patrolling guards keep walking through the next 1,500 frames, exactly as before the
  plate (the trace's per-window movement: 2/12 drones moving before, 2/10 after, no state stall). `Drone_EnableAll(0)`
  stops every drone, so SP #298's event 5 did not run.

**The port (after fix-script-start-tick, merged):** the plate starts SP #298 (script 0x06000AD4) in the same walk;
its event 5 freezes every NPC for ~1,400 ticks, holding the gate pair until the script ends. That matches a test the
main agent ran by writing ch 0x43 by hand over PINE, but not real play.

**Find why the game doesn't run SP #298 when the player triggers the plate**: its trigger condition beyond ch 0x43
(the SP's mask/mode, `SP_LoadScript` 0x1dbc08's arming, a second channel, a once-only flag, Bond's creep/shimmy state,
a difficulty or progress gate), or whether the script only freezes in some context. Then make the port do what real
play shows: the swap on the next frame, no freeze. Re-assert `spawner_kill_live.rs` against the trace, frame for frame.

Tests (NF_ISO, through the real input path, fail-first): walk onto plate #301 → the next tick the gate pair is gone
and the placed pair active, and a patrolling guard keeps moving; the sister: whatever condition DOES start SP #298
still freezes.

Files: `crates/nf-game/src/{nis,spawners,drone}.rs`, `crates/nf-view/tests/spawner_kill_live.rs`.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-wallhug-freeze`.

---
claimed-by: sandbox6 at 2026-10-13T06:18Z

---
## Report (done, 2026-10-13T07:52Z)

