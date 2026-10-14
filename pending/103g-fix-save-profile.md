## Your task: `save-profile`, codename profiles and progress saved to disk

The front end has "New Codename" / "Play without saving" / profile rows (101k). Port the game's profile save: what the
memory-card save holds (unlocked missions, difficulty per mission, high scores / medals (the Dossier's Records), Bond
moves, options, the controller style), its layout from the game's save code (find it: the profile/save functions and
the memory-card block), and store it as a file next to the port's config (one file per codename). Creating a codename,
saving after a mission, loading on the codename page, and the Dossier Records reading it. Tests: a round trip; the
unlock after completing a mission; sister assertions. No game bytes in the repo.

Files: a new `crates/nf-game/src/profile.rs`, `crates/nf-view/src/menu.rs` (the hooks).

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

**Branch:** `s/save-profile`.
