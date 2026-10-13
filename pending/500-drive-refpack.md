## Your task: `drive-refpack`, driving phase 0: the RefPack decompressor and the decompressed census

Read `docs/atlas/driving.md` and `docs/formats/driving-viv.md` (research-driving) first; this is §6 phase 0.
1. Write a RefPack (EA's 0x10FB LZ) decompressor in `crates/nf-formats/src/refpack.rs` from its documented format (cite the
   source you use), with unit tests on hand-built streams covering every opcode class.
2. Decompress every RefPack entry of the 23 driving archives (the DRIVING folder: `NF_DRIVING`, or extract it from the ISO
   with 7-Zip to a folder OUTSIDE the repo) and pin the decompressed sizes and SHA-1s in a golden (numbers only), with a sister.
3. Cross-check against an independent decompressor if one exists for you to run (e.g. a Python one in a venv under
   `work/third_party/`, never committed); on any disagreement, find out why.

Files: `crates/nf-formats/src/refpack.rs` (new), `crates/nf-formats/tests/driving_golden.rs`, `golden/`, docs/formats/driving-viv.md.

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

**Branch:** `s/drive-refpack`.
