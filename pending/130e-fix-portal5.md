## Your task: `portal5`, the courtyard walls belong to the visible room: fix the port's wall attribution

Read `docs/reports/fix-portal4.md` on `s/fix-portal4` (02c0786) and then `docs/live/2026-10-13-portal-ground-truth.md` on
master (all three sections). Continue from `s/fix-portal4`: its chain builder (`build_chains` via `build_FindCel`) is
right and verified against the live 40-node chain.

**Your forced-objects hypothesis is checked and refuted [L]:** at the courtyard spot only 12 objects pass
`View_AddForcedObjects`' gates for the main view (mask 0x1), all small (r ≤ 4.9: the player's parts, two props, four doors);
no wall. And the game's VisCelList is {eye cel = your 33, 0x0179fb30 = your 21}. So in the game the courtyard wall
geometry is drawn through room 21's own glist or its chain. The port puts that geometry under its wall ROOM cells
20/22/38/13 (your census: World materials node 13 ×48, 20 ×64, 22 ×55, 38 ×96). That attribution is what differs.

**Do:**
1. Find how the game assigns a world mesh/glist to a cel: the static map data's cel placement → glist (`parsemap_block_map_data_static`
   0x1d1998, `hashtable_hashcode_to_celglist`, `parseentity_fixup_entity` 0x1d00c0), and how fix-render3's cel
   attribution (scene.rs) decides a World material's `node`. Find why the wall meshes land in rooms 20/22/38/13 in the
   port while the game draws them with only 33 + 21 visible.
2. Check it with live data if the decompile isn't enough: the snapshot `work/ee_start_05.bin` has the runtime cel
   records (glist pointers in the record; the census method from your report). Or write a precise PINE/gdbcall request
   in your report (address + what to read), and the main agent will run it.
3. Also settle the 9 extra chain nodes on cell 21 (your residue): the same per-cel collision-list question.
4. Pin it: at the courtyard spot the wall materials draw with the walk on, the visible rooms stay {33, 21}, and the
   other ground-truth spots don't change (07 window: room hidden; 07 hall identical). Show the test failing first.
5. Full four-crate suite (start it first, in the background) and the render spots.

Files: `crates/nf-view/src/{scene,gpu,lib}.rs`, `crates/nf-view/tests/fix_portal*_live.rs`.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/portal5` (start from `s/fix-portal4`, then merge `origin/master` into it).
