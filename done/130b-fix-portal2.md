## Your task: `portal2`, land fix-portal without the wall holes

Read `docs/reports/fix-portal.md` on `s/fix-portal` (a894d7c) first. Its eye-cel rule (`Camera_CheckLocation`
0x1bae90), the derived runtime cel flags (0x804c023 / 0x804c047), the depth-0 near plane and the 0x40 cap are good
work, and its pinned counts (07000007 start view = 177 draws) are a strong cross-check. The main agent merged it,
then rendered with the walk on and with `NF_NOPORTAL=1`, and did NOT push it:

- **07000006, `--bond 7.24,42.03,-39.31,10.161,0.428 --frame 120`** (looking up at the castle wall and the walkway
  from the courtyard, the user's 004-07000006 spot): with the walk ON, the masonry under the left walkway and the
  lower part of the right castle wall are MISSING, and open sky shows through where the walls stand. With the walk
  OFF they draw. Holes the player can see through are not a cull the game makes: something visible is in a cel the
  walk rejects. Candidates: the 200-unit depth-0 near-plane clip applied to portals the eye looks UP through (the
  plane is built from the view direction, so a steep pitch changes what it keeps); an outdoor/sky cel graph where
  the game's inner-node frustum-only path (`View_AddCels`) should be drawing these; the bbox-face stand-in for
  `build_FindCel`'s raycast picking the wrong eye cel in the courtyard.
- **07000007, `--bond 61.11,44.12,-52.58,-2.541,-0.010 --frame 120`** (the ballroom window): the lit building seen
  THROUGH the window is hidden with the walk on. Your report says that room is walk-hidden in the game too. Keep
  it only if you can show it from the game (a PCSX2 shot request is fine), because a building visible through
  a window disappearing reads as a bug.
- Spots where on and off render identically (good): 07000006 `--bond 9.87,43.04,9.84,3.177,0`, 07000007
  `--bond 34.25,44.13,-70.71,1.549,-0.070`.

Reproduce the 07000006 hole headlessly first: a test at that spot that lists the World cels the frustum keeps and the
walk drops, and asserts the walls' cels stay visible, failing on today's branch. Then find the game's rule that keeps
them. Bring the branch up to `origin/master` and run the full suite. In your report, list render spots for the main
agent, each with what should and shouldn't change.

Files: as fix-portal (`crates/nf-view/src/{gpu,lib,scene}.rs`, `crates/nf-view/tests/{fix_portal_live,gpu_skin_live}.rs`).

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/portal2` (start from `s/fix-portal`, then merge `origin/master` into it).

---
claimed-by: sandbox3 at 2026-10-13T13:01Z

---
## Report (done, 2026-10-13T14:37Z)

