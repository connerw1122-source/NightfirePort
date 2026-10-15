## Your task: `shading-parity`, shading and lighting: match the game's look

The user, comparing the 07000007 Kimiko scene side by side (PCSX2 vs nf-view): "The shaders and lighting look much
nicer than our port." The main agent's compare (port `nf-view <iso> 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 600`
vs the real game's close-up): the game's characters have soft, smooth, warm directional shading on faces and skin with
a clear light side and shadow side; the port's look flat, greyer and harsher. The port's lamp-lit walls are brighter
with a harder falloff, and its edges alias (no AA).

Find the game's lighting model from the code and port it exactly:
- characters: how many directional lights per character, where they come from (the cel/room lights,
  `Lights_CalcAmbientLight` 0x1cb178, the scene/script lights, the VU1 lighting microcode's inputs), the ambient +
  directional sum, the PS2 colour scale (0x80 = 1.0, the x2 modulate), any rim/specular/environment map on skins and
  clothes (Bond's suit, hair);
- world: the baked vertex colours and their x2 scale, lightmaps if any, the dynamic light falloff;
- texture sampling: bilinear filtering and mipmaps where the game uses them (the GS TEX1 settings);
- edges: MSAA or a resolution scale option in the port (the game runs 640x448 interlaced; smooth edges are expected).
Report the model with addresses, then render the Kimiko close-up (frames 600, 1000, 1400) and 05's trench before and
after. Use PCSX2 captures for shape and light direction only — they carry a CRT filter (colour/brightness unreliable).

Files: `crates/nf-view/src/{gpu,scene}.rs` and the shaders.

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

**Branch:** `s/shading-parity`.

---
claimed-by: sandbox3 at 2026-10-14T23:52Z

---
## Report (done, 2026-10-15T01:24Z)

