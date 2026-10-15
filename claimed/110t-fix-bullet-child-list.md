## Your task: `fix-bullet-child-list`, the searchlight bullet box's exact path (U-041)

U-041 (docs/UNKNOWNS.md; G-033 in docs/GAPS.md): the searchlights take bullet hits through a stand-in box in the port.
The game finds them through the bullet's collision child list. `Door_ReactToHit` has the same open "exact proxy-list
path" (G-035).

Do:
1. Find in the decompile how a player/drone bullet's ray reaches the searchlight object: the collide pass, the child/proxy
   list it walks (`Collide_*`, `Bullet_CollisionHandler` 0x12d068 area), and the box or geometry it tests. Document it in
   docs/systems/searchlight.md (U-041) with addresses.
2. Replace the port's stand-in box with the game's test, in `crates/nf-game/src/searchlight.rs` / the bullet collision
   caller.
3. Test (NF_ISO, a level with breakable searchlights — `docs/systems/searchlight.md` lists them): a shot that the game
   would register (aimed at the lamp housing) breaks the light; a shot just outside the game's volume doesn't, while the
   old stand-in box would have caught it. Show that sister failing on the old code first.

Files: `searchlight.rs`, the bullet collision caller, docs/systems/searchlight.md, docs/UNKNOWNS.md (U-041), the new test.

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


**Branch:** `s/fix-bullet-child-list`.

---
claimed-by: sandbox3 at 2026-10-15T13:04Z
