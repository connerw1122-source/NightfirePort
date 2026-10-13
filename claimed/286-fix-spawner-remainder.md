## Your task: `spawner-remainder`, spawners: the wave delay and the power-station spawn sound; 01's missing guard

1. GAPS G-025: the spawner mode-3 wave-delay byte (+0x1F) and the power-station spawn SFX branch (`DroneSpawner` `Control`
   0x137e68). Tests on a level with a mode-3 spawner (find it by census).
2. GAPS G-040: 07000001's second gate guard (placement 277) is gated by a channel set by script 0x0600007f, which isn't in the
   kind-7 NIS archive, so it self-deletes. Check whether this is still true on master; if so, load kind-5 scripts / the global
   script archive the game uses, and show the guard appear on his channel.

Files: `crates/nf-game/src/spawners.rs`, `crates/nf-game/src/nis.rs` (script loading), docs/systems/actors.md.

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

**Branch:** `s/spawner-remainder`.

---
claimed-by: sandbox4 at 2026-10-13T22:59Z
