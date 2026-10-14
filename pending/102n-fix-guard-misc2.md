## Your task: `guard-misc2`, continue the guard fixes (sniper aim, 06 doors, start poses)

Continue 102i-fix-guard-misc: its session ran out of time before any fix (branch `origin/s/fix-guard-misc`, report
docs/reports/fix-guard-misc.md on that branch: read it first). It left failing repro tests in
`crates/nf-view/tests/fix_guard_misc_live.rs` and narrowed the causes:
1. **The 0c rooftop sniper takes no damage** (drone #17 at (43.77, 21.43, -205.13)): the damage path and his 16 hit
   boxes are sound; the bug is Bond's AIM at that range/pitch (the aim-assist / pitch handling): the bullets miss.
   Find the game's aim-assist and pitch (`Player_AutoAim`, the aim ray from `Player_GetAimingPoint`) and fix it so the
   shot hits as in the game.
2. **06 guards never open doors**: in 7200 ticks no drone enters `DSTATE_OpenDoor` (0x81) and no door opens without
   Bond. Find the route nodes with doors on 06 and why the dispatch (`NDrone2_ReachedDestNode` door case) never
   fires (the nodes' door binding, the drone's path through them).
3. **06 "crazy models" at the start**: not reproduced as unposed animations; the report asks for renders. Render
   `nf-view <iso> 07000006 --frame <1,5,30,60> --shot` from the truck start and describe what's wrong in the first
   frames (T-poses, stretched limbs, wrong blend), then fix it.
Merge `origin/s/fix-guard-misc` first to keep its harnesses. Each fix: its repro turning green, sister assertions, a
full suite on the merged tree with 0 failures.

Files: `crates/nf-game/src/{drone,door,damage,player}.rs`, `crates/nf-view/src/scene.rs`.

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

**Branch:** `s/guard-misc2`.
