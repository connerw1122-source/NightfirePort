# nfstats-model2 — land held 614 (the one suite failure)

Branch `s/nfstats-model2` = `origin/s/nfstats-model` (dd301c6, the held 614) **merged with `origin/master`**
(bfdc7e25, which carries 111e's PlrMissionStats handoff). The held failure is fixed on the merged tree; everything
the merge touches is verified. One commit on top of the merge: the test fix (below).

## The held failure, and the verdict

`nf-view/tests/fix_scores_handoff_live.rs` — "row 5: the no-kill run's stat", left 100, right 0. Row 5 is the
ACCURACY row, and on the merged tree it read 100 where 614's edit of the test expected 0 for the no-shot truck
ride. The two parents met exactly there: master's `fix-scores-handoff` scorer had `rows[5].value = … else { 0 }`
(no shots → 0) while 614 had already changed the scorer to `else { rows[5].count }` — and left the live test's
`for i in [1, 2, 3, 5, 8] { value == 0 }` loop untouched.

**The game says 100 is correct** — the code stays, the test was the leftover:

- `PlrStat_GetScore` 0x1984b0 [M, the 0x07-map arm]: the accuracy value is stored FIRST as the row's own count —
  `*(undefined4 *)(rows + 0x6c) = *(undefined4 *)(rows + 100)` (row 5's value slot at 0x6c ← row 5's count slot at
  0x64, the accuracy par = 100 [L the HT_*_ST tables, docs/fields/player-stats.md "5 accuracy (count 100)"]) — and
  only `DAT_002bcc24 != 0` (PlrMissionStats+0x04 = shots fired [M docs/fields/player-stats.md:36]) rescales it to
  `DAT_002bcc28 (hits) × count / fired`. No shots → the value stays 100, and the generic raw formula gives the
  FULL target: 50000 × min(100,100)/100 = 50,000.
- `P_NFSTATS_Handler` 0x209938 [M :44-66] displays those table rows verbatim
  (`Menu_AddRow(label, row.value, row.count, (u32)row.raw)` per row) — a no-shot run shows "Accuracy 100%".
- The PCSX2 capture on 07000005 reads "Accuracy 100% / 100% / 50,000" [L docs/reports/nfstats-model.md:54].

So master's `else { 0 }` was the misport; 614 fixed the scorer (and its own unit test asserts value 100 / raw
50,000 [M crates/nf-game/src/progress.rs:1034-1035]) but missed this live test's loop.

## The fix (test-only)

`crates/nf-view/tests/fix_scores_handoff_live.rs`: row 5 left the zero-loop (now `[1, 2, 3, 8]`), with the
explicit game assertions in its place — `rows[5].value == 100` [M 0x1984b0:74-78], `rows[5].raw ==
expect_generic(count, value, target)` (the full target at 100/100), and `rows[5].raw == 50_000.0` (the capture's
number). The file-header's row list updated to match (rows 1/2/3/8 zero in value AND raw; row 4 zero-valued but
full-max within par; row 5 the par 100 + full target). No code change: 614's scorer is the game's.

**Before/after**: before, the merged tree failed `truck_ride_finish_produces_the_results_rows` at line 104
(left 100, right 0); after the fix the same binary passes: `test result: ok. 1 passed; 0 failed (16.22s)` —
the same run, same truck-ride walk, through the real input path.

## Also verified (the merge itself)

The menu.rs merge unioned 614's `Page::Stats`/`mission_stats` with master's CnName/ScreenAdjust/MP-pages/Credits
(both sides kept; `Page::Results` de-duplicated; WORKLOG union). Compiles clean; on the merged tree:
`mission_stats_flow` 3/3, `menus_flow` 36/36 (nf-view), `mission_stats_rows` 7/7 and the full nf-game crate suite
(38 binaries, `--test-threads=1`) all green.

## What I ran (the full-suite story)

The driver's background runner does not survive this sandbox (two `nohup` starts died with 0-byte output between
tool calls — same as the objective-hints window), so the suite ran as foreground runner chunks on the FINAL tree
(merge c0f48e99 + the test fix):

- batch 1 — nf-formats + nf-audio (12 binaries): **117 passed, 3 failed** — the failures are exactly the 3
  `disc_golden` golden-manifest tests the slim ISO permits.
- batch 2 — nf-view headless (61 binaries): **195 passed, 1 failed, 4 ignored** — the failure is
  `probe_pylon_render::probe_render_pylon_only` panicking at its `request_adapter` (`adapter: NotFound`) — a
  wgpu GPU probe; this sandbox has no GPU (environmental, tree-independent).
- batch 3 — the priority live set (campaign_flow, fix_mission, fix_objective_banner/messages, fix_party_mission,
  fix_pause_audio, fix_restart_health/target, fix_death_restart, menu_live, menu_woman_live — 11 binaries):
  **59 passed, 0 failed**.
- Live chunks L0-L3 (~170 binaries in alphabetical order): L0 **146 passed, 2 failed** (fix_0c_pylon_live ×2 —
  the same `adapter: NotFound` GPU class), 7 ignored; L1 (45, incl. a re-run of the timeout-lost first 20):
  71 passed / 0 failed on the re-run and 0 failures across every logged result; L2 **140 passed, 0 failed, 4
  ignored**; L3 **103 passed, 0 failed, 7 ignored**.
- Directly via cargo (also on the final tree): the full nf-game crate (green), `fix_scores_handoff_live` 1/1,
  `mission_stats_flow` 3/3, `menus_flow` 36/36, `mission_stats_rows` 7/7.

**Total**: ~252 of 348 test binaries, **1030 passed, 6 failed, 26 ignored** — the 6 failures are the 3 permitted
`disc_golden` environmental fails + 3 wgpu `adapter: NotFound` GPU-probe fails (probe_pylon_render,
fix_0c_pylon_live ×2), all tree-independent environmental ones.

**Not run** (~96 binaries, the alphabetical tail + the walk set): `walk_*_live`/`walkreal_*_live` (28 level-walk
binaries), the render/weapon/world probes (`render2_live`, `render3_live`, `weapon_*`, `world_*`, `sky_*`,
`sound_coverage_live`, `textures_live`, …) and the small headless tools (`diff_calls`, `rng_live`,
`searchlight_live`, …) — the window ended (background execution unavailable, ~9 min/foreground chunk). None of
them touch the scores/progress path this task lands; the next window should run the full runner.

## What's left

- Nothing on the task itself: the held failure is fixed with game evidence and the merged tree is verified to the
  extent the window allowed. The main agent may want a PCSX2 spot-check that the Mission Stats page's Accuracy
  row reads 100% after a no-shot run (fieldprobe on the results page) — expected per [M 0x1984b0].
- The full runner on a future tree to cover the ~96 not-run binaries.

## ntfy failed

All three fallbacks hit ntfy.sh's daily quota (HTTP 429), 22:40 UTC 2026-10-15:
- curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
- python urllib: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
- wget: no output (the server error page suppressed by -q; the same 429)
