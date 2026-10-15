# The pause OBJECTIVES tab's ✕ Hint (objective-hints)

The in-mission pause page (menu-def page 0x4000004b, handler `C_GCPAUSE_Handler` 0x204408 — docs/systems/mission.md
§5/§8) has four tabs; the OBJECTIVES tab lists the mission's announced objectives and its ✕ shows the selected
objective's hint in a modal option box. This doc is the data model + flow the port implements (`Mission::objective_state`
in `crates/nf-game/src/mission.rs`, the `pause_*` fields/getters on `FrontEnd` in `crates/nf-view/src/menu.rs`); the
DRAWING is the main agent's (the rows and the box).

Tags: [M] decompile (work/decomp/ACTION.ELF, address + function), [L] EE snapshot / .data dump, [S] suggested.
Every test uses numbers/ids only; the label strings resolve through `Lang::bind` (USATxt.dat).

## 1. The rows (populate, msg 0x4e case 1)

`C_GCPAUSE_Handler` 0x204408's populate case for tab 1 [M 0x204408 — the 0x4e arm, prologue at 0x20447c, case 1 from
0x2046a8, the loop 0x20476c-0x204850]:

1. The prologue hides every control with msg 0x2b value 1 (controls 6, 8, 9, 0xa, 0xb, 0xc, 0xd, 0xe, 0x10, 0xf) and
   re-tints the page (msg 0x29 = 0x806d59ff) [M 0x2044a0-0x2046a4].
2. The tab's hint bar is label **0x202** ("✕ Hint ✚ Scroll ✚ Next" — the port's `~A Hint  ~V Scroll  ~H Next`), sent
   as `Menu_Send(0x1000006f, 0x18, Txt_BindLabel(0x202))` [M 0x2046a0-0x2046b0; tab 0 binds 0x201, the CONTROLS tab
   0x204, the SCORE tab 0x205].
