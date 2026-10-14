# fix-front-end-chain — the front-end leg after a mission ends (campaign-flow blocker 7)

## What was done

The port landed on 0x07000048 after a mission end and stopped. This task ported the leg the
game runs there: the results/medal page, the unlock write into the progress layer, and the
mission select offering (and starting) the next mission. The campaign runner now drives it from
01's end to 05's start.

1. **The results page's fields** (`crates/nf-view/src/menu.rs`): new `FrontEnd::results_enter`
   — `P_NFRESULTS_Handler` 0x208e78 case 0x4c against the live progress layer:
   `PlrStat_GetScore(0)` (the commit call [M :77]), the medal compare of the last score
   (ScoringTable+0x34) against the thresholds (+0x4 bronze .. +0x10 platinum [M :98-142]), the
   target line = the threshold ABOVE the score paired with its own label (0x2cc platinum ..
   0x2cf bronze [M :104-139]), the medal name (0x2c7-0x2ca) or the composed no-medal line (Txt
   0x10001fd with the codename, else Txt 0x10001fe [M :144-154]), and the emblem texture
   (0x300013d/e/f, 0x300019a [M :109-139]). `SeparateNumber` 0x1dec78 ported for both numbers
   (comma grouping). `results_page` now draws the game's numbers (the old "—" stand-ins and the
   P_ENDMISSION restart rows are gone from the complete page — the page has no row list [L page
   0x40000036's controls]; the hint is its ~A Continue).
2. **The unlock write**: results_enter runs `Progress::set_level_bonus(BaseMap, medal, 0)` —
   `Menu_SetLevelBonus` 0x1ffa10 [M :169, outside the score_table arm] — then mirrors the packed
   `nightfire_status` (Menu_GetNightfireStatus 0x202360) into the wheel flags, sets
   `just_completed` / `new_reward` (DAT_002e1c2c), and saves the codename profile (the MSSN
   carries the 32-bit status [M LS_MakeMission 0x1cd720:49-50]). `mission_ended` (Mission_Update
   case 4) no longer unlocks or saves — the game's case 4 does neither [M 0x182c60:65-76].
3. **Two progress-layer bugs fixed** (`crates/nf-game/src/progress.rs`), both shown failing first:
   - `get_score`'s row source was inverted: the game picks by the hash's top byte — the 0x07
     maps build the rows from `PlrMissionStats`, the 0x09 maps copy `NewScoresRef` and abort
     NULL [M PlrStat_GetScore 0x1984b0:38-84]. The port aborted every ON-FOOT map, so no results
     page could ever show numbers.
   - `get_score`'s commit semantics were inverted: the game's param **0** is the commit
     (DoneBetter = best ≤ total, guarded by total ≠ 0, best read BEFORE the update; last/best
     update) [M 0x1984b0:301-315]; a read leaves the entry alone; the ms ≠ 6 arm zeroes and
     clears DoneBetter [M :316-333]. The port updated best on every call and computed
     done_better after the update (always true).
   - `set_level_bonus`'s medal loop now runs only for counters 1..4 (the `param_2 < 5` guard
     [M 0x1ffa10:70]) — a no-medal run (medal 5) awarded ALL FOUR medal bits before — and sets
     `new_reward` only when a bit was clear [M :72-79].
4. **The chain routing** (`menu.rs`): Confirm on the completed results page → the mission list
   0x4000001c with P_NFMAP's cursor rule (scan sp_level+0x10 from row 11 down — the list opens
   on the highest unlocked row [M 0x20b608 case 0x4c]). A NEW reward routes to the bonus page
   0x40000038 whose own Confirm lands on the same list [M 0x209670:50-53] — the port elides the
   stop and takes the landing (reported). The FAILED page keeps P_ENDMISSION's three arms
   unchanged [M 0x216618:19-27]. The host's F8/F9 demo hook (`lib.rs demo_results`) now runs
   results_enter too, so the rendered page has the fields.
