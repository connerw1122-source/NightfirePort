## Your task: `drive-ptp`, driving phase 2: the PTP block and the ACTION↔DRIVING handover

Read `docs/atlas/driving.md` §2 first. Port the PTP block (0xa50 bytes, built by `Boot_GetPTPData` 0x1c8198) as a Rust
struct with every field the decompile writes and reads on both sides (ACTION fills it; DRIVING reads the level key at
+0x4f4; the return writes the result flag +0x97c and the next level key), plus `PlrStats_Get/SetDrivingData`'s 12 slots.
In the port: when a mission reaches its driving level (the levels.md list: e.g. after 07000004 → the Paris drive), build the
block from the Sim's state, skip the drive with a result (a dev choice: success/fail), and resume ACTION at the level the
return names, with the stats carried. Tests: the block built from a known state (field values), and the resume target for
each driving mission.

Files: a new `crates/nf-game/src/ptp.rs`, the level-transition hook in nf-view (minimal), docs/atlas/driving.md (status).

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

**Branch:** `s/drive-ptp`.
