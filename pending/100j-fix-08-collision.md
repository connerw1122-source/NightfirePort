## Your task: `08-collision`, 07000008: an invisible wall blocks the safe room

The user's playtest (2026-10-13 night, 07000008 at `--bond 155.19,43.12,-113.38,1.57,0`, portal culling ON): "the wall
is still there but invisible". The exterior room `Room_Outside_3`'s mesh (graph cell 15) has wall slabs whose
triangles cross the whole `Room_Guardhouse_F2` interior (fix-08-walls / fix-safe08: "20 triangles centred INSIDE the
bedroom interior"). Culling hides them visually, but their COLLISION still blocks Bond inside the room. The game
collides Bond only against his own cel's object lists (fix-portal-sweep's `build_FindCel` 0x1c8cb0 finding: per-cel
lists, not one shared world), so those exterior triangles never block indoors. Give the port's collision the game's
per-cel attribution (coordinate with 101j-fix-portal-eyecel, which needs the same per-cel lists for the eye cel: merge
its branch if it lands first), so walls from other rooms don't block. Test: inside the missile room, walking across the
room where the slabs pass meets no wall; the room's real walls still block; sister assertions.

Files: `crates/nf-game/src/world.rs`, `crates/nf-view/src/scene.rs` as needed.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/08-collision`.
