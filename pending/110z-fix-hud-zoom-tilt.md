## Your task: `fix-hud-zoom-tilt`, the view-model tilt with zoom and aspect (G-044)

G-044 (docs/GAPS.md; docs/fields/bldata.md §Live census): `Player_WeaponRecoil` divides the crosshair tilt by
`glb_viewer+0x108` (the zoom) and scales the x term by `viewer+0x114` (the aspect). The port's gun axes (`scene.rs`) use a
fixed 4/3 and no zoom divide.

Do: read `Player_WeaponRecoil` (and whoever writes viewer+0x108/+0x114). Port the zoom divide and the aspect term into
the gun-axis computation, using the port's zoom (`Sim::zoom`, BL+0x8D0) and the game's aspect value (find what +0x114
holds: 4:3, or the display aspect?). Test (NF_ISO): at zoom 2 the tilt term is half its zoom-1 value, as the game's
formula gives, and the aspect term matches the decompile's constant. Sister: zoom 1 is unchanged from master,
bit-for-bit.

This changes a view-model pose number. The main agent will render-check it before merging, so keep the change small
and list in the report the exact spots (weapon, zoom) where the pose differs from master.

Files: `crates/nf-view/src/scene.rs` (the gun axes), the new test, docs/GAPS.md (G-044).

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


**Branch:** `s/fix-hud-zoom-tilt`.
