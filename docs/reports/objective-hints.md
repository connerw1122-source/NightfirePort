# objective-hints — the pause OBJECTIVES tab's ✕ Hint

Branch `s/objective-hints` (merged with origin/master at ce62cd7e). Task: port the OBJECTIVES tab's hint data,
selection and ✕ flow from `C_GCPAUSE_Handler` 0x204408 — no drawing (the main agent draws the box against PCSX2).

## What's done

**The data model** (`crates/nf-game/src/mission.rs`, doc `docs/systems/objective-hints.md`):

- `Mission::num_vis_objectives()` = `Mission_NumVisObjectives` 0x182540 and `Mission::objective_state(vis)` =
  `Mission_ObjectiveState` 0x182630 — the exact traversal: LEVELS table order, other missions' records skipped,
  the walk STOPS at the first record of the mission whose part is beyond the current one (`ThisOrderNum < part`),
  counting each objective whose runtime state (`record+0x14`) is > 1 (announced: 2/3/4/5). `ObjectiveState
  { state, text (+0x00), hint (+0x08) }`. The game's OBJ_STATE[1] (0x02000001/0x02000000/0x16f/0x170 by state) has
  no reader (both 0x182630 call sites in 0x204408 read [0]/[2]/[3] only — checked in the disassembly at
  0x2047bc/0x2047e4/0x205bc8) — omitted, documented.

**The ✕ flow** (`crates/nf-view/src/menu.rs` FrontEnd, state + getters only):

- `pause_objectives: Vec<PauseObjective { text, hint, done }>` — was `Vec<(u32, bool)>`; the host
  (`pause_objective_rows`, crates/nf-view/src/lib.rs) now fills it from `Mission::objective_state` — the WHOLE
  mission's announced rows in LEVELS order (the old fill listed only the current level's record, missing multi-part
  missions' prior parts).
- `pause_objectives_sel` — control 8's selected row: re-selected to the LAST row on every tab entry (the populate's
  msg 0x1d = numVis−1 [M 0x204840-0x204850]), Up/Down move it (clamped).
- `pause_hint: Option<u32>` — the ✕ (Confirm) on tab 1 opens the modal ok-only hint box (`Menu_CreateOptionBox
  0x202218 → 0x1f9608, box_id 10, ok_only 1, the OK row label 0x266 [M 0x1f9608:34-42]`) with the selected row's
  hint label; **hint == 0xffffffff shows label 0x31d** [M 0x205bd0-0x205bdc]. Confirm/Back pop it (the box's 0x4b
  OK row / 0x6d cancel, option_return 4 ok-only [M C_LBMSGOPTIONS 0x2022a0]); while up, all other input idles
  (the box page 0x4000002f is modal [M 0x1f9608:45]); dismissal re-shows the list — the game's pause page handler
  does it on msg 0x50 (`option_return != 0 → control 8 0x2b(0)` [M P_PAUSE_Handler 0x203c28:19-24; Handler_HandleMessage
  0x2029e0 routes page 0x4000004b → P_PAUSE and menu-def 0x10000028's controls → C_GCPAUSE — page-level events vs
  control-level events]); the ok-only box returns 3/4 (both non-zero) so the list always comes back with its rows
  and selection intact. The port just clears the field, the row state untouched underneath.
- `open_pause()` resets the box + selection; the OBJECTIVES draw arm's yellow cursor follows `pause_objectives_sel`
  (was pinned to the last row).
- Render plumbing for the main agent: `--page pause --pause-tab 1 --pause-hint` pre-opens the box state
  (`pause_hint` from the cursor row, after the objectives fill) — the box itself is still to draw.

**The questions the task asked:**

