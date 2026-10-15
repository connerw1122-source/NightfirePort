## Your task: `fix-door-react-path`, Door_ReactToHit through the game's collide path (G-035)

G-035 (docs/GAPS.md): the port's doors call `Door_ReactToHit`'s anti-crush through a sphere-test stand-in, and since
A-11b it no longer runs from `Door_Update`. The game calls it from `Collide_Update` through its proxy list. The
`Door_ReactToHit` proxy-list path (U-041-like) and the use-sphere vs collision-mesh test are open.

Do:
1. Read `Collide_Update` and `Door_ReactToHit` in the decompile: who calls it, with what object pairs, and when in
   the frame. Document it in `docs/systems/doors.md` (the existing door doc) with addresses.
2. Port the call from the matching place in the port's collide step. Replace the sphere stand-in with the game's test
   if the decompile shows it (collision mesh or proxy box).
3. Test (NF_ISO): a door closing on Bond reverses the way the game's rule says, and a guard in the doorway does the same.
   Sister: a door with nobody in it closes. Keep `door.rs`'s existing tests green.

Files: `crates/nf-game/src/door.rs`, the collide caller, `docs/systems/doors.md`, the new test.

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


**Branch:** `s/fix-door-react-path`.
