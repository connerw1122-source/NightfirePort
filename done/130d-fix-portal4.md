## Your task: `portal4`, match the game's visible set at the 06 courtyard from live data

Read, in this order: `docs/reports/fix-portal3.md` (on `s/fix-portal3`, 377873e), then
`docs/live/2026-10-13-portal-ground-truth.md` on master. The main agent captured the real game at the exact spot the
port gets wrong (`07000006 --bond 7.24,42.03,-39.31,10.161,0.428`):

1. **A screenshot:** the real game draws the masonry under the left walkway and the lower right castle wall solid. The port
   with fix-portal3 still hides wall cells 20/22/38/5/13/23 there.
2. **The game's own visible list (GDB):** `Vision_Portal_Recurse` runs 3× per frame (three viewers). For the main
   view, **VisCelCount = 2**: the eye cel (sphere (4.5, 43.5, −42.1) r 11.4: your cell 33, so your eye-cel fix is
   confirmed) and a big room **0x0179fb30** (sphere (−13.0, 49.0, −37.0) r 44.0) at depth 1.
3. **That big room's `+0x18` chain, all 40 nodes with class and world sphere** (in the doc). The eye cel's
   chain is empty.

So with only 2 rooms visible, the game draws those walls from the big room's glist or its 40-node chain. Your
"the game holes too at this placed camera" hypothesis is refuted by (1). Find the mismatch:
- Identify the port's cell for 0x0179fb30 (match by world sphere: `CelNode.sphere`, the placement-transformed bbox) and
  check the port's walk reaches it from cell 33 at this camera. The game's depth-1 entry says one portal hop.
- Compare the port's `CelGraph::chains` for that cell against the 40 live nodes (match by sphere). This settles the [S]
  0x40-cap / per-map-block chain-builder model: make the port's chain for that room equal the live one, and derive
  the rule that generalises (pick a second room on 05 or 07 and check it too, using the same gdbcall/PINE method:
  the commands are in the doc).
- Then check which of the port's wall cells 20/22/38/5/13/23 are the game's chain nodes. If a wall is not a chain node
  of a visible room and not a visible room, find which list the game draws it from (View_AddCels 0x1e6ba0's other
  branches, the forced/object lists) before changing anything.
- Pin it with a test: at this spot the port's visible room set is {cell 33, the 0x0179fb30 cell} and its chain
  equals the 40 live spheres (numbers in the test, not game bytes), and the wall cells draw. The other spots in the
  ground-truth doc must not change (07 window: the room stays hidden; 07 hall: identical).

Run the full four-crate suite (start it in the background first thing; fix-portal3's run didn't finish) and list
render spots for the main agent.

Files: `crates/nf-view/src/{scene,gpu,lib}.rs`, `crates/nf-view/tests/fix_portal*_live.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/portal4` (start from `s/fix-portal3`, then merge `origin/master` into it).

---
claimed-by: sandbox5 at 2026-10-13T16:36Z

---
## Report (done, 2026-10-13T18:18Z)

