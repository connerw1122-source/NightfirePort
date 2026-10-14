## Your task: `cutscene-faces`, cut-scene actors placed AND their faces moving

100e-fix-cutscene-draw (`origin/s/fix-cutscene-draw`, held) fixed the missing cut-scene cast: the face track listed first
in a script (Kimiko's 060004f1: windows [(1, 04000467) 22 channels, (1, 04000332) 222 channels]) took bones 0..5 in
`script_pose_together` (anim.rs) and zeroed the root, so every actor stood at the world origin. Its fix: the widest
track owns every bone it covers. The main agent rendered the scene with it: the whole cast is back (Kimiko, Bond,
Dominique, the kiss). But merged with master the suite fails
`fix_cutscenes2_live::the_kimiko_scene_characters_play_their_face_tracks` ("the face track must move its bones (frames
399): the mouth animation"): the body track now owns the face bones too, so the mouths freeze.

Do both, the game's way: the report itself says the game maps channels to bones through the SKIN's table
(`AnimFrameCopy` receives the sAnimSkin; `psiBuildMatrixPalette` 0x111498 reads `sAnimObject+0x94`), so a 22-channel
face track reaches only the face bones, and the body track the body. Port that mapping (channel index -> bone via the
skin/track table) instead of first-writer or widest-wins. Merge `origin/s/fix-cutscene-draw` first; keep its tests and
the cast render. Tests: the Kimiko scene's actors at their scripted world positions (not the origin) AND the face
bones moving over the face track's frames; sister assertions. Render
`nf-view <iso> 07000007 --bond 94.2,44.6,-45.9,0,0 --frame <150..1500> --shot` and check the cast is there.

Files: `crates/nf-game/src/anim.rs`, tests.

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

**Branch:** `s/cutscene-faces`.

---
claimed-by: sandbox7 at 2026-10-14T06:07Z
