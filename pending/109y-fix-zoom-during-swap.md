## Your task: `zoom-during-swap`, right click held through a weapon switch

The user (playtest 1015-0046, `--give 0x02,0x1e,0x1c`): "If I'm holding aim while swap, like, say I'm on the shotgun and
I swap to the sniper rifle, I swap off the shotgun and before the sniper rifle comes up and I'm pressing right click,
it doesn't actually zoom in once the sniper rifle's finished. And when I let go of right click, it like does like a
half zoom, but never actually zooms in." Right click is the scope button on the mouse (`Input::scope`, the aim gate
`Sim::aim_gate` in `crates/nf-view/src/scene.rs`; it only reaches the aim bit on +0x68 & 0x40 weapons). Find how the
game handles the aim button held across `Player_WeaponChange`/the draw (`Player_Weapon` 0x1a24c8, the aim raise
`Player_SetWeaponAnimObj` case 0xf 0x1a2e38:554-561, `Player_Zoom` 0x1a1a70) and make a held right click zoom the sniper
once its draw finishes, and the release zoom back out cleanly. Tests (NF_ISO) through `Sim::step_bond` with `scope`
held across the switch.

Files: `crates/nf-view/src/scene.rs` (the aim gate), a new test file.

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

**Branch:** `s/zoom-during-swap`.
