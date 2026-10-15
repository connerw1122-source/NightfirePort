## Your task: `aims20-lights-render`, the AIMS-20's four side lights never show red

109d (`s/fix-aims20-modes-lights`, merged) fixed the light GATES (bones 8..0xB of the view model, `frame_state`'s aims
arm in `crates/nf-view/src/scene.rs`), but the main agent's render of master shows the row still dark: at 07000005's
spawn (`nf-view <iso> 07000005 --dev --give 0x1a`, AIMS-20 drawn, in Burst AND in Grenade mode, standing still AND
strafing), the four round lights on the receiver are flat grey, never red. The real game (read
`docs/live/2026-10-14-weapon-measurements-pcsx2.md`, "The side lights", on master): the four lights are lit RED from the
moment the gun is drawn and STAY LIT WHILE IDLE; the ammo pattern (grenade rounds 0 → red, red, dark, red; burst dry →
red, dark, red, red) is the only thing that darkens one.

Two things to fix:
1. **The render.** Find why the lit mesh 0x2000132 attached to bones 8..0xB [M `Player_WeaponFiring` 0x1a5460:256-327]
   doesn't show: is it drawn at all (the attach path in the view-model draw list), drawn at the wrong place (hidden
   inside the receiver), or drawn with the wrong texture/colour/blend (an additive/self-lit material drawn as a normal
   lit one, which at night comes out grey)? Compare the mesh's material flags with how the game draws it. A
   draw-list test is not enough on its own: render the view model with `--shot` before/after and describe what shows.
2. **The idle all-off gate.** The port darkens the whole row after ~0.75 s with no stick input (0x1a5460:314's
   `FRAME_RATE * 45.0 < +0x93a`), but PCSX2 shows the row staying lit while idle. Re-read that gate in the DISASSEMBLY
   (not the C) — the 109d report's own "What's left" flags the `sVar5` chain at 0x1a5460:305-314 — and make the port
   match the measured behaviour, citing the instructions.

Files: `crates/nf-view/src/scene.rs` (frame_state's aims arm, the view-model draw), the renderer file that draws
view-model attachments (find it; list it in the report).


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

**Branch:** `s/aims20-lights-render`.

---
claimed-by: sandbox6 at 2026-10-15T03:43Z
