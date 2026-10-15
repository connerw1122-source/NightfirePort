## Your task: `cutscene-letterbox`, cut-scenes: the letterbox bars and the framing

Main agent's PCSX2 vs port compare of the 07000007 Kimiko scene (trigger ch 0x48; port `--bond 94.2,44.6,-45.9,0,0
--frame 200..1600`): every shot, pose, costume and cut matches, but (1) the real game draws black LETTERBOX bars top and
bottom through the whole scene and the port draws none (277-fix-script-commands ported `Script_CameraStart` 0x1d9140's
bars with SpriteInfo 0x2c9468 — find why they don't show here: the flag the scene's script sets, the bar height, the
fade in/out of the bars), and (2) the port's widescreen view shows more at the sides than the game's 4:3 frame — e.g. at
frame 400 Dominique waiting in the left doorway for her entrance is visible in the port but outside the game's frame.
Make cut-scenes show what the game shows: the bars at the game's size and timing, and the scene's horizontal extent
matching the game's (pillar-box or the game's own aspect for cut-scenes; keep gameplay widescreen). Render frames 200,
400, 1000 and 1400 before and after and describe them.

Files: `crates/nf-view/src/{scene,lib,gpu}.rs` (the cut-scene camera/draw, small).

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

**Branch:** `s/cutscene-letterbox`.

---
claimed-by: sandbox2 at 2026-10-14T22:49Z

---
## Report (done, 2026-10-15T00:38Z)

