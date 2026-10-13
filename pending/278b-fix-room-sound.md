## Your task: `room-sound`, the room reverb and the guards' foley calls

Read `docs/reports/fix-sound-remainder.md` "What's left" first; it has the evidence already gathered.
1. **Room environment:** `Sound_UpdateListeners` 0x1dd278 sends `SFXSetEnvironment(cel s16 +0x98, underwater = (cel+0x3c &
   0x10000000) || env >= 20)` from the active viewer's cel (glb_viewer[4] wins when its +0x29 is set). `cel+0x98` is
   written by `parseentity_fixup_entity` 0x1d00c0 (class 0xc023 -> 0x9898, the underwater sentinel; 0xc047 -> level_tag+0x50).
   Keep the cel env value in the port's map parse, track the viewer's current cel, and feed the real value into
   `sound_tick` (nf-view lib.rs's `drv.set_environment(0, 0)` stand-in). Pin 07000005's two values (env 0 / env 40) [L].
2. **Guard foley call sites** on `Drones::foley`: kick 0x28 [M 0x1684c0], interrogation 0x4a [M 0x161a40], civilian patrol
   0x48 [M 0x15d610], hostage-die 10 [M 0x1725a0], spawner 0xd6+Rand(2) [M 0x137e68], the pain pools [M 0x125ed8]. The pools
   draw from the shared `Rand_*` stream: re-pick the moved seed tests by the WORKLOG 0072 procedure against the same live
   expectations.

Files: `crates/nf-audio/*`, `crates/nf-game/src/drone.rs` (the foley calls), `crates/nf-view/src/{lib,scene}.rs` (the env feed).

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

**Branch:** `s/room-sound`.
