## Your task: `fix-striped-boxes2`, striped boxes: hide only what the game hides

Base: `s/fix-striped-boxes` (merge origin/master into it). HELD at merge: its 0x10 no-draw gate in scene.rs's misc sweep
(FLAGS_COPYING = 0x30 0x34 0x35 0x3b 0x71 0xe1 0xe3 0xfa 0xfd 0x4a) hides far more than the striped markers, and
`models_misc_live` (misc_census_golden + misc_static_entities_match_the_census_on_every_level) fails on the merge
because its census doesn't apply the gate. What the gate hides on the disc (main agent's probe, name / flags /
batches using the 32x32 red-yellow marker texture):

- 07000005: WallHug_Blocker1_Short 0x10 (1/1 striped), WallHug_Blocker2_Windows 0x10 (1/1), TouchPlate_GrappleLedge 0x50
  (1/1), Valley grapple_touchplate 0x50 (0/1)
- 07000008: SmallExplosionBox 0xe1 0x30 (0/1); 0700000d: WallHug_Blocker1_Long 0x10 (0/1)
- 07000011: 2x 3RD_Person_Icon 0xfa 0x50 (0/1)
- 07000012: "Computer to hack" 0x35 0x50 (0/5 batches), ExplosionBox 0xe1 0x30, 3RD_Person_Icon 0xfa
- 07000014: 12x SWITCH_Consoles_Evilbase 0x30 0x50, 19x DESTROY_BoxSatellite 0xe1 0x30, 3x 3RD_Person_Icon 0xfa
- 07000015: 3x DESTROY_BoxSatellite; 0700001b: 8x Apocalypse 0x4a 0x30, DESTROY_BoxMissile/BoxSatellite/BoxSmall 0xe1

Only the first three are the striped markers. For EVERY kind in the gate, cite the creator's own instructions that copy
`glist+0x44` into obj+0xF0 (address + the store), as the report did for 0x3b/0xfa/0x34; the report's "0x30/0x35/0x71/
0xe1/0xe3/0xfd/0x4a the same way" is not evidence. Also check whether anything CLEARS bit 0x10 later for those kinds
(a destroyable's break, a console's activation, a hack target's state, the 0xe8 LOAD_LEVEL precedent) and whether the
object is meant to be seen (e.g. is "Computer to hack" the visible laptop, or an invisible use volume over world
geometry?). Keep the gate only where both hold. For each object above, state "drawn" or "not drawn" in the game with the
evidence, and give a `--bond` spot for each kind so the main agent can compare against PCSX2.

Update `models_misc_live`'s expected_misc to apply the same rule, and re-pin `misc_census_golden` with the per-level
line diff in the report. Render (nf-view --shot) the 05 trench spot (`--bond 16.67,30.04,58.60,1.755,-0.001`: the
striped box must be gone, the pulley still there) and one spot each for 12's "Computer to hack" and 14's consoles,
before and after.

Files: `crates/nf-view/src/scene.rs` (the misc sweep's gate, small local edit), `crates/nf-view/tests/
{models_misc_live,fix_striped_box_census}.rs`.

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

**Branch:** `s/fix-striped-boxes2`.
