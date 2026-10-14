## Your task: `fix-fire-modes3b`, fire modes: the weapon-sound sweep fails on the merge

Base: `s/fix-fire-modes3` (merge origin/master into it). HELD at merge: the full suite on master + this branch passed
1087 and failed ONE test that passes on master:
`fix_weapon_sounds_live::every_weapon_on_07000005_draws_fires_and_reloads_with_sounds` — "0x37: none of its missing
sounds [1112] ever played; 0x59: none of its missing sounds [1096, 1112] ever played". The test gives each weapon in
turn, fires 25 ticks, reloads, and checks the level-bank-missing sound ids start from their owning bank. Find which of
the branch's four changes (the 0x100000/0x200000 blast gate, the 0x6C&0x200 quarter-speed spawn, the other two) moves
these: a slower projectile whose impact/explosion sound now lands after the 25-tick window, a blast that now consumes
the projectile earlier, or state left over from an earlier weapon in the sweep. Decide from the game's code whether
the new behaviour is right; if it is, extend the test's window per weapon citing the game's flight time (do not drop
the ids); if not, fix the code. Run `tools/test/suite.py` on the final tree and give the total.

Files: `crates/nf-game/src/drone.rs`, `crates/nf-view/src/scene.rs` (small), `crates/nf-view/tests/fix_weapon_sounds_live.rs` as justified.

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

**Branch:** `s/fix-fire-modes3b`.

---
claimed-by: sandbox2 at 2026-10-14T16:35Z

---
## Report (done, 2026-10-14T19:30Z)