- Which row is selected: the LAST visible row on entry, then ~V-scrolls (§ above).
- What ✕ shows: a modal MEMO/option box (page 0x4000002f) whose text is `Txt_BindLabel(record+0x08)` (or 0x31d when
  the label is −1), one "continue" row (label 0x266, row id 3), tinted 0x645a4cff [M 0x205c14-0x205c2c]; long hints
  page through the memo box (Memo_Update [M 0x1f9608:67] — the box's generic behaviour, no pause-specific code).
- Are hints limited / do they affect scoring: **no**. The hint label (record+0x08) has no reader outside
  `Mission_ObjectiveState` (decompile grep; `Hint_Create`/`Hint_Update` 0x1b3248/0x1b3380 are the in-world hint
  objects); PlrStat has no hints counter (the SCORE tab populate 0x204408:447-498 reads only PlrStat fields); the
  ✕ case writes no flag/channel/record byte [M 0x205b98-0x205c34]. Every shipped record HAS a hint (none is
  0xffffffff [L ee_start_05.bin records = the port's mission/c0*.rs]) — the 0x31d fallback is for safety/mods.
- Back: on the OBJECTIVES tab Back unpauses (no back action in the 0x4b arm; the hint bar 0x202 lists no ~B); on
  the box Back cancels it without unpausing (the port keeps `closed` false).

**Corrected docs** (surgical edits): `docs/systems/mission.md`'s "hint … never drawn by any ACTION.ELF code found"
claim (§1) and the two "pause-screen objectives list is not ported" gap lines (§7/§8). New doc:
`docs/systems/objective-hints.md` (full evidence trail + live checks).

## How it was verified

- `crates/nf-game/tests/objective_state.rs` — 5 headless tests on the real embedded records: fresh mission lists
  nothing; announced rows in table order with their text/hint (0x07000009's 0x04000016/21…); prior parts first +
  future parts cut (the part gate, 3+3+5 = 11 rows at order 2); other missions never leak; completed (4) and
  failed (5) rows still list, re-armed (0) rows drop out. Each with a sister assertion.
- `crates/nf-view/tests/objective_hints_flow.rs` — 7 tests through `FrontEnd::action()`: the last-row initial
  selection (and the 1-row case), ✕ on row N opens row N's hint (per-row), 0xffffffff → 0x31d (and a hinted row
  never hits the fallback), the box is modal (list keys idle, no tab cycle) until Confirm, Back on the box closes
  it WITHOUT unpausing (sister: Back without the box IS the resume), the selection clamps, no ✕ on an empty list,
  and an NF_ISO check binding 0x202 (the hint bar, contains the ~A glyph), 0x31d and 0x266 from the disc's
  USATxt.dat (distinct strings). All green on the merged tree.
- Touched-crate runs before the pushes: `cargo test --offline --release -p nf-game` (no failures) and nf-view's
  `menus_flow` (36/36) + `fix_objective_messages_live` (5/5) + the two new files (5/5, 7/7).
- Full suite: `NF_ISO=… nohup python3 tools/test/suite.py > suite.out 2>&1 &` on the final merged tree
  (ce62cd7e) — tally appended below when the run finishes.

## Suite tally (final tree)

- (pending — filled in before the final push)

## What's left / gaps

- The hint BOX drawing (memo box page 0x4000002f over the pause) is the main agent's: the state is
  `fe.pause_hint = Some(label)`; while set, the pause draws beneath it and the box needs the text bound from
  `pause_hint`'s label, one OK row (0x266) and the box art (0x1000011e/0x1000011f/0x10000120 are the option-box
  pages' controls). Multi-page memo behaviour for long hints (Memo_Update's paging) is not modelled — the box
  shows the whole bound text; the game wraps it [S].
- OBJ_STATE[1]'s constants are stored-unread in the game; if a live check finds a reader (a colour? a status
  word?), the struct can grow a field.
- The populate's per-tab hint-bar labels (0x201/0x202/0x204/0x205) were already drawn by fix-pause-menu2; nothing
  to do.

## Live checks for the main agent

1. **The last-row selection** (0700000a or any level with ≥ 2 announced objectives): pause → OBJECTIVES — the
   yellow cursor starts on the LAST row [M 0x204840-0x204850 msg 0x1d = numVis−1]; ~V moves it; ~H away and back
   re-selects the last row.
2. **The ✕ box**: ✕ on a row → the memo box over the paused frame, text = the objective's hint
   (0x07000009 row 0 → USATxt label 0x04000021), one continue button (0x266); ✕ or circle closes it and the LIST
   IS BACK with its rows (the 0x205670 sweep) and the MISSION tab's rows/hint bar are intact (tab 1 re-selected).
3. **Multi-page memo**: a long hint (e.g. 0700000b's "Find door to outer balcony" 0x04000026's text length) — if
   the game pages it, that is Memo_Update's generic behaviour; the port shows the full text (report what PCSX2
   does).
4. **Scoring**: show 2-3 hints on a level, finish it, compare the SCORE tab and the results' 007 Bonus against a
   no-hint run — no difference is expected [M no reader exists].

## Notes

- The earlier suite run (19:56-20:58) overlapped my edits + a disk-full incident (`cargo check` on a full disk);
  its 8 failures are not a clean signal — the final-tree run above is the one that counts.
- ntfy: (filled at finish).
