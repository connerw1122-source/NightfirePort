## Your task: `0c-pylon`, the 0c pylon isn't on screen (the drawn set says it is)

102e-fix-missing-objects fixed the 06 fountain, the guard-room furniture and the lockers (render-verified against
PCSX2). Its report says the 0c pylon "is not reproducible: all four probes show the pylon drawn" and pins it with
`the_0c_pylon_is_in_the_drawn_set_at_the_user_spot`. But the RENDER disagrees: `nf-view <iso> 0700000c --bond
14.22,16.28,-139.54,0.986,-0.269 --skip-cutscenes --frame 30 --shot x.png` (master + the branch) shows the roof, the
crates and the pylon's baked SHADOW on the ground, and empty sky/sea where PCSX2 at the same spot shows the steel lattice
pylon right of centre (its four parts `pylon`/`pylon_back`/`pylon_left`/`pylon_right`, placements 160-163 at x 44-60,
z -115..-106; the camera looks toward +x,+z, so it's in view).
"In the drawn set" isn't "on screen". Follow the pylon's draws all the way: the material (alpha test / blend: a lattice
is an alpha-tested texture, so check the alpha-test threshold and whether its texels load with alpha), the depth/near-far
range, the vertex positions after the world-baked transform (a (0,0,0) translation on world-baked geometry: is it
being transformed twice?), the cel visibility on the actual frame. Fix it and replace the drawn-set pin with a test on
the rendered frame (or the exact GPU draw list: projected inside the viewport, depth in range, non-zero alpha);
sister assertions. Render the spot before/after.

Files: `crates/nf-view/src/{scene,gpu}.rs`.

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

**Branch:** `s/0c-pylon`.

---
claimed-by: sandbox4 at 2026-10-14T09:59Z
