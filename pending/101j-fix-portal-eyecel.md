## Your task: `portal-eyecel`, the eye cel: the 05 trench walls still vanish with portal culling on

130g-fix-portal-sweep (merged at c3b989b) added `scene::near_los_rescue` for the 07000005 trench spot, and its tests pass,
but the RENDERED frame is unchanged: `nf-view <iso> 07000005 --bond 16.67,30.04,58.60,1.755,-0.001 --skip-cutscenes
--frame 30 --shot a.png` still draws the trench with the near stone walls missing (sky and far mountains where the walls
are), and with `NF_PORTAL_DEBUG=1` the run prints the `[pwalk]` walk from cell 14 but no `[rescue]` line at all: the
rescue restores 0 cels in the real draw path. `NF_NOPORTAL=1` draws the walls. The playtest launcher keeps portal culling
off until this is right.

Read docs/reports/fix-portal-sweep.md first: it found the ROOT cause. The game's eye cel at this spot is 15 (the trench
room), the port's is 14 (the giant outdoor room), because `build_FindCel` 0x1c8cb0's tie-break casts its up/down rays
against each CANDIDATE's own object list, and the port's stand-in scopes the shared collision world, so both candidates
tie and 14 wins. Fix that instead of a rescue heuristic: give each cel its own collision/object list the way the game's
loader attributes polys and objects to cels (`build_link_objects_to_rooms` 0x1c9150 is already ported for the render
chains; find the collision side), so the eye cel resolves like the game's. Then remove the rescue if it's no longer
needed. Prove it on the RENDERED path: a test that renders (or builds the exact draw list `--shot` uses) at the trench
spot and asserts the near wall instances are drawn, failing on the current master; the eye cel is 15 there; the 27-level
sweep stays clean; the four PCSX2 ground-truth spots in docs/live/2026-10-13-portal-ground-truth.md still match (hidden
rooms stay hidden). Include headless `--shot` commands for the trench spot with and without NF_NOPORTAL; the two
images must show the same walls.

Files: `crates/nf-view/src/{gpu,scene}.rs`, `crates/nf-game/src/world.rs` (the per-cel lists), tests.

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

**Branch:** `s/portal-eyecel`.
