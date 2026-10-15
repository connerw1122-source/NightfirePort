# unlock-popups — report

Branch `s/unlock-popups` (from origin/master 1d1b98dc). Everything below cites the evidence in
`docs/systems/unlock-popups.md`; [M] = decompile, [L] = dump made this task.

## What's done

**The three post-results unlock pages are ported as data + state + flow + getters (no drawing,
per the task):**

1. **The routing** (`crates/nf-view/src/menu.rs`, the Results/Complete Confirm arm): after the
   results page, the game's exit sends msg 0x44 to 0x40000038 (007 Bonus) while a new reward
   stands (`DAT_002e1c2c`, set inside `Menu_SetLevelBonus` 0x1ffa10's reward loop), else to
   0x40000053 (WINGAME) while `just_completed` (level == 0x0700001b, the SetLevelBonus return),
   else to 0x4000001c (the mission list) [M P_NFRESULTS 0x208e78:39-52/:204-217].
2. **P_NFBONUS 0x40000038**: `FrontEnd::bonus_enter(progress, level, has_text)` rebuilds
   `Menu_GetLevelBonuses` 0x1ffe98's record — per tier 1..4 `Menu_HasMedal(level, tier, 0)` plus
   the RewardsTable entry's label (+0x1c..+0x28) and texture (+0x30..+0x3c), shipped as the new
   `nf_game::progress::REWARD_TEXT` static [L ee_start_05.bin 0x2bc920 dump]. The composed text
   keeps the game's tier 1→4→3→2 order [M 0x209670:22-41] and the DAT_002e1c2c gate (no text at
   all when no reward is new [M :16-17]). Page geometry + label ids pinned [L the menu-def dump]:
   header Txt 0x2c5, hint 0x2d0, texture control 0x100001cf (89,112,171×256), text list
   0x100001d0 (268,94,284×292), aux movie 0x07350048.
3. **P_WINGAME 0x40000053**: `win_enter()` / `win_movie_finished()` model the handler's movie
   state machine [M 0x215100:12-49] — the page is a bare record with no controls/aux [L], the
   handler plays 0x073f0048 itself, and the movie's end opens the credits page 0x40000030 with
   the win page as the source [M :48].
4. **P_CHEATMEDAL 0x4000004c**: the handler is ported (6-row list; rows grant
   `medal_from_cheat` 0x30cc74 = 999/1/2/3/4 and poke the mission-end channels 99/100 [M
   0x2086b0:21-45]) with its reachability flagged as an OPEN QUESTION: no PAGE record in the
   menu-def and no code/data route into it anywhere [L the 56-page walk + the whole-decompile
   grep].

## How it was verified

- `crates/nf-view/tests/unlock_popups.rs` — 12 headless tests, numbers/ids only, each with a
  sister assertion (the routing trio including the "no reward → mission list" arm; the bonus
  record with a gold finish vs a platinum finish; the no-text gate; the unknown-level zero
  record; the cheat-medal grant table incl. the 999 row; the wrap; the page ids; the REWARD_TEXT
  dump pins).
- `crates/nf-view/tests/unlock_popups_live.rs` — ISO-gated: walks the disc's menu-def like
  `MenuManager_Create` and pins NFBONUS's layout, WINGAME's bare record, CHEATMEDAL's absence,
  and that no control of any of the 56 pages routes into the three ids.
- Run on this tree: `cargo test --offline --release -p nf-view -p nf-game -- --test-threads=1`
  — every binary green (205 lib + 35 + the per-file sets incl. the 12 + 1 new ones; no FAILED).
  Fail-first recorded: the live pin "the win page has no controls" was wrong (the root control
  counts) and the test caught it before the fix.

## What's left

- The draw pass for the three pages (the main agent's; data sources listed in the draw arm's
  comment — `fe.bonus`, the pinned geometry).
- The CHEATMEDAL opener and any reader of `medal_from_cheat` 0x30cc74 / 0x27031c / 0x270320 —
  write-only in ACTION.ELF [M]. Live checks requested (see the doc's §5): gdbwatch
  0x27031c/0x270320 and 0x30cc74 through a results→bonus→map walk and a Dossier cheats visit.
- The NFBONUS separator/format strings at 0x34d048 / 0x30d350 / 0x30d358 (ELF .data bytes) for
  the draw pass's exact punctuation.
- The full four-crate suite ran as per-crate foreground chunks (background jobs get reaped in
  this sandbox): nf-view + nf-game fully green; nf-formats/nf-audio untouched by this branch
  (no code changed there — the branch's diff is menu.rs, progress.rs, two test files, two docs).

## ntfy failed

All three fallbacks hit the ntfy.sh daily quota (HTTP 429, code 42908 "limit reached: daily
message quota reached") at 2026-10-15 ~22:28 UTC: curl printed
`{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`,
the urllib fallback raised `HTTPError: HTTP Error 429: Too Many Requests`, and wget returned
empty. The finish.sh step itself succeeded (`OK already closed in done/, report attached` for
619-unlock-popups).
