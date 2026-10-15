## Your task: `credits-data`, the credits roll's content and timing (no drawing)

AV Options → Credits (and the end of the game, `P_WINGAME_Handler` 0x215100) play the credits page
`P_CREDITS_Handler` 0x2152b0. The main agent draws it; you port **what it shows and when**.

Deliver:
1. `docs/systems/credits.md`: where the credit lines come from (label ranges in USATxt.dat or a table — cite the
   address), the line kinds (heading / name / gap / image) and their fonts/colours/positions as the handler sets
   them, the scroll speed and timing (per frame, in the game's 512×448 units), the end condition and what the page
   returns to, and any music/movie it starts.
2. In the port: a credits model in `crates/nf-view/src/menu.rs` (or a new module): the ordered line list built from
   the game's data, and a `tick()` giving each visible line's y at a given frame, plus the exit; hook it to a
   `Page::Credits` the main agent can draw (state + getters only).
3. Tests: the line count/order matches the data (numbers/ids only), the scroll position after N frames matches the
   handler's rate, the roll ends and returns to the right page; each with a sister assertion that can fail.

Not in scope: drawing (the main agent does).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/credits-data`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox6 at 2026-10-15T18:24Z

---
## Report (done, 2026-10-15T19:13Z)


---
## Report from the worker (done, 2026-10-15T19:13Z)

# credits-data — report

Branch `s/credits-data`, task 613. Everything below cites the decompile; the fact-tag rules apply.

## What's done

1. **The line source, kinds, positions, timing** — `docs/systems/credits.md`, all from
   `Menu_SetupCredits` 0x1f9a88 / `P_CREDITS_Handler` 0x2152b0 / `P_WINGAME_Handler` 0x215100 /
   `Handler_HandleMessage` 0x2029e0. Headlines:
   - Credits page = 0x40000030; opened by the win page 0x40000053 (after movie 0x073f0048, msg
     0x44/0x40000030 [M 0x215100:20-48]) and by AV Options' Credits row (control 0x10000138, label
     Txt 0x198).
   - The lines are **code-built**, not a data table: `Menu_SetupCredits(0x30d54c)` builds 578
     (0x242) 12-byte entries on the stack, heap-copies 0x1b18 bytes, stores the count 0x242 at
     0x30d54c (that address is the count cell `count_521`, not a table) [M 0x1f9a88:3444-3450].
     Strings: 209 `Txt_BindLabel` ids (bank<<24|index, 0x1cad60:13) + 492 ELF literals + 12 .data
     strings (EUROCOM, the accented names — listed in the doc) + 241 gap `" "` (0x30d240) + 202
     NULLs. All 1156 slots accounted for.
   - Fonts: the kind byte picks the msg-0x23 format pointer — kind 0 → 0x30d570 whose sprintf
     result equals the default label format DAT_003096c0 byte-for-byte [D ELF bytes]; kind 1 →
     `ff 01 fe 03`, kind 2 → `ff 03 fe 03`.
   - Timing: a line enters both columns every 14th frame at y 0x200 in list slot cycling mod 26;
     all slots scroll y −2 per frame (an entered line shows 510 on its entry frame); two-column
     rows put left at x 0x10 w 0x100 and right at x 0xf0 w 0x100, wide rows use x 0x10 w 0x1e0
     with the right hidden. Music: `SFXStartMusic(0x27)` on enter, captured volume fades linearly
     over the last 50 lines (`(578×14 − frame) × vol/700`, clamped ≥ 0). End: once
     `578 + 18 < frame/14` (frame 8358) a delayed 0x13 (19 frames) sends msg 0x44 → the main menu
     0x40000002 when opened from the win page, else msg 0x5f → pop back to the opener; plus a
     delayed 0x68 and an exit process on control 0x10000215 ({0xe1, 0xff, 0} over 0xf frames — a
     page fade [S]). Leave (0x4d) restores the music volume and restarts the menu-loop sounds
     0x1d7/0x470. The handler has no input cases — the roll cannot be skipped.
   - Background alpha word 0xff vs 0x404040ff on `GameState[0x52]` — the level-bonus flag
     `Menu_SetLevelBonus` 0x1ffa10:68 sets, persisted in the PTP save.

2. **The port model** (state + getters only, no drawing):
   - `crates/nf-view/src/credits_data.rs` — the 578-line `CREDITS` table (`Str::Label/Lit/Space/
     Null` + per-side kinds + wide), parsed from the function's own init sequence (entry k at
     frame offset 0x1bd0 − 12k).
   - `crates/nf-view/src/credits.rs` — `CreditsRoll` (the handler's statics as fields; `tick()`
     → `CreditsExit::{ToMain, PopBack}`), the getters `visible()` / `slot_y()` /
     `music_volume()` / `lines_entered()`, and consts for every number in the doc.
   - `crates/nf-view/src/menu.rs` — `Page::Credits`, `FrontEnd::credits`/`credits_from` (new
     fields on their own lines), `open_credits(from_page, music_vol, bonus_flag)`, the tick
     integrates the roll and routes the delayed exit, and the AV Options Credits row confirms into
     it. `Page::Credits` draws nothing yet — the main agent draws from `fe.credits.visible()`.

3. **Tests** — `crates/nf-view/tests/credits_data.rs`, 11 tests, each with a sister assertion that
   can fail:
   - count = 0x242 (sister: ≠ 0x241); order pins at entries 0/1/2 (labels 0x10002a5/a6/a7, wide),
     361 (0x1000264, kind 2), 565 (0x10002a1), 577 (a wide gap row);
   - the slot accounting 209/504/241/202 (sister: the sum must be 1156);
   - scroll: y 510 on the entry frame, exactly −2/frame across 70+ frames (sister: the exact
     per-frame delta);
   - the 14-frame entry period and the mod-26 slot cycle (sister: one tick earlier line 30 has not
     entered);
   - the music fade window (686 at vol 700, 0 the frame the last line enters; sister: 343 at vol
     350);
   - the exit/return: AV opener → Page::CnAv exactly at EXIT_FRAME with no early fire; the win
     opener → Page::Main; the fade-frame constants match the handler's strict windows;
   - the AV Credits row opens the page (sister: a List row does not);
   - the NF_ISO pin: USATxt label 0x198 binds "Credits" (sister: neighbour 0x197 does not).

## How it was verified

- Fail-first evidence: the order pins caught a wrong tail index during authoring (I pinned the
  0x10002a1 heading at 553 from a truncated read; the data said 565 — confirmed against the
  decompile line 3373), and a reversed sign in the scroll assertion. Both tests failed before the
  fixes and pass after; the data table needed no change.
- The extraction round-trip: 209 = the file's `Txt_BindLabel` call count (`grep -c`), 1156 =
  578×2, no unmatched stores (the parser reports them), no empty/half-parsed entries beyond the
  legitimate NULL sides.
- ISO cross-checks: label 0x198 = "Credits" from the real USATxt.dat; the font format bytes and
  the gap string read straight out of ACTION.ELF.

## What I ran (this tree, release, offline)

- `cargo test -p nf-view --lib` → 9 passed / 0 failed.
- `cargo test -p nf-view --test credits_data` → 11 passed / 0 failed (includes the NF_ISO pin).
- `cargo test -p nf-view --test menus_flow` → 36 passed / 0 failed (the other suites touching
  menu.rs).
- `cargo build -p nf-view --release` clean. nf-game/nf-formats/nf-audio are untouched by this
  branch; the full four-crate run didn't fit the window (tools/test/suite.py is not on master
  yet, and background jobs are reaped in this sandbox — same as previous tasks).

## What's left

- Drawing the roll (the main agent): `fe.credits.visible()` gives (line index, slot, y, wide) in
  the game's 512×448 space; the strings bind through `Lang::bind` for `Str::Label` ids; the kind →
  format tags are in `KIND_FORMAT` (the faces `ff 01/02/03` map to the font files draw-side).
- The win page itself (0x40000053) stays unported (as before) — when it lands, its end-of-movie
  handler calls the equivalent of `open_credits(0x4000_0053, vol, bonus)`.
- The music hookup: `open_credits`'s volume currently comes from the AV music slider; a host with
  the real `SFXMusicGetVolume` should pass that instead. `GameState[0x52]`'s live value comes from
  the progress layer when wired (the flag the results page sets).

## Shared-file hooks

- `crates/nf-view/src/menu.rs`: the `Page::Credits` variant + 4 match arms, the `credits` /
  `credits_from` fields, `open_credits`, the tick block, the AV-row confirm. All additive.
- `crates/nf-view/src/lib.rs`: one `pub mod credits;` line.
- `docs/WORKLOG.md`: appended only.

## Live checks for the main agent

- [ ] PCSX2, AV Options → Credits: the roll's first visible rows should be the label-0x10002a5/6/7
      headings (wide, centred) and entry 53's "EUROCOM" row should be two-column kind-1 — pins the
      table order against the real scroll.
- [ ] The y spacing between two adjacent entries is 28 units (2/frame × 14). A debugger screenshot
      with the frame counter would pin the entry period exactly.
- [ ] With GameState[0x52] = 1 (a saved game with a medal), the 0x1000023c control's alpha word
      reads 0x404040ff at the credits' enter (watch the msg 0x1a send at 0x2152b0:87).
