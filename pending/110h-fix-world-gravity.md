## Your task: `fix-world-gravity`, `WldGravity` per level instead of a constant

The port hard-codes Bond's gravity (`const GRAVITY: f32 = -9.8` in `crates/nf-game/src/player.rs`). The game reads a
global vector `WldGravity` (0x2d8890..0x2d889c; y at 0x2d8894 = −9.8 on 07000005, [L] ee_start_05.bin). It is used by
`Player_Collision` (the jump/fall) and by `Player_ChangeSubState` 0x190630 case 0xd/0xe, which picks the death anim by
it: `WldGravity.y == 0 ? 0x60008e0 : 0x6000151` [M :68-73]. The space levels are the likely zero-gravity ones.

Do:
1. Find every writer of `WldGravity` in the decompile (grep `DAT_002d8890`..`DAT_002d889c` / `WldGravity` across
   `work/decomp/ACTION.ELF`). Document where its value comes from per level (a level header field? a script op? a
   hard-coded level check?) in `docs/systems/player-physics.md` (create the section if it's missing), with addresses.
2. Produce the per-level value for every level on the disc (a table in that doc), from the data the game reads.
3. Port it: a `world_gravity: [f32; 3]` in the Sim's level data, set the way the game sets it; Bond's physics reads its y
   (keep the existing behaviour on every −9.8 level: the suite must stay green). Add
   `pub fn death_script(&self) -> u32` next to it returning 0x6000151 or 0x60008e0 by the rule above; don't edit
   `death.rs` (the main agent's death-anim branch will call your function).
4. Test (NF_ISO, `crates/nf-view/tests/fix_world_gravity_live.rs`): the table's values for 07000005 and for one level
   where the value differs (if any); `death_script()` follows it. Sister assertion: a deliberately wrong expected
   value fails.

Files: `crates/nf-game/src/player.rs`, the level-data struct you add it to,
`docs/systems/player-physics.md`, the new test.

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


**Branch:** `s/fix-world-gravity`.
