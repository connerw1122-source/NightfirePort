## Your task: `fix-death-retry-boost`, the restart's health floor after a death

`Player_RamLoad(Bond, 1)` (0x1957a8) is what the fail menu's Restart calls (`P_ENDMISSION_Handler` 0x216618:19-21). Its
second argument (the "boost") raises Bond's restored health to a difficulty floor, `ContinueHealthBoost*` (all 50.0 on the
retail build, [L] ee_start_05.bin 0x30cc34..3c). The port doesn't do this yet: `crates/nf-game/src/player.rs` ~line 1204
says so ("The `boost` argument ... is not ported"), so a restart after a death at low health brings Bond back at that low
health.

Do:
1. Read `Player_RamLoad` 0x1957a8 in the decompile (`work/decomp/ACTION.ELF/001957a8_*.c`) and write down exactly what
   the boost does: which field, which difficulty table entry, a max or a set, and armour or not. Put it in
   `docs/systems/mission.md` §8 (the death/restart section) with the addresses.
2. Port it: the fail-menu Restart path passes boost = 1; the trigger-exit warm load passes 0.
3. Test (NF_ISO, `crates/nf-view/tests/fix_death_retry_boost_live.rs`): on 07000005, drop Bond's health to 10, take the
   entry checkpoint the way the level start does, kill him, choose Restart (`fail_choose_restart`), then assert the health
   after the reload equals the game's rule (50 if the floor is a max). Sister assertion: a trigger-exit warm load keeps 10.
   Use `crates/nf-view/tests/fix_death_restart_live.rs` as the model (its helpers `load_carry`, `tick`, `exit`).

Files: `crates/nf-game/src/player.rs` (the RamLoad), `crates/nf-game/src/death.rs` (if the restart call lives there),
`docs/systems/mission.md`, the new test.

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


**Branch:** `s/fix-death-retry-boost`.

---
claimed-by: sandbox2 at 2026-10-15T10:05Z