3. Control 8 (the d1=8 list box, 98,147,445×190 [L page 0x4000004b's control]) is fetched with msg 0x39 and
   configured (0x75=100 width, 0x2b=0 show, 0x17 clear, 0x28/0xe/0x59 layout, `+0x26c = 0`) [M 0x2046c8-0x20476c].
4. For `i` in `0 .. Mission_NumVisObjectives()` [M 0x182540]:
   - `Mission_ObjectiveState(&obj, i)` [M 0x182630] fills the row;
   - msg 0x10 with the empty string at 0x30d2d8 [L ACTION.ELF .rodata 0x30d2d8 = `""`] adds the row;
   - column 1 gets the status marker string: `obj.state == 4` → 0x30d2e0 = `"~J"` else 0x30d2e8 = `"~I"` [M
     0x2047bc-0x2047fc; the strings are ACTION.ELF .rodata 0x30d2e0/0x30d2e8 [L] — font control codes drawn through
     the specialchar table];
   - column 2 gets `Txt_BindLabel(obj.text)` [M 0x204800-0x204828 — `lw a0, 0x8(sp)` = OBJ_STATE[2], the record's
     +0x00 label].
5. The selection is msg 0x1d with **numVis − 1** — the list opens on its LAST row (the most recent objective) [M
   0x204840-0x204850: `addiu a2, s2, -0x1; msg 0x1d`; the same 0x1d = "set selection" the YES/NO confirm opens with
   (value 1 = NO)].

### What is a "visible" objective

`Mission_NumVisObjectives` 0x182540 [M] walks the `MissionData` records **in table order**:

- only records of the CURRENT mission (`record.mission == BaseMap`);
- stopping at the first record whose part is beyond the current one (`ThisOrderNum < record.part` → return);
- counting each objective record whose runtime state (`record+0x14`) is **> 1** (or < 0 in the game's signed read —
  the shipped states are 0..5): the ANNOUNCED states 2 (fail-objective pending), 3 (announced), 4 (complete),
  5 (failed) [M the same gate in 0x182630's fill loop; the state machine is docs/systems/mission.md §3].

So pending objectives (0/1 — not yet announced) never appear, earlier PARTS of the same mission do (the port's old
per-level fill missed them), and future parts never do. `Mission_ObjectiveState` 0x182630 [M] is the same walk,
returning the `vis`-th hit as a 4-word `OBJ_STATE`:

| slot | content | reader |
|---|---|---|
| [0] | the live state (`record+0x14`) | the populate loop (the ~J/~I marker) [M 0x2047bc] |
| [1] | a state-keyed constant: 2 → 0x02000001, 3 → 0x02000000, 4 → 0x16f, else → 0x170 [M 0x182630] | **none found** — both call sites in 0x204408 read [0], [2], [3] only (0x2047bc/0x2047e4/0x205bc8); the port omits it |
| [2] | `record+0x00` — the title label | the row text [M 0x2047e4] |
| [3] | `record+0x08` — the hint label | the ✕ box [M 0x205bc8] |

The port: `Mission::num_vis_objectives()` / `Mission::objective_state(vis) -> Option<ObjectiveState>` with
`{ state, text, hint }` (crates/nf-game/src/mission.rs) — the exact traversal above over the embedded LEVELS records
and `Mission::states`.

## 2. The ✕ Hint (msg 0x4b, control 1)

The ✕ press arrives as the 0x4b arm (`0x20443c beq a3, 0x4b → 0x205934`) on **control 1** — the page's hidden hint
button [M 0x205934-0x20595c: the arm switches on `param_2->0x20` (the activated control): 6 = the MISSION rows,
0xd = the YES/NO confirm, **1 = the hint**]. Case 1 [M 0x205b98-0x205c34]:

1. `selected = Menu_SendEx(0x10000028, 8, msg 0x34)` — control 8's selected row [M 0x205ba0-0x205bb8].
2. `Mission_ObjectiveState(&obj, selected)` [M 0x205bbc jal 0x182630].
3. **The no-hint fallback**: `if (obj.hint == -1) obj.hint = 0x31d` [M 0x205bc4-0x205bdc: `lw v1, 0xc(sp)`,
   `bne v1, -1`, else `sw 0x31d, 0xc(sp)`]. 0x31d is a real lang label (bound at draw; every shipped objective
   record HAS a hint — 0xffffffff appears in none of the 24 tables [L ee_start_05.bin's records, the port's
   mission/c0*.rs] — so the fallback is for safety/mod levels).
4. Control 8 is hidden (0x2b(1)) [M 0x205be0-0x205bf8] and the hint opens as a **modal option box**:
   `Menu_CreateOptionBox(mgr, hint_label, box_id=10, ok_only=1, 0)` [M 0x205bfc-0x205c10 jal 0x202218 → 0x1f9608]:
   - the box text page 0x1000011f gets `Txt_BindLabel(hint)` (msg 0x18) [M 0x1f9608:23];
   - ok_only → ONE list row, label **0x266** (the "continue" text), row id 3 [M 0x1f9608:33-42 — `param_5 == 0 →
     0x266`, `Menu_Send(0x10000120, 0x10, label, 3)`];
   - the memo box (multi-page: Memo_Update 0x1f9608:67) sizes itself from the text — long hints page through
     ("~H Next" while reading) [S — the memo's paging is the memo box's generic behaviour; no pause-specific code];
   - the modal page 0x4000002f pushes over the pause (manager msg 0x44) [M 0x1f9608:45], and the box page 0x10000120
     is re-tinted 0x645a4cff [M 0x205c14-0x205c2c].
5. **Dismiss**: the box's OK row (0x4b on the box → `option_return = the row's value` [M C_LBMSGOPTIONS_Handler
   0x2022a0]) or a cancel (msg 0x6d → `option_return = 4` because ok_only [M 0x2022a0's else arm]) pops the box
   (manager msg 0x5f) [M 0x2022a0]. When the box pops the manager notifies the page handler with **msg 0x50**, and
   the pause page's own handler re-shows the list: `P_PAUSE_Handler` 0x203c28:19-24 (page 0x4000004b's handler —
   `Handler_HandleMessage` 0x2029e0 routes 0x4000004b → P_PAUSE and the menu-def 0x10000028's controls →
   C_GCPAUSE; the split is page-level events vs control-level events) does `option_return != 0 → Menu_SendEx
   (0x10000028, 8, 0x2b, 0)` — the ok-only box returns 3 (OK) or 4 (cancel), both non-zero, so the list ALWAYS
   comes back with its rows and selection intact (they were only hidden, never cleared). The port models the box
   as `FrontEnd::pause_hint: Option<u32>` — set on ✕, cleared on Confirm/Back (both pops), every other action
   swallowed while it is up (modal), the row state untouched underneath.

### Selection state

`FrontEnd::pause_objectives_sel` (usize) is control 8's selected row: it starts at **numVis − 1** on every entry to
the tab (the populate's 0x1d [M 0x204840-0x204850]) and moves with Up/Down (~V Scroll — the list's own behaviour).
The draw puts the yellow cursor at it (the port's old code pinned the cursor to the last row).

## 3. Back, and the other tabs

- **Back** (circle) is not the objectives tab's action: the 0x4b arm has no control for it and the hint bar lists no
  ~B — Back unpauses through the generic manager path (the port's existing `Back → closed`).
- **~H Next** cycles tabs (the port's Side action — 0 MISSION / 1 OBJECTIVES / 2 CONTROLS / 3 SCORE); every entry
  re-runs the populate (the 0x1d selection reset included) [M the per-tab binds at 0x2046a0/0x2047c/0x204858/…].
- **Scoring**: hints are FREE. The hint label (record+0x08) has no reader outside `Mission_ObjectiveState` 0x182630
  [M the decompile grep — only 0x204408 calls it; the `Hint_Create`/`Hint_Update` pair 0x1b3248/0x1b3380 is the
  in-world hint objects, unrelated]; no "hints used" counter exists in `PlrStat` (the SCORE tab's populate reads
  PlrStat fields only [M 0x204408:447-498]), and the 007 Bonus targets are the static score records [L 0x2bc658].
  Showing a hint changes nothing in the mission state — the handler writes no flag, no channel, no record byte
  [M 0x205b98-0x205c34: the case's only writes are the 0x2b send, the OBJ_STATE stack slot and the box].

## 4. The port's surface

- `nf_game::mission::ObjectiveState { state, text, hint }` + `Mission::num_vis_objectives()` /
  `Mission::objective_state(vis)` — the game's traversal (§1).
- `FrontEnd::pause_objectives: Vec<PauseObjective>` — `PauseObjective { text, hint, done }` per visible row
  (`done = state == 4`), filled by the host from the sim's mission (`pause_objective_rows`, crates/nf-view/src/lib.rs)
  — now the WHOLE mission, not just the level's record.
- `FrontEnd::pause_objectives_sel: usize` — the selected row (starts at the last, §2).
- `FrontEnd::pause_hint: Option<u32>` — the open hint box's label (0x31d already applied); Some = modal.
- `FrontEnd::action()` Page::Pause: tab 1's Up/Down move the selection, ✕ (Confirm) opens the box with the selected
  row's hint (the −1 → 0x31d fallback [M 0x205bd0-bdc]), Confirm/Back close it, other input idles while it is up;
  entering the tab re-selects the last row.

## 5. Tests

`crates/nf-game/tests/objective_state.rs` — the traversal on the real embedded records (pending excluded, prior
parts included, the part stop, text/hint passthrough, the −1 sentinel untouched here). `crates/nf-view/tests/
objective_hints_flow.rs` — the flow (the last-row initial selection, ✕ on row N opens row N's hint, the 0x31d
fallback, the modal, Back closing, the re-selection on tab entry; one NF_ISO-gated check binds 0x202/0x31d/0x266
from the disc's USATxt.dat). Each has a sister assertion that can fail.

## 6. Live checks for the main agent

1. **The last-row selection**: PCSX2, 07000005 pause → OBJECTIVES (1 announced objective — sel = 0; use a later
   level with several). gdbwatch on control 8's selection word is awkward; instead: with ≥ 2 announced objectives
   the yellow cursor should sit on the LAST row when the tab opens (fieldprobe screenshot).
2. **The hint box**: ✕ on a row → the memo box over the paused frame with the objective's hint text and one
   "continue"-labelled row; ✕ again (or circle) closes it and the list returns WITH its rows and selection (msg 0x50
   re-shows control 8 [M P_PAUSE 0x203c28:19-24]).
3. **The no-hint fallback**: not reachable on the shipped records (every hint ≠ −1 [L]) — only a mod would show
   0x31d's text ("No hint available" or similar — bind 0x31d from USATxt.dat to see the shipped string).
4. **Scoring**: show several hints, finish the level, check the SCORE tab / results — no "hints" row, no bonus
   delta [M §3; a screenshot of the score tab before/after suffices].
