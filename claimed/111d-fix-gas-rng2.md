## Your task: `fix-gas-rng2`, land 110o (held: 4 suite failures)

110o (`s/fix-gas-rng`, Gas_Init's 6 RNG draws per puff) was HELD on 2026-10-15. On master + 110o the main agent's full
suite gave **1333 passed, 4 failed** (master alone is green):
- `nf-game/tests/ai_live.rs::guard_takes_cover_like_the_game` — `UnderCoverAim lasted 90 (live 37)` (line 249). Master
  matches the PCSX2 measurement.
- `nf-view/tests/walkreal_07000001_live.rs::walk_from_the_spawn_to_the_exit_with_the_contact` — `the contact must walk
  his route to the LoadPointActivator plate`.
- `nf-view/tests/models_misc_live.rs::firing_spawns_a_muzzle_smoke_puff_that_fades_and_dies` — `the puff outlived its
  life` (line 325). The drawn lifetime must be the field the game sets from its draw, and the puff must die on time.
- `nf-view/tests/fix_guard_weapons_live.rs::the_fire_rate_matches_the_game_interval_clamp_and_burst_cut` — `no candidate
  guard with weapon 0x12 completed the collection` (line 156).

The first, second and fourth are RNG-order consequences: guard rolls moved. The draws themselves may be right while
their POSITION in the frame is wrong (the game creates the puff at a specific point of the object update; the port
might draw them earlier or from the wrong stream). Verify the draw position against the game's order with a
side-by-side RNG call log (`diff_calls`-style: record the sim's `Rng` call sites per tick around a muzzle puff and
compare with the decompile's order). The `ai_live` cover test is pinned to a live measurement, so the fixed port must
match it again. Fix the life bug too. The full suite must be green; report the total line.

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


**Branch:** `s/fix-gas-rng2`.

---
claimed-by: sandbox7 at 2026-10-15T15:13Z
