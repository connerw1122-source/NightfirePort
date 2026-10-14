## Your task: `menu-backdrop`, the menus' background movie never decodes

The front-end pages' background is an aux FMV (fix-menus-real/-real2: the main page's orange lens panel, Bond in the
tuxedo and the 007 ring are INSIDE movie 07330048; the list pages use 07350048). The main agent can't get a frame out
of it on the user's retail ISO (both PSS files are on the disc, 30 MB each):
`NF_ISO=<iso> nf-view <iso> 07000048 --page main --frame 60 --movie-frame <2|6|12> --shot x.png` with the `movies`
feature prints "[front end] movie 07330048: no frame N in the first chunk — the dark plate stands in" for every N, so
every page draws the dark plate. (Also: the headless still path reads `NF_ISO`, not the ISO argument: fix that.)
The user's verdict on the menus ("so far from finished") likely includes this: the windowed loop may be failing the
same way. Find why no frame decodes from the first chunk (the PSS demux: the video stream id, the pack layout, the first
GOP's position; compare with the briefing movies 0710xxxx that do play) and fix both the still and the live loop. Test:
a decoded frame of 07330048 and 07350048 (dimensions 512x448, a pixel hash at frame 12, numbers only); the windowed
loop's player starts and advances on the main page. Render `--page main` and describe it against PCSX2 (the lens panel,
Bond, the ring visible).

Files: `crates/nf-view/src/{movie,lib}.rs`.

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

**Branch:** `s/menu-backdrop`.

---
claimed-by: sandbox5 at 2026-10-14T10:30Z

---
## Report (done, 2026-10-14T12:22Z)

