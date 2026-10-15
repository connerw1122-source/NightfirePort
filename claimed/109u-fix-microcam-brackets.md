## Your task: `microcam-brackets`, the Micro-Camera's green target brackets

The user (playtest 1015-0046, `nf-view <iso> 07000005 --dev --give 0x1a,0x54`, Micro-Camera zoomed with right click):
"The micro camera, though, has the ability to put these green brackets around enemies, even through walls. And that's
not in this." The real game's Micro-Camera view (0x54/0x55) draws a green bracket box around every enemy on screen,
visible through walls. Find the game's code: the camera pane (`HUD_UpdateCameraPane` 0x1a12d8 / its create, pane 11 —
see `docs/fields/hud-panes.md`, "camera = 1 slot") and the per-drone projection (`View_3DPoint2Screen` like
`HUD_UpdateRedeemerPane` 0x19e178's copter bracket), the bracket sprites and their colour/scale rules, and port them in
`crates/nf-view/src/hud.rs` next to the zoom overlays. Render it with `--shot` (`--give 0x54 --draw 0x54 --zoom-hold`
on 07000005 with guards in view) and describe the picture in the report. Tests (NF_ISO) for the bracket draw list
(one box per on-screen enemy, none for friendlies/corpses, drawn through a wall).

Files: `crates/nf-view/src/hud.rs`, a new test file.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/microcam-brackets`.

---
claimed-by: sandbox3 at 2026-10-15T07:34Z
