# The codename name-entry page (codename-entry)

The "New Codename" page — reached from Select Codename 0x40000025's row 0, from Codenames → Edit
Codename 0x4000001b's row 0, and (as the cheat entry) from the "Edit \<name\>" wheel 0x4000001d's row 0 —
is **P_CNNAME_Handler 0x213790** [M Handler_HandleMessage 0x2029e0:372-373 maps page 0x40000020 to it].
Its letter grid is control 0x10000074, handled by **C_KEYBOARD_Handler 0x213a98** [M 0x2029e0:171]. The
validator is **Menu_IsValidCodename 0x1f9930**; the default proposal comes from the strings at
0x30d530 / 0x30d2d8 and the reserved-name list from **def_codename 0x2e1b18** [M symtab]. The port keeps
the game's data and flow in `crates/nf-view/src/menu.rs` (`Page::CodenameEntry`, the `cn_*` API) and the
card in `nf_game::profile` (docs/reports/fix-save-profile.md); the main agent draws every page.

## The three entries and cheat_mode [M]

`Manager_SendMessage` 0x21c858:302/344-345 passes the OLD page's id into the new page's 0x4c init, where
P_CNNAME computes `cheat_mode = (param_6 == 0x4000001d)` [M 0x213790:33]:

| opened from | handler | mode |
|---|---|---|
| 0x40000025 Select Codename row 0 | C_SBNFCN 0x20b390:46-49 (`Menu_MapDefaultCodename(0,0,0xff)` + `Menu_ChangePageCloseIris(0x40000020)` + the return-page send to 0x10000100) | profile names |
| 0x4000001b Edit Codename row 0 | C_SBCNSELECT 0x212f58 case 0x4b sel 0 | profile names |
| 0x4000001d wheel row 0 ("Secret Unlocks") | C_SBCNOPTIONS 0x213528 case 0 | **cheat** |

In cheat mode the ✓ runs **Menu_SpecialCodenameCheck 0x200238** instead of the validator [M 0x213a98:67-73]:
a 52-entry table built on the stack (names copied from rodata 0x308578-0x3087a0 — PASSPORT, POWDER, …,
LAUNCH — each `{char name[0x20]; u32 unlock_mask; u64 bonus}`, stride 0x38), compared with `strcmp`
(exact, case-sensitive) [M :1148]; a hit writes the 12 cheat bools at sp_level+0x10 (0x2df2f0, stride
0x18) and calls Menu_SetBonus, then C_KEYBOARD opens box 0xc "Unlock successful" (Txt 0x100029d) and sets
`cn_modified_flag` [M 0x213a98:71-72]. **Not ported** (reported): the port records the check ran
(`cn_cheat_checked`) and creates nothing.

## The grid [M + L]

- The typed buffer is `buffer` at **0x34e028** (shown through control 0x10000075 via msg 0x18 [M
  0x213790:46]); capacity **8 characters** — an append is refused once `strlen > 7` [M 0x213a98:75-77].
- Cell ids are the pressed cell's `ctrl+0x20`: **1000 = ←** (erase the last character, :39-42), **1001 =
  → / space** (appends the byte at DAT_0030d380 = 0x20 `' '` [L ee_snapshot], :28-36), **1002 = ✓**
  (confirm, :45-65). Digit/letter cells type the first byte of the cell's own text (queried with msg 0x2f,
  :79-85) [M].
- P_CNNAME's init re-labels four cells — SendEx(grid 0x10000074, cells 100/101/102/103, msg 0x18, the
  single bytes **0xc4 / 0xc2 / 0xc6 / 0xc0** written at 0x30d8d8/da/dc/de) [M 0x213790:50-57, disassembly
  `24060064/0065/0066/0067` + `sb`s] — and focuses **✓ 1002** [M :47-48, `addiu $a2,0x3ea` + msg 0x39].
  Which glyphs the menu font draws at those slots is the draw code's question.
- The title label 0x10000242 binds Txt **0x1f5 "Enter New Codename"** in the normal mode and **0x10002d4
  "Secret Unlocks"** in cheat mode [M 0x213790:37/41; both strings [D USATxt.dat]].
- The proposals: the normal entry copies the 8 bytes at **0x30d530 = "BOND"** over `buffer`; the cheat
  entry copies **0x30d2d8 = ""** [M 0x213790:36/43-44; the bytes [L ee_snapshot]].
- `del_when_keypressed` (set on open [M 0x213790:34] and after every refusal [M 0x213a98:50]) makes the
  FIRST press after open clear `buffer` back to the proposal default and **swallow the space key's press
  entirely** [M 0x213a98:18-27].
- The port's row packing (digits 0-9, A-Z over three rows, then the four glyph cells + ← → ✓) is [S];
  the game's packing sits unparsed in page 0x40000020's grid data. Typed keys going straight into the
  buffer (`cn_type_char`) is the port's own PC convenience [S]; the grid stays the reference.

## The validator [M]

**Menu_IsValidCodename 0x1f9930**: empty (strlen 0) → **0x3bc** "Please enter a codename consisting of at
least one letter or number"; else trim both ends in place, `strupr` and — on success — write the
normalized name back into `buffer` [M :60-77]. It then strstr's the uppercased name against each of
**def_codename 0x2e1b18**'s entries: two records, stride 0x38, bounded by 0x2e1b87, first dword = the Txt
label — **0x29b "New Codename"** (0x2e1b18) and **0x211 "Default"** (0x2e1b50) [L ee_snapshot; the strings
[D USATxt.dat]]. A hit returns **0x2c0** "This codename is not allowed because it is too similar to the
default codename. Please enter a new codename" [M :63-76]. So a name is refused when it *contains* "NEW
CODENAME" or "DEFAULT" case-insensitively. C_KEYBOARD binds the returned label onto option box **6**
(`Menu_CreateOptionBox(player, label, 6, 1, 0)` [M 0x213a98:47-51]).

