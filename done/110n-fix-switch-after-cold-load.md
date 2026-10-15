## Your task: `fix-switch-after-cold-load`, a weapon switch is ignored after a cold load

Seen by the main agent on 2026-10-15 while merging 109r (fix-start-loadouts): on a cold `nf_view::load` of
07000016.bin (the level's kit holds 0x11, `loadout: ... switch request 17`), giving the P2K
(`inventory.equip_weapon(&wd, 2, 7, true)` returned true, `owned[2]` true) and then stepping with
`Input { switch_to: Some(2), .. }` left `gun.weapon` at 0x11 after 90 ticks (`switch_request` still 0x11, state 0).
Setting `sim.gun.switch_request = 2` directly switched fine. The number keys and the dev panel go through `switch_to`,
so a player pressing 2 right after loading a level may get no switch.

Do:
1. Reproduce headlessly (NF_ISO test, the real input path) on 07000016 and on one other level whose cold kit differs
   from the P2K (07000012: 0x10).
2. Find the cause in the port against `Player_WeaponChange` 0x1a6f80 / the `anim+0x63` request handling (is the switch
   refused while a loadout request is pending? is `switch_to` consumed only on an edge? does the gate look at the wrong
   state?). Fix it to match the game: what does the game do with a number-key press during the level-start draw?
3. Test with a sister: the switch works after the start draw settles and on the first possible tick, as the game allows.

Files: `crates/nf-view/src/scene.rs` (the switch path) or `crates/nf-game/src/gun.rs`, the new test.

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


**Branch:** `s/fix-switch-after-cold-load`.

---
claimed-by: sandbox3 at 2026-10-15T12:00Z

---
## Report (done, 2026-10-15T13:04Z)

Closed by the driver: the report is on `s/fix-switch-after-cold-load` (docs/reports/fix-switch-after-cold-load.md); the worker didn't run finish.sh.
