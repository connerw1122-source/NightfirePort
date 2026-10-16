# Report: unlock-popups2 (branch `s/unlock-popups2`)

Task 619b — land the held 619 (`s/unlock-popups`, the post-results bonus / cheat-medal / win
pages) on top of master. The hold was one suite failure:
`nf-view/tests/campaign_flow_live.rs:751 front_end_chain_01_to_05` — expected `Page::MissionSelect`
after the results page's Confirm, got `Page::NfBonus`.

## What's done

1. **The merge.** `origin/master` merged into `origin/s/unlock-popups` (branch `s/unlock-popups2`,
   pushed with a WORKLOG start entry first). `crates/nf-view/src/menu.rs` had four conflict
   blocks — all "both sides" cases: 619's fields (`results_level`, `bonus`, `win_movie`,
   `cheat_medal_sel`, `medal_from_cheat`, `win_movie_vol`), its four methods
   (`bonus_enter`/`win_enter`/`win_movie_finished`/`cheat_medal_confirm`) and its wheel-row arms
   (`NfBonus`/`WinGame`/`CheatMedal`) now sit beside master's codename-entry, nfstats-model,
   secret-codes and screen-adjust fields/arms. `secret: SecretEntry` was on both sides (kept
   once). Two slips in my first resolution were caught by the compiler and fixed as their own
   commit: `stats_enter` was nested inside `cheat_medal_confirm` (unclosed delimiter) and the
   master-side constructor initializers were missing (E0063).

2. **The decision, from the game** (the driver's question: does P_NFRESULTS → P_NFBONUS appear
   after mission 01's completion with that test's stats, and what gates it?):
   - The gate is `DAT_002e1c2c`: Menu_SetLevelBonus 0x1ffa10 clears it at entry and its tier loop
     (medal 4..1, `PlrStarts_ProcessRewardCounter`) sets it on the first tier that was **not
     already standing** [M 0x1ffa10:52, 71-82].
   - P_NFRESULTS's case 0x4b routes on it: new reward → the 007 Bonus page 0x40000038
     [M 0x208e78:48-53]; only with no new reward does Confirm go straight to the mission list
     0x4000001c [M :39-44]; the final mission without a new reward goes to the win page
     0x40000053 [M :46].
   - P_NFBONUS's own Confirm (0x4b) lands on 0x4000001c while `!just_completed` [M 0x209670:50-53].
   - A probe of the failing leg: the chain test completes mission 01 (0x07000001) through the live
     mission layer and the results page scores **24150000 → medal 4** (platinum) on a **fresh
     profile** — so tiers 1-4 are all newly granted and the game **does** show P_NFBONUS. The
     port's `new_reward` (set by `results_enter` → `progress.set_level_bonus` →
     `progress.new_reward`) mirrors exactly this gate, and the port's Confirm route
     (`new_reward → NfBonus`, `just_completed → WinGame`, else the mission list) matches the game's
     case 0x4b arm for arm.
   - **Verdict: the code is right; the test was wrong.** It must Confirm through the bonus page
     when the game shows it — exactly the driver's suggested reading.

3. **The fix (test only).** `campaign_flow_live.rs`'s chain leg now: Confirm → if the page is
   `NfBonus`, assert `fe.new_reward` (the hop only happens on a new reward [M 0x1ffa10:78]), run
   the host's `bonus_enter(&progress, fe.results_level, fe.new_reward)` (the game builds the
   page's fields on the 0x4c enter [M 0x209670:16-43]; the port splits it out of the action
   handler, which owns no Progress), assert the fields built, Confirm again → the mission list.
   The legs that earn no medal still go straight to MissionSelect (the `if` mirrors the game's
   gate). Before/after: the test failed at :751 (`got Page::NfBonus`) on the un-merged hold; on
   the merged tree it failed identically (reproduced); after the fix the whole binary passes
   5/5. `front_end_chain_live.rs`'s single-Confirm test is untouched — its `mission_ended` path
   never runs `results_enter`, so `new_reward` is false there and the game goes straight to the
   mission list (consistent with the hold's 1540/1 record).

## How it was verified

The full suite on the final merged tree (merge 7f715272 + the menu fix b1f7b487 + the test fix
eea62bcc). The background `tools/test/suite.py` starts kept dying in this sandbox (the cargo-lock
race plus the known detached-process reaping), so the coverage came from foreground slices plus a
direct execution sweep of the built binaries:

- **nf-game: 41 binaries, 375 passed / 0 failed** (`cargo test --offline --release -j2 -p
  nf-game -- --test-threads=1`) — full.
- **nf-formats + nf-audio: 105 passed / 3 failed** — the 3 are exactly the `disc_golden` tests
  the task allows to fail on the slim ISO.
- **nf-view: all 304 test binaries executed** (cargo's first 77 + a direct `-P 2` sweep of the
  rest, each binary `--test-threads=1`). The only real failure is the known no-GPU
  `fix_0c_pylon_live` pair (2 tests — the same environmental pair 507/drive-ptp/619 recorded).
  Four sweep artifacts were re-verified serially and pass: `campaign_flow_live` 5/5 (99 s),
  `fix_guard_weapons_live` 3/3 (164 s), `fix_party_mission_live` 15/15 (74 s),
  `fix_death_restart_live` 8/8 (13 s) — their sweep failures were the 90 s per-binary cap (rc 124)
  or the parallel kill (rc 137), not real results.
- Net: the hold's "1540 passed, 1 failed" is now **0 real failures** on this tree (the recorded
  environmental set — 2 pylon tests + 3 disc_golden — still fails headless/no-GPU, as allowed).

## What's left

- The pylon pair needs the main agent's GPU render check, as already recorded by 507/drive-ptp —
  not this task's change.
- The port's P_NFBONUS page-enter build is host-driven (`bonus_enter` after the page switch —
  the action handler owns no Progress). A host integration may want to call it automatically when
  the page changes to NfBonus; the game does it on the page-enter message [M 0x209670:16-43].
- 619's own carry-overs (docs/systems/unlock-popups.md's open questions) are unchanged.

## Live checks for the main agent

- **L-9 — the results hop**: in a paris_mis01 run that earns any medal on a fresh save, break on
  P_NFRESULTS 0x208e78's case 0x4b; expect `DAT_002e1c2c != 0` and the Manager msg 0x44 →
  0x40000038 (not 0x4000001c) [M expectation]. Then break on P_NFBONUS 0x209670's 0x4b arm;
  expect the next msg 0x44 → 0x4000001c with arg 1 [M expectation].
- **L-10 — the tier loop's gate**: poke `sp_level[row*0x18+0x30+tier]`-equivalent bits (or use a
  save that already holds the medal) before completing the same mission again; expect
  DAT_002e1c2c to stay 0 this time and Confirm to skip the bonus page [M expectation —
  0x1ffa10:75-79's "already standing" arm].

## Shared files touched

- `crates/nf-view/src/menu.rs` — the merge resolution only (no new code of mine).
- `crates/nf-view/tests/campaign_flow_live.rs` — the chain leg's Confirm through the bonus page.
- `docs/WORKLOG.md` — appended my entries only; the merge conflict resolved by keeping both sides.

## ntfy failed

(recorded below only if all three fallbacks fail — see the final section.)