5. **The campaign runner** (`crates/nf-view/tests/campaign_flow_live.rs`): new
   `front_end_chain_01_to_05` — plays 01-04 (the runner's level play, warm carries), and between
   missions drives the front end: mission_ended → the outro code rides the load request
   [M 0x182c60:89-116] → the loader's phase 0/2 wraps to the front-end map with the results page
   [M 0x1d6360:160-163 + 0x1d3560] → results_enter → the fields asserted against the scoring
   entry (score = +0x34, target = the next threshold, the label/emblem pairing) → the award
   asserted in the packed status → Confirm → the mission list on the highest unlocked row → the
   next mission's row confirmed (its start rides the request [M 0x20b8d8:21-24]). After 04: 05
   (07000011) is unlocked, its MSSN bit is in the codename profile file (the exact set
   0b1111_0111 asserted — row 3, a driving mission, correctly stays clear), and confirming its
   row requests 0x07000011 — 05's start.

## How it was verified (failing first)

- `the_completed_results_page_continues_to_the_mission_list` (front_end_chain_live.rs) failed
  pre-fix (`left: Main, right: MissionSelect` — shown in the transcript) and passes post-fix;
  its sister `the_failed_page_keeps_the_fail_menu_arms` passed pre- AND post-fix (the fail menu
  untouched).
- `the_on_foot_mission_rows_come_from_the_mission_stats` and
  `set_level_bonus_medal_none_awards_no_bits` (progress.rs units) failed pre-fix (the abort and
  "medal 5 awards no counter 1") and pass post-fix, with sisters (the 0x09 NULL abort kept; a
  real medal stamps every counter ≤ medal).
- `get_score_arithmetic` restructured to the corrected row sources (the 0x07 path through the
  mission tally; the 0x09 path through a NewScoresRef stand-in; the read/commit and ms≠6
  sisters). All 152 nf-game lib tests pass.
- `front_end_chain_01_to_05` runs the whole leg headless through the real ISO (16.5 s): every
  results page shows the game's numbers, every award lands, 05 unlocks in the profile and on the
  mission page. 0700000b's leg races the forced complete against its own self-loop exit trigger
  (campaign-flow blocker 4, recorded) — the chain accepts either end; the front-end leg is the
  same both ways.
- Updated to the corrected sites: menus_flow's unlock test (results_enter) and
  `the_results_pages_split_complete_from_failed` (complete → the mission list; failed keeps the
  three arms); fix_save_profile_live's `completing_a_mission_unlocks_and_saves` (the award via
  results_enter, the save after it); movie_playback_flow untouched and green. Totals on the
  merged tree: nf-game lib 152/152; menus_flow 34/34; movie_playback_flow 9/9;
  fix_save_profile_live 5/5; campaign_flow_live 4/4 (incl. the chain).

## Suite

The full suite (`tools/test/suite.py`) was started in the first minutes (19:47 UTC, master
baseline) and restarted on the final merged tree (89e153e9) at 21:26 UTC; the sandbox window
closed before the final-tree run could print its total — the per-crate totals above (the
touched crates' complete test sets, run on the final tree) are the verification of record.
Allowances on the slim ISO: the 3 `disc_golden` tests, and `pss_movies_live` without the retail
image.

## What's left

- The bonus page 0x40000038 (P_NFBONUS 0x209670's gallery) and the win page 0x40000053 are not
  ported: results Confirm with a NEW reward takes the bonus page's landing (the mission list)
  directly; just_completed refuses (reported). The routing evidence is cited in menu.rs.
- The pause screen's own GetScore read path (the game's param ≠ 0) is now faithful but has no
  dedicated test.
- 0700000b's self-loop/forced-complete race is campaign-flow blocker 4's territory (a live
  check is filed there).
- The full-suite total line on the final tree (the next session can re-run it; ~40 min).

## Files touched

- `crates/nf-game/src/progress.rs` — get_score's row source + commit semantics;
  set_level_bonus's medal guard + `new_reward`; the restructured unit tests.
- `crates/nf-view/src/menu.rs` — ResultsFields + results_enter + separate_number; the complete
  results routing (P_NFRESULTS 0x4b + P_NFMAP 0x4c cursor); results_page's field draw;
  mission_ended's unlock/save moved to results_enter; rows()/row_label split by outcome.
- `crates/nf-view/src/lib.rs` — the demo hook runs results_enter (shared file; the one hook).
- `crates/nf-view/tests/front_end_chain_live.rs` — new (the routing test + the fail-menu sister).
- `crates/nf-view/tests/campaign_flow_live.rs` — new chain test.
- `crates/nf-view/tests/menus_flow.rs`, `crates/nf-view/tests/fix_save_profile_live.rs` —
  updated to the corrected sites.

## Live checks for the main agent

PCSX2 (spot / action / expected):
1. Complete 01 (07000001, the Double Cross extraction): after the "Mission Complete" banner the
   outro movie 0x0730004 plays, then the front end opens on the RESULTS page with the score
   number, a "… Medal Target:" line with a number, and the medal name (or the no-medal line with
   the codename). Confirm → the mission list, opened on the highest unlocked mission.
2. The mission list after 01: "Night Shift" (07000009, wheel row 5) is offered (bright, not
   grey); row 3 (Enemies Vanquished, 09000006) still grey.
3. nf-view `--bond` render points for the results page: the page is menu-space (not world
   space) — render with the results page up (F8 on a loaded mission) at any bond position; the
   header "RESULTS", the two number lines at y≈336/360 and the medal plate at y≈264 should
   match the PCSX2 capture's placement (the CRT filter note applies: presence and placement,
   not colour).
