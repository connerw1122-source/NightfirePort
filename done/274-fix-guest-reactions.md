## Your task: `guest-reactions`, civilians when Bond attacks one, and the death replay loop

The user on 07000007: "the moment I started punching a guest, all the other guests T-posed. And once I died, we went into a
replay loop where it kept replaying at the start of the mission over and over again"; also "If you punch a guard, it should be
an automatic mission fail" (protected actors, `docs/systems/actors.md`).

1. **The T-pose:** reproduce it (punch a guest on 07000007 headless) and find the state the other guests enter (cower/flee/
   alert: the civilian states in the NDrone2 table) and which animation they're missing. Port the state's anim choice.
2. **The replay loop:** death → the results/restart page (fix-menus' P_NFRESULTS / P_ENDMISSION) → one restart. Find why the
   port re-enters the death sequence repeatedly and fix it. Test: die once, assert exactly one restart.
3. **Punch = fail:** a fist hit on a protected actor fails the mission the same as a shot (`0x145bd0`/`0x13e560`). Test it.

Files: `crates/nf-game/src/drone.rs` (civilian arms), `crates/nf-game/src/mission.rs`, `crates/nf-view/src/menu.rs`.

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

**Branch:** `s/guest-reactions`.

---
claimed-by: sandbox2 at 2026-10-13T17:35Z

---
## Report (done, 2026-10-13T18:59Z)


---
## Report from the worker (done, 2026-10-13T19:07Z)


---
## Report from the worker (done, 2026-10-13T19:08Z)

