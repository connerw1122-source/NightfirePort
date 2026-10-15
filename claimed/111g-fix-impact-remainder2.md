## Your task: `fix-impact-remainder2`, land 110q (held: 2 failures pinned to live measurements)

110q (`s/fix-impact-remainder`, the penetration roll + the impact emitter's RNG) was HELD on 2026-10-15. On master + 110q
the main agent's full suite gave **1333 passed, 2 failed** (master alone is green):
- `nf-game/tests/ai_live.rs::guard_takes_cover_like_the_game` — `UnderCoverAim lasted 90 (live 37)`: pinned to a PCSX2
  measurement, which master matches.
- `nf-view/tests/walkreal_07000001_live.rs::walk_from_the_spawn_to_the_exit_with_the_contact` — the contact never walks
  his route.
(110o, the gas-puff RNG, broke the same two. Its follow-up is 111d.)

Your branch's report already noticed the ai_live failure and called it pre-existing. The main agent's runs show master
GREEN on it (several full suites today), so it comes from the branch's RNG consumption.

Do: verify WHEN in the frame the new draws happen against the game's order, with a per-tick RNG call log (caller tag +
draw count) through a wall-hit tick, compared with the decompile's order (Bullet_CollisionHandler 0x12ce80 runs inside
the bullet object's update; `Emitter_Update` 0x17b080 runs as its own object later in the object list, on the tick
AFTER creation). The guard's cover roll and the shot's roll must interleave as the game interleaves them. Fix the
order, then show both tests green and the FULL suite green. Report the total line.

Files: the branch's files plus whatever the root cause needs.

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


**Branch:** `s/fix-impact-remainder2`.

---
claimed-by: sandbox5 at 2026-10-15T15:38Z
