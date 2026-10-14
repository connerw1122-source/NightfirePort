## Your task: `fix-aim-raise2`, aim raise: rebuild on master's weapon-anims

Base: `s/fix-aim-raise` — merge origin/master into it FIRST. HELD at merge: master now carries fix-weapon-anims +
103k2-fix-weapon-anims2, which implement the aim raise/lower as a VIEW ANIMATION (fields `view_alert`, `prev_aiming`,
`view_aim_anim`; the raise +0xB8 @1.25 queued on the aim edge from the idle machine, the +0x93a idle-counter reset, the
idle branch's advance gated on `switch_state == 0`). This branch implements the same raise/lower separately (its own
`view_aim_anim` + `view_aim_raising`, advancing in `aim_gate` and gating the aim bit / zoom / auto-aim lock on the
finish). The merge conflicts in scene.rs in four places (the Sim fields, the +0x93a counter, the view-script advance,
the Sim constructor) and resolving them blindly would queue or advance the raise twice.

Rebuild the aim gate ON TOP of master's single raise/lower: one queue point, one advance, and the finish of master's
raise sets the aim bit (state 0xF, [M 0x1a2e38:554-561]) that the zoom, free-aim window and auto-aim lock wait for;
records without +0xB8 aim at once. Keep master's weapon_anims_live tests and this branch's fix_aim_raise_live tests
green, plus the 0c sniper (fix_guard_misc_live). Run the full `tools/test/suite.py`; total in the report.

Files: `crates/nf-view/src/scene.rs` (the aim block and the view-script block), `crates/nf-formats/src/weapon.rs`.

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

**Branch:** `s/fix-aim-raise2`.

---
claimed-by: sandbox2 at 2026-10-14T19:32Z
