## Your task: `guard-weapons`, guards' weapons: damage, fire rate and accuracy per weapon and difficulty

The user reported guards "shooting me way too quickly after seeing me" (102d fixed the first-shot hold). Check the rest
of the guard fire model against the game for every weapon guards carry: the per-weapon damage (`weapon_data+0x0C`),
fire interval and burst, the accuracy model (the spread/aim error per difficulty, `difficulty` on `Data`), the distance
falloff, the armour absorption on Bond. Port what differs, citing the decompile (`DroneWeap_*`, `Drone_Fire*`,
`Player_Hurt`). Tests: for 3 weapons x 3 difficulties, the hit rate over 100 shots at 10 units and the damage per hit
equal the game's model (numbers); sister assertions.

Files: `crates/nf-game/src/{drone,damage}.rs`.

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

**Branch:** `s/guard-weapons`.
