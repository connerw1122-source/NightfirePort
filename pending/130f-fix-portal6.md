## Your task: `portal6`, keep the courtyard fix, bring back the 07 clock hall's props

Read `docs/reports/fix-portal5.md` on `s/fix-portal5` (391d466). Its two fixes are real: the own-cell key misalignment
(379/765 non-room instances keyed under an earlier placement's cell) and the game's load-time chain linker
(`build_link_objects_to_rooms` 0x1c9150). The main agent merged it and rendered the four ground-truth spots
(`nf-view <iso> <level> --bond … --frame 120 --shot`, walk on vs `NF_NOPORTAL=1`):

- **07000006 courtyard `7.24,42.03,-39.31,10.161,0.428`: FIXED.** The walls under the walkway and the castle wall draw
  solid, matching the PCSX2 capture.
- **07000007 window `61.11,44.12,-52.58,-2.541,-0.010`:** the room stays hidden, matching the game. Good.
- **07000006 `9.87,43.04,9.84,3.177,0`:** fine (barrels now visible at the bottom; check they're in the game's view too).
- **07000007 clock hall `34.25,44.13,-70.71,1.549,-0.070`: REGRESSION.** With the walk ON, the grandfather clock at the
  end of the hall and BOTH wall shields with crossed swords are missing. With `NF_NOPORTAL=1` they draw, and in the real
  game (PCSX2 capture, docs/live ground-truth table) the clock, the shields and the two guards are all there. Before
  portal5 this spot rendered identically on/off.

So some generic props in visible rooms are no longer reached: they're probably hung on a room the walk doesn't visit
by the new linker rule, or dropped by the own-cell fix's filters. Find which cells the clock and the shields' objects
key under now vs before, which rooms the walk visits at that spot, and the game's own owner for them. Use
`ee_start_05.bin`'s method for the 07 level if a snapshot exists, or write a precise PINE request: the main agent can
read the game's `cel+0x18` chains at that spot as it did for the 06 courtyard (docs/live). Fix it without losing
the courtyard. Pin all four spots in tests: the courtyard walls draw, the window room hidden, and the hall's clock
and shields drawn.

Run the full four-crate suite (in the background, first thing) and report the count.

Files: `crates/nf-view/src/{scene,gpu,lib}.rs`, `crates/nf-view/tests/fix_portal*_live.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/portal6` (start from `s/fix-portal5`, then merge `origin/master` into it).
