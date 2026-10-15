# nfstats-model — the Mission Stats page's numbers

Branch `s/nfstats-model`. The task: port the Mission Stats page's computation
(`P_NFSTATS_Handler` 0x209938) — where each Current comes from, each Target, each TOTAL, the
formatting, the page flow — and expose a `stats_rows()` getter returning the rows as display
strings + ids. Docs first (`docs/systems/mission-stats.md`), tests, no drawing.

## What's done

**The data model — `crates/nf-game/src/stats.rs`** (new module, `pub mod stats;`):
- `separate_number(u32)` — `SeparateNumber` 0x1dec78's comma grouping (`%d` / `%d,%.3d` /
  `%d,%.3d,%.3d`, the millions group unpadded) [M the format strings 0x30d090/0x30d098].
- `timer_string(cs)` — `Timer_Seconds2String` 0x1deaa8 mode 0: round-half-up seconds, `mm:ss`;
  the hour branch [S its sprintf args are dropped in the decompile]; negative → zeros [M].
- `StatsRow { id, category, current, target, total, indented }` and `stats_rows(&mut Progress,
  base_map, ms, difficulty, Option<&Lang>) -> Option<Vec<StatsRow>>` — mirrors the handler
  0x209938 exactly: the header quartet exposed as `STATS_HEADER` (0x1000204/0x10001e1/0x10001e2/
  0x163 [M :44-56; the pause SCORE tab binds the same four, 0x204408:447-498]), the label-only
  "Opponents" row 0x15a and the blank spacer 0x30d2d8 [M :66-68/:117], the stealth trio
  (0x15c/0x16e/0x15d) gated on the entry's +0x38 flag [M :68], pct currents
  `fptoui(ratio·100+0.5)` [M :92/100/106], Time's timer cells (elapsed 1/100 s, par × 100 [M
  :103-108]), the tail Subtotal/Difficulty Bonus/TOTAL through `SeparateNumber` alone [M
  :117-123]. `None` = the game's `score_table == 0` arm [M :38-40].

**The page flow — `crates/nf-view/src/menu.rs`**: `PAGE_STATS = 0x40000037` [M
`Handler_HandleMessage` 0x2029e0:448], `Page::Stats`, `FrontEnd::mission_stats` +
`stats_enter(...)`; the results page's side action opens stats [M `P_NFRESULTS_Handler`
0x208e78 case 0x5e → `Manager_SendMessage(0x44, 0x40000037, 0)`] and its back goes to the
Dossier [M case 0x5d → 0x4000002b]; the stats page idles every key but back (the handler arms
only the 0x4c build [M 0x209938:31-124]); △ returns to the results page [S — the game plays the
page's own menu-def script (`Page_SendMessage` 0x21ec68 case 0x5d → `Script_PlayDefault`);
the target lives in the unparsed front-end archive]. The draw arm renders the DATA rows only —
the page art is the main agent's.

**Three scorer misports fixed in `Progress::get_score`** (each caught by a failing test; the
PCSX2 capture is the disproof):
1. **The time row under-par capped at max** — the game computes
   `ceil₁ₑ₋₅(max × par / max(elapsed_s, 1))` in BOTH branches [M 0x1984b0:118-170: the under-par
   arm multiplies max × par and divides by max(elapsed, 1); the over-par arm divides by
   elapsed/par]. 01:19 / 09:00 → 170,887 = ceil(25000 × 540 / 79). The port showed 25,000.
2. **Row 4 stealth within par scored 0** — the port's `4 if count < value` guard fell through
   to the proportional arm; the game awards the FULL max with ratio 1.0 when detections ≤ par
   [M 0x1984b0:225-233]. The capture's 100% / 25,000 on a clean run.
3. **The accuracy value started at 0 with no shots** — the game copies the par into the value
   first and only a shot tally rescales it [M 0x1984b0:74-78]: no shots → 100% full score.
