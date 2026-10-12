Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-portal`, fix the portal visibility walk, then turn it back on

The portal walk (`fix-perf-level`, `scene::portal_visible_cels`) is now opt-in (`NF_PORTAL=1`). On 07000006, walking between the truck and the courtyard, it hid the entire level except characters and doors (user screenshot: an empty night sky over water).

1. Reproduce it headless: walk Bond along that route on 07000006, and log the eye cel, the walk's visited cels and the visible draw count each tick. Find where visibility collapses.
2. Likely causes:
   - the eye-cel choice (the smallest-volume tie-break versus the game's `build_FindCel` raycast);
   - outdoor cels with no portals;
   - the depth-0 plane convention;
   - the 0x40 VisCelCount cap;
   - doors treated as open but the portal record links wrong.
   Port the game's rule for each.
3. Add a test that walks the 07000005, 07000006 and 07000007 routes and asserts the visible world draw count never drops below what the frustum test alone allows, minus occluded rooms. That is, it must never be near zero while the frustum sees geometry.
4. Then make it the default again, with `NF_NOPORTAL` to disable.

Files: `crates/nf-view/src/{scene,gpu}.rs`, `crates/nf-formats/src/level.rs`.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**User playtest (2026-10-11), a second symptom of the portal walk being off:** on 07000007 at `--bond 61.11,44.12,-52.58,-2.541,-0.010` (looking out of a ballroom window) a lit room box, another room's interior, floats in the snow outside. The main agent rendered it headless: with `NF_PORTAL=1` the box disappears (correct), but **two partygoers inside that hidden room are still drawn, standing in the snow**. The walk culls only World draws; the game also skips characters and objects whose cel isn't visible (`View_DrawObjList` 0x1e69a0 walks the visible cels' object lists). Port that too, and add both spots (this one and the 07000006 one that went black) as headless tests: count the draws of the hidden room and of its characters with the walk on, with a sister assertion that they're drawn with it off. Debug aid: `NF_SKIP_MAT=i,j` skips material indices in the renderer, and `crates/nf-view/tests/probe_render4.rs::probe_draws_near_a_point` lists every draw near `NF_PROBE_AT=x,y,z`.

**Branch:** `s/fix-portal`.