## The accept and the card [M]

✓ with a valid name: `DAT_002e1ae0 = 999` (a NEW save slot), `strcpy(0x2e1ae8, buffer)` (the working
codename), `PlrStats_ResetScoring`, `ls = 1`, `DAT_002e1ad8 = 0x4000001d` (the continuation), box mode
`DAT_002e1b09 = 1` [M 0x213a98:53-65] — and `Menu_UpdateMessageBox` 0x1f7de8 case 1 runs **LS_Save
(0x2e1ae8)**: the file name IS the codename [M docs/reports/fix-save-profile.md]. P_CNNAME's 0x50 tick
watches option box **4** (the card flow's overwrite question, MemCard_ASK_OVERWRITE 0x10ee38) and saves on
YES [M 0x213790:62-76]; the port refuses instead and surfaces `cn_box = (4, 0)` [M the existing
create_codename note]. After the save the flow lands on the wheel 0x4000001d, whose init forwards to
Controls **0x40000022** when coming from 0x40000020 non-cheat [M P_CNMENU 0x213288:38-40 — not ported;
the port stops on the wheel and records `cn_exit = 0x4000001d`].

## Edit Codename: the list, ~X Delete, the wheel's Save Codename [M]

- The list page 0x4000001b (P_CNSELECT 0x212ad0) rows: 0 "New Codename", 1 "Default", 2+ the card's names
  (Menu_PutCodenamesIntoControl 0x1f79c0). Confirm on a saved row loads it — box mode 0 = LS_Load [M
  C_SBCNSELECT 0x212f58 case 0x4b sel ≥ 2] — and opens the "Edit \<name\>" wheel (header `sprintf(Txt
  0x10001e4 "Edit %s", 0x2e1ae8)` [M P_CNMENU 0x213288:32-33]).
- **~X Delete**: C_SBCNSELECT case **0x5e** on a saved row (sel > 1) sets slot 999, `ls = 1`, box mode
  **3** → `psiDeleteData(0x2e1ae8)` / `psiDeletingData()` [M 0x212f58:78-95, 0x1f7de8:38-57] under the
  PS2 card flow's confirm; P_CNSELECT case **0x6e** re-enumerates the list afterwards (box mode 2 =
  `Menu_EnumSaves(player, 0)` [M 0x212ad0, 0x1f7de8:31-36]). The port: `Action::Side` arms
  `cn_delete_row`, Confirm = YES (Profile::delete_in + reload), Back = NO.
- **The wheel's LAST row (row 6) is "Save Codename"** (Txt **0x1f4** [L CN_MENU[6] = table 0x2dfc28;
  the string [D USATxt.dat]]): C_SBCNOPTIONS case **6** — for a default (slot < 2) it re-maps
  (Menu_UpdateDefaultCodename) and leaves; for a saved codename it saves through box mode **1** =
  `LS_Save(0x2e1ae8)` [M 0x213528:54-70] — the codename's edits persist under its own name. There is NO
  in-place rename in the game: a new name typed through name entry always creates a NEW slot-999 file
  [M]; "rename" = create the new codename, then ~X-delete the old one (both ported).
- Leaving the wheel with `cn_modified_flag` (0x30d527 [M symtab], set by the option pages) pops the box
  **0xd** with Txt 0x10002d3 "Are you sure you want to exit and lose your changes?" and vetoes the close
  [M P_CNMENU 0x213288:52-54] — the loss confirm, not ported (the port's Back goes straight back).

## The port's surface (crates/nf-view/src/menu.rs)

`Page::CodenameEntry` (id 0x40000020) + `FrontEnd` fields `cn_typed/cn_cell/cn_from/cn_clear_on_key/
cn_box/cn_delete_row/cn_reserved/cn_exit/cn_cheat_checked` (each added on its own line [WORKLOG lesson
4]); the API: `cn_open(from)`, `cn_press(cell)`, `cn_type_char(ch)`, `cn_move(dx,dy)`, `cn_confirm`
(private), `cn_dismiss_box`, `cn_delete_begin/confirm/cancel`, the getters `cn_typed_name /
cn_cursor_cell`, the consts `CN_KEY_DELETE/SPACE/CONFIRM` (1000/1001/1002), `CN_GLYPH_CELLS` (100-103),
`CN_GRID` (the [S] packing), `CN_MAX_CHARS` (8). Wiring: Select Codename 0x40000025 row 0 and Edit
Codename row 0 call `cn_open` [M the handlers above]; the wheel's row 6 saves via `save_profile`; the
delete flows through `Profile::delete_in` (new in nf-game::profile, mirrored on
`Menu_UpdateMessageBox` case 3). `nf-view --page cnname|cncheat` render the page headless for the main
agent's `--shot`.

Tests: `crates/nf-view/tests/codename_entry.rs` (14 — the grid ids, the clamp, the swallow, the
validator's two refusals, create/route, overwrite refusal, delete, the wheel's save, the cheat path,
plus the NF_ISO label sister) and `nf_game::profile::delete_tests::delete_in_removes_only_the_named_file`.