Plus the missing `ceil₁ₑ₋₅` on the generic/stealth raws [M :181-185/:213-218 — `(int)f` then the
1e-5 bump] and row 7's ratio from raw/max on the game's 2-decimal grid [M :234-253].

## How it was verified

- **`crates/nf-game/tests/mission_stats_rows.rs`** (7 tests): a known mission end driven through
  the real `log_*` path asserts every cell (ids, strings, indents); the PCSX2 capture on
  07000005 (Time 01:19 / 09:00 / 170,887; Stealth 100% / ---- / 25,000; Accuracy 100% / 100% /
  50,000; Health 100% / 100% / 50,000; Subtotal = TOTAL 295,887 at the ×0.0 difficulty) as a
  numbers-only golden; the status ≠ 6 zeroing gate as the fail-first sister (a dropped gate dies);
  the 0x09 stealth-row hiding; the unknown-hash None.
- **`crates/nf-view/tests/mission_stats_flow.rs`** (3 tests): side action opens, other keys
  idle, △ returns, `stats_enter` fills 14 rows ending "295,887", the draw takes them; the
  results Back → Dossier arm.
- The old tests that had locked in the misports were updated to the game's formulas (each
  updated assert cites the decompile line): `progress.rs` lib tests ×4 (get_score_arithmetic,
  the_on_foot_mission_rows, both finish_level handoffs), `progress_remainder_live` (the driving
  row-7 ratio → 0.67 on the 2-decimal grid), `fix_scores_handoff_live` (row 4 full, row 7 ceil,
  `expect_generic` ceils).
- **Suite** (the sandbox reaps background jobs — `nohup tools/test/suite.py` died in under 60 s
  with an empty suite.out, canary checked; foreground chunks instead, `--release
  --test-threads=1`): nf-game lib 188/188 + integration 35/4/1/2/2 green; nf-formats 9+1+2+1+73
  green with exactly the 3 allowed `disc_golden` fails (the slim-ISO set); nf-audio green;
  nf-view lib 9/9 + menus_flow 36 + mission_stats_flow 3 + front_end_chain_live 1 +
  fix_progress_live 3 + fix_progress_detected_hook_live 2 + fix_scores_handoff_live 1 +
  progress_remainder_live 4 + progress_remainder_failfirst 3 + menu_live 12 +
  menu_backdrop_still 1 + menu_movies 2 + menu_woman_live 5 + dev_menu_flow 1 — all green.

## What's left

- **The draw**: the port's stats arm renders the plain rows; the main agent's render pass
  against PCSX2 should place the art/columns (the pause SCORE tab's measured column x's are the
  only layout reference in-repo). Suggested observation point: finish 07000005 and press the
  side action on the results page; the numbers must match this report's golden.
- **The stats △ target** [S]: the results page is the certain opener; the game's exact back
  target is in the front-end archive's script for page 0x40000037 (unparsed). A live check:
  on PCSX2, open Mission Stats from the results page, press △, and record which page appears
  (results 0x40000036 expected) — and whether the side action on the BONUS page (0x40000038)
  also opens stats [M 0x209670:62-66 says it does].
- **The hour branch of `Timer_Seconds2String`** [S]: no reachable level has a > 99 min par, so
  the `h:mm:ss` format's exact glyphs are unverified (the case-2 `mm:ss` is [M]).
- The win page 0x40000053 and the bonus page 0x40000038 remain unported (reported by the
  front-end-chain task; this task's Confirm routing keeps their elision).
- The rest of the nf-view integration suite (~180 live binaries) unreached in this window —
  the sandbox kills background jobs, so the sweep needs the driver's runner; the change is
  additive to nf-game/nf-view with every touched binary run green above.

## ntfy failed

All three fallbacks hit ntfy.sh's daily quota (HTTP 429), 19:48 UTC 2026-10-15:
- curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
- python urllib: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
- wget: exit 8 (the server error response; empty body)
