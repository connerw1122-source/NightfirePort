## Your task: `08-walls`, 07000008: a wall throws Bond onto the zip line; the big wall clips into the missile room

The user's playtest (results 1013-1758, 07000008, the safe/briefcase area around (155, 43, -113)): "I walked into a wall
and it threw me onto a zip line on the other side of the wall" and "the big wall is still clipping into the missile
room. This wasn't fixed at all." Reproduce both: (1) walk Bond into the walls around the safe room and the zip line's
grab volume (the wire objects on 08: `sim.wires`) and find which wall lets the wire grab through it (the game's grab
test likely needs line of sight or a facing check: find it); (2) find the wall mesh drawn inside the missile room (a
world instance or a forced object placed/transformed wrongly, or a cel drawn that the game hides: compare with
`docs/live/2026-10-13-portal-ground-truth.md`). Fix both with tests (no wire grab through a wall; the missile room's
visible-instance set excludes that wall); sister assertions. List `--bond` spots for the user to check.

Files: `crates/nf-game/src/wire.rs`, `crates/nf-view/src/scene.rs`, as needed.

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

**Branch:** `s/08-walls`.

---
claimed-by: sandbox6 at 2026-10-14T03:45Z

---
## Report (done, 2026-10-14T04:58Z)

