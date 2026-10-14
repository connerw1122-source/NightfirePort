## Your task: `armour-ring`, the HUD ring with armour (the orange 007 seen in PCSX2)

100n-fix-context-hints2 fixed the hint bar's position and removed the stray grapple prompt, and found (its report, item
2) that the only 007-logo draw in ACTION.ELF is the trigger-driven "Bond Moment" zoom at (256, 80), not a bottom-left
ring swap; it did not guess. A likely explanation it couldn't see: in EVERY PCSX2 capture showing the orange 007 ring
bottom-left, the main agent's capture script (`tools/trace/pcsx2_shot.py --god`, `cheats.py`) was writing health 100
AND ARMOUR 50 (`BLData+0x8B0`) every frame. So the orange 007 ring may be the HEALTH/ARMOUR pane's armour state: the
health pane ring's tint/sprite changes when Bond has armour (`HUD_UpdateHealthPane` 0x19b9c8, `HealthSprInfo[]`,
the armour fields). Check the pane's code for the armour branch (which sprite and tint when armour > 0, the 007 logo
inside the ring?), port it, and test: armour 0 -> the green ring (as now); armour 50 -> the game's armoured ring;
sister assertions. Render both with `--shot` (the dev panel's armour row or a test hook to set armour) and describe them.

Files: `crates/nf-view/src/hud.rs`.

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

**Branch:** `s/armour-ring`.

---
claimed-by: sandbox1 at 2026-10-14T11:29Z

---
## Report (done, 2026-10-14T12:38Z)

Closed by the driver: the report is on `s/fix-armour-ring` (docs/reports/fix-armour-ring.md); the worker didn't run finish.sh.
