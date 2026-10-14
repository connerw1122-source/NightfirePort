## Your task: `fix-exact-math-swap3`, exact math: why the cut-scene camera moves

Base: `s/fix-exact-math-swap2` (merge origin/master into it). HELD at merge: renders of master vs the branch differ by
far more than float precision can explain.

- `nf-view <iso> 07000002 --bond -82.30,5.30,32.05,1.570,-0.100 --frame N --shot x.png` for N = 140, 170, 200: on the
  branch the intro's cut-scene camera sits consistently further right and closer to the bald man (Bond at the left edge
  instead of centre-left). Same characters, same poses; the CAMERA transform differs at every frame, so it isn't a
  timing shift.
- `nf-view <iso> 07000007 --bond 42.59,44.10,-71.71,-1.228,0 --skip-cutscenes --frame 170`: the guard on the left of the
  hall is gone from the frame on the branch.

A bit-exact `sinf` differs from the host's by ~1 ulp; that cannot move a camera visibly. Something else in the
swapped path changed behaviour: a matrix convention (row vs column, the translation row), the quaternion order or
sign in `quat_trans_to_mat`, the angle units or wrap in `object_matrix`'s RotMatrix, or `anim::mul`'s operand order.
Find it: dump the NIS camera matrix (and one actor's) at frame 170 on master and on the branch, element by element,
locate the first function whose output differs by more than 1e-5, and decide from the decompile (RotMatrix 0x1e2368,
Quat_QuatTransToMat 0x1e3fa8, psiBuildMatrixPalette 0x111498) which side is the game's. If the branch is right, the
report must say exactly why master was wrong and by how much, with the decompile lines, so the main agent can confirm
against PCSX2; if master is right, fix the swap. Keep the grenade-bounce fix (`Bullet_CollisionHandler` 0x12ce80) either
way. Test: the frame-170 camera matrix equals the value the decompile's arithmetic gives (numbers); sister assertion.
Render both spots before and after and describe them in the report.

Files: `crates/nf-formats/src/{anim,placement}.rs`, the camera/NIS path that consumes them.

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

**Branch:** `s/fix-exact-math-swap3`.

---
claimed-by: sandbox7 at 2026-10-14T15:31Z

---
## Report (done, 2026-10-14T17:54Z)

