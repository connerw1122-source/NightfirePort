## Your task: `sound-remainder`, the sound remainder: guard foley, the environment, the SPU filter

GAPS G-032's open items:
1. **Drone foley** (`Drone_SfxInfo`, `NDrone2_PlaySFX`): footsteps, cloth, equipment, per surface material.
2. **The cel/room environment**: `SFXSetEnvironment`'s value from the room Bond is in (reverb per room).
3. **G-032d, the SPU2 interpolation**: replace the Catmull-Rom 4-tap with the SPU2's fixed-point Gaussian table (it's in the
   public nocash PSX-SPX documentation; cite it), with a sample-exact test on one VAG.
4. **Pause in cut-scenes**, the precache, and the op-1 per-weapon remaps for every weapon (coordinate with
   `272-fix-weapon-modes` if it's merged: only add what's missing).

Files: `crates/nf-audio/*`, `crates/nf-game/src/drone.rs` (the foley calls, additive), docs/systems/sound.md.

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

**Branch:** `s/sound-remainder`.
