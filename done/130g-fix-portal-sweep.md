## Your task: `portal-sweep`, portal culling must never hide what the player can see, on every level

Portal culling is on in master (the fix-portal → fix-portal6 chain, verified against PCSX2 at four spots on 06/07:
docs/live/2026-10-13-portal-ground-truth.md). The user's playtest found a bad spot on **07000005**:
`--bond 16.67,30.04,58.60,1.755,-0.001` (a snow trench between stone walls, near grapple point #282). With the walk ON,
the trench walls around Bond are GONE: he sees straight through them to the distant cliff and a cut-off castle. With
`NF_NOPORTAL=1` the trench walls and the red/yellow wall-hug marker draw. The near room itself is being culled. The
playtest launcher now sets `NF_NOPORTAL=1` until this is fixed.

1. **Reproduce at that spot** headlessly: list the eye cel, the visited rooms, the chain nodes and the World draw counts
   on vs off; find which cels hold the trench walls and why the walk drops them (the eye-cel choice in an outdoor area,
   the room class, the chain linker, `Vision_InView`). Fix it from the decompile.
2. **A sweep, so this doesn't come back:** a test that, for every on-foot level, places the eye at each start point
   and at a grid of points along the level's nav nodes (a few hundred spots in total), looking in 4 directions, and
   asserts that every World cel the frustum alone keeps AND that is within ~30 units of the eye in clear line of sight
   (a ray against the collision world) is kept by the walk. Near, unoccluded geometry must never be culled. Report the
   failing spots per level, fix the causes, and pin the sweep (counts only).
3. Keep the ground-truth spots unchanged (06 courtyard walls solid, 07 window room hidden, 07 clock hall props, 06 start).
4. If a level can't be made correct in this session, add an explicit per-level fallback (the frustum-only path)
   with the failing reason documented, so no level ever renders with holes.

Files: `crates/nf-view/src/{scene,gpu}.rs`, `crates/nf-view/tests/fix_portal*_live.rs`, a new `crates/nf-view/tests/portal_sweep_live.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/portal-sweep`.

---
claimed-by: sandbox4 at 2026-10-13T23:55Z

---
## Report (done, 2026-10-14T01:08Z)

Closed by the driver: the report is on `s/fix-portal-sweep` (docs/reports/fix-portal-sweep.md); the worker didn't run finish.sh.
