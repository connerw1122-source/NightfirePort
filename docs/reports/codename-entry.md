# codename-entry — report

Branch `s/codename-entry` (off origin/master 678c9195). Task: port what the codename name-entry page
shows and does — P_CNNAME_Handler 0x213790, C_KEYBOARD_Handler 0x213a98, Menu_IsValidCodename 0x1f9930,
def_codename 0x2e1b18 — no drawing.

## Done

- **docs/systems/codename-entry.md**: the full data + flow doc with [M]/[D]/[L] tags — the three entries
  and cheat_mode (from-page via the 0x4c param), the grid (buffer 0x34e028, 8-char clamp, cell ids
  1000 ← / 1001 space / 1002 ✓, the four glyph cells 100-103 with font bytes 0xc4/0xc2/0xc6/0xc0, the
  titles 0x1f5 "Enter New Codename" / 0x10002d4 "Secret Unlocks", the proposals "BOND"/""), the validator
  (trim + strupr, 0x3bc empty, 0x2c0 reserved-substring against def_codename's labels 0x29b "New Codename"
  / 0x211 "Default" at 0x2e1b18/0x2e1b50 [L ee_snapshot]), the accept (slot 999 → LS_Save via box mode 1,
  overwrite box 4, continuation 0x4000001d → Controls 0x40000022), Edit Codename's ~X Delete (msg 0x5e →
  box mode 3 psiDeleteData + the card-flow confirm, re-enum via 0x6e), and the wheel's LAST row = Txt
  0x1f4 **"Save Codename"** (C_SBCNOPTIONS case 6 = box mode 1 LS_Save(0x2e1ae8) for a saved codename;
  Menu_UpdateDefaultCodename re-map for a default). Note: the game has NO in-place rename — a new name
  creates a new slot-999 file; rename = create + delete (both ported).
- **Port**: `Page::CodenameEntry` (id 0x40000020) in `crates/nf-view/src/menu.rs` — state fields
  (`cn_typed/cn_cell/cn_from/cn_clear_on_key/cn_box/cn_delete_row/cn_reserved/cn_exit/cn_cheat_checked`,
  each on its own line) and the model API (`cn_open`, `cn_press`, `cn_type_char`, `cn_move`,
  `cn_dismiss_box`, `cn_delete_begin/confirm/cancel`, getters `cn_typed_name`/`cn_cursor_cell`, consts
  CN_KEY_*, CN_GLYPH_CELLS, CN_GRID, CN_MAX_CHARS=8). Wired: Select Codename row 0 and Edit Codename row
  0 open the page; Edit Codename's ~X delete with its YES/NO; the wheel's last row saves the loaded
  profile in place (`save_profile`); the picked codename's data loads before the wheel
  (`load_profile_data`) so the save writes real data. `nf-view --page cnname|cncheat` render headless.
- **nf_game::profile**: added `Profile::delete_in(dir, name)` (mirrors Menu_UpdateMessageBox 0x1f7de8
  case 3 → psiDeleteData).
- **Tests** (numbers/ids only, each with a sister that can fail):
  `crates/nf-view/tests/codename_entry.rs` — 14 tests: the proposals, the first-press clear + swallowed
  space, the 8-char clamp + ← key, the grid's cell ids (digits/letters/glyphs/1000-1002), the glyph
  cells' bytes, empty → box (6, 0x3bc), reserved → box (6, 0x2c0) with a passing contrast name, create →
  file + lists + `cn_exit = 0x4000001d` + normalized "TESTING", second create → box (4,0) overwrite
  refusal, ~X delete YES/NO, the wheel's last row writes the edit + the default re-map, the cheat confirm
  creates nothing, the cursor walk/clamps, and the NF_ISO sister binding all 11 label ids against
  USATxt.dat. Plus `nf_game::profile::delete_tests` (the delete removes only the named file).
  **Result: 14/14 + 6/6 profile lib tests green** (the label sister ran against the ISO and printed the
  strings).

## How it was verified

- Every behavior cites the decompile + disassembly (capstone over ACTION.ELF; the SendEx argument roles
  and the glyph immediates were confirmed in the raw instruction stream, e.g. `24060064/0065/0066/0067`,
  the focus `240603ea` + msg 0x39). The EE snapshot supplied the data bytes ("BOND" at 0x30d530, the
  space at 0x30d380, the def_codename dwords). USATxt.dat supplied the strings through the port's own
  `Lang` parser (the ISO-gated test prints them).
- The delete/box-mode semantics were re-derived from Menu_UpdateMessageBox 0x1f7de8 itself: mode 0 =
  LS_Load, 1 = LS_Save, 2 = Menu_EnumSaves, 3 = psiDeleteData. This CORRECTED the research pass's "wheel
  row 6 = delete" reading — row 6 is "Save Codename" (Txt 0x1f4 confirms it).

## What I ran

- `cargo check --offline -p nf-game -p nf-view` (clean), then
  `cargo test --offline --release -p nf-view --test codename_entry -- --test-threads=1` (14/14, with
  NF_ISO set) and `cargo test --offline --release -p nf-game --lib profile` (6/6).
- The fast runner (`tools/test/suite.py`) was started in the background per the task but this sandbox
  kills detached processes between tool calls (suite.out empty, process gone — the same result the
  s/playtest-spots worker recorded). No full-suite total line this round; the four-crate command's
  nf-game/nf-view portions ran green as above. Recommend the main agent run the suite on the merged tree.

## What's left

- The cheat path's Menu_SpecialCodenameCheck 0x200238 (the 52-name table at 0x308578) — recorded, not
  ported; the port surfaces `cn_cheat_checked` and creates nothing.
- The page's grid DATA (0x40000020's control records in 07000048.bin) is unparsed — CN_GRID's row packing
  is [S]; the draw agent should render from CN_GRID + the getters and compare against PCSX2.
- The wheel's modified-loss confirm (box 0xd, Txt 0x10002d3) and the Controls forwarding 0x40000022 are
  documented, not ported.

## Shared files touched

- `crates/nf-view/src/menu.rs` (the new page + API; one existing line widened: the Left/Right remap's
  `matches!` now also lists Page::CodenameEntry; the CnMenu Confirm arm restructured for row 6 — no
  existing arm renamed or repurposed).
- `crates/nf-view/src/lib.rs` (two appended `--page` arms), `crates/nf-game/src/profile.rs` (an appended
  `impl Profile` block + test module), `crates/nf-view/tests/codename_entry.rs` (new),
  `docs/systems/codename-entry.md` (new), `docs/WORKLOG.md` (appended only).

## Live checks for the main agent

- PCSX2, Select Codename → New Codename: the grid's four non-alphanumeric keys should read (via the
  debugger's text hooks at 0x30d8d8/da/dc/de) the single bytes 0xc4/0xc2/0xc6/0xc0 — confirm which glyphs
  the font maps there (expected ← / → / ✓ / a fourth, order per the SendEx sequence cells 100,101,102,103).
- On a refused name the option box shows Txt 0x2c0 / 0x3bc; on ✓ with "BOND" already on the card the
  overwrite box (id 4) appears [M P_CNNAME 0x50:62-76].
- The wheel's last row label must read "Save Codename" (Txt 0x1f4) and saving a modified codename rewrites
  its file (LS_Save), not deletes it.
