## Your task: `fix-blast-scripts2`, blast scripts + cut-scene actors: one 0x060007c4 path

Base: `s/fix-blast-scripts` — merge origin/master FIRST (it now has fix-fire-modes3's Samurai blast gates; if
`s/fix-samurai-fire` isn't on master yet, merge it too), then merge `s/fix-cutscene-actors` (291). HELD at merge:
1. On master + this branch, `fix_blast_scripts_live::the_samurai_bolt_plays_the_cyan_blast_script` fails: "exactly
   one 0x060007c4 one-shot player (the bolt's cyan blast) — left 0, right 1". Master's Samurai fire path (fire-modes3 /
   samurai-fire: the 0x100000 world + flesh arms, the quarter-speed and swoosh) and this branch's per-source script
   carry each need to agree on where the bolt's blast is created and which script it plays.
2. 291-fix-cutscene-actors separately added the 0x060007c4 script to the scene build sweep so its entities can draw
   — the same effect from the other side.
Make ONE path: the bolt's contact (world and flesh) creates the explosion with script 0x060007c4 and its entities
draw. Keep both branches' tests (fix_blast_scripts_live, fix_cutscene_actors_live) and fix_samurai_fire_live green.
Render a Samurai Overcharge bolt hitting a wall on 07000005 before/after. Run the full `tools/test/suite.py`.

Files: `crates/nf-game/src/{drone,explode,props}.rs`, `crates/nf-view/src/scene.rs` (small).

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

**Branch:** `s/fix-blast-scripts2`.

---
claimed-by: sandbox6 at 2026-10-14T22:33Z
