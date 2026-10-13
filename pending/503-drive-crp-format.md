## Your task: `drive-crp`, driving phase 1: the .crp track and car container

Read `docs/atlas/driving.md` and `docs/formats/driving-viv.md`. `.crp` (data\track\<level>.crp, data\car\model\*.crp) is the
driving side's core new format. It's RefPack-compressed, and task 500 ports the decompressor: if `s/drive-refpack` isn't on
master yet, merge it into your branch. Decode `.crp`'s container layout (chunks, tables, the track mesh, the car model and
its parts) from the data and DRIVING.ELF's consumer (task 501's map names it, or find it from the BIGF read of the track
entry), write `docs/formats/crp.md`, and add a parser with golden numbers for every .crp in the 23 archives. Use the full
retail image or the DRIVING folder (`NF_DRIVING`).

Files: `crates/nf-formats/src/crp.rs` (new), `crates/nf-formats/tests/driving_golden.rs`.

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

**Branch:** `s/drive-crp`.
