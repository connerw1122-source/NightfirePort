## Your task: `ammo-pane`, the HUD ammo pane: counts and the mode label

At 0700000c's walkway with the sniper rifle out, the real game's bottom-right HUD shows the ammo pane — an ammo icon,
"5-30" (clip-spare) and the fire-mode label "Single" — while the port shows only the weapon NAME ("Winter Tactical
Sniper") [main agent's PCSX2 vs nf-view compare, `--bond 14.22,16.28,-139.54,0.986,-0.269` frame 30]. Port the game's
ammo pane (`HUD_UpdateAmmoPane` 0x19a7f0 and its create): when the name shows (on switch, how long) vs the
counts/icon/mode label, the layout and font, the per-ammo-type icon, gadgets and weapons without ammo. Tests: for 4
weapons (P2K, sniper, MGL, a gadget) the pane's text, icon id and timing equal the game's (numbers); sister
assertions.
VISUAL CHECK RULE: if anything you change is visible, render the spot with
`nf-view <iso> <level> --bond x,y,z,yaw,pitch --skip-cutscenes --frame N --shot x.png` before and after and describe
both in the report. The main agent's PCSX2 captures have a CRT filter on: use them for presence/placement/layout,
not exact colour or brightness.

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

**Branch:** `s/ammo-pane`.

---
claimed-by: sandbox6 at 2026-10-14T21:04Z

---
## Report (done, 2026-10-14T22:30Z)

