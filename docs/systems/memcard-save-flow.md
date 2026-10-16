# The memory-card save flow (when the game saves, and what it shows)

Evidence tags: **[M]** decompiled ACTION.ELF code (function + address), **[D]** the disc,
**[L]** an EE snapshot (`work/ee_snapshot.bin`, file offset = EE address), **[S]** suggested.
Strings are from the disc's language bank: `Txt_BindLabel` 0x1cad60 resolves label ids through
`USATxt.dat` inside `FILES.BIN` [D USATxt.dat offset 3086336 size 135168 in the FILES.BIN
layout `crates/nf-formats/golden/files_bin.tsv`; the bank format is
`u32 blobSize, blob, pad4, u32 numEntries, numEntries-1 u32 offsets, u32 numFixups, fixups`
— M `Txt_LoadLanguage` 0x1caad0]. This doc covers the FLOW around the profile; the save
file's contents are `docs/systems/` + `docs/reports/fix-save-profile.md` (already ported).

## 1. When the game saves

All card operations funnel through one dispatcher: a page handler sets a MODE global
`DAT_002e1b09` plus the file name buffer `DAT_002e1ae8` and calls
`Menu_UpdateMessageBox(page, mask, slot)` 0x1f7de8, whose switch on the mode runs the op:
case 0 = `LS_Load` 0x1cdfc0, case 1 = `LS_Save` 0x1ce0c0, case 2 = `Menu_EnumSaves(slot, 0)`
0x1f7b78, case 3 = the delete flow (`psiDeleteData` 0x111168, `del_state_161`), case 4 =
`Menu_EnumSaves(slot, 1)` [M 0x1f7de8:17-57]. Every case ends in
`PS2_DoCardFlow(manager+page·0x1d8, state, &MENU_LS 0x2e1ad0)` 0x217bd8, the message machine
(§3). `Menu_UpdateMessageBox` is re-entered every tick by the page handlers while the op runs
(e.g. P_CNMENU msg 0x50 calls it unconditionally [M 0x213288:14-16]).

Case 1's gate [M 0x1f7de8:21-30]: saves only run when the main menu is up
(`DAT_002e1b0d != 0` — set once by P_MAIN_Handler 0x20a7b0:58 on the front end's init) and
the name buffer is non-empty; otherwise the op is silently dropped and the completion flag
`DAT_002e1b0a` (MENU_LS+0x3a, §3) is set so the waiting page routes on.

The triggers (who sets mode 1 = save, or mode 3 = delete):

| trigger | handler | what it does |
|---|---|---|
| **Mission results commit** | P_NFRESULTS_Handler 0x208e78 | msg 0x4b (the page's Confirm) commits the score (`PlrStats_DoneBetter` 0x1986c0) and, when a SAVED codename is in play (`DAT_002e1ae0 > 1`), opens the "Do you want to save your progress?" box (Txt `0x1000206` [D], box id 0xb, Yes/No) [M 0x208e78:24-32]. msg 0x50 (the box's answer): row 1 (Yes) → mode 1, mask 0xff, name = the codename, `Menu_UpdateMessageBox(page, 0xff, 0)` [M :174-186]; row 2 (No) → only the completion flag [M :186-189]. When no saved codename is in play there is NO save and NO prompt — the Default template is updated in memory and the page routes on [M :33-36]. |
| **Codename create** | C_KEYBOARD_Handler 0x213a98 (the keyboard's ✓) | after `Menu_IsValidCodename` 0x1f9930 and the special-codename (cheat) checks, a valid name sets mode 1, mask 0xff, name = the typed buffer, and pumps the box [M 0x213a98:44-64]. P_CNNAME_Handler 0x213790 does the same on its tick when the name was accepted [M 0x213790:62-76]. The post-save page message `MENU_LS+8` = 0x4000001d (back to the codename wheel) [M both]. |
| **Codename options save row** | C_SBCNOPTIONS_Handler 0x213528 | the codename menu's row 6: with a saved codename in play (`DAT_002e1ae0 >= 2`) → mode 1, mask 0xff, and `MENU_LS+8` = the row's next-page message [M :60-69]; with the Default/Play-without-saving rows (0/1) it only updates the in-memory template via `Menu_UpdateDefaultCodename` and changes page — no file write [M :52-58]. |
| **Codename delete** | C_SBCNSELECT_Handler 0x212f58 | the wheel's delete on a saved row (sel ≥ 2): mode 3 = the delete flow under the selected name [M :80-92]. The flow asks first (§3, ASK_DELETE). |

That is the complete save/delete trigger set — every `Menu_UpdateMessageBox` caller was
enumerated [M the 13 files listed by grep over `work/decomp/ACTION.ELF`].

Load/enum triggers (for context, not saves): boot enumeration P_PARISENUM 0x20a648 (msg 0x4c
sets `DAT_002e1b0d = 1`, mode 4), the mission-select list P_NFSELECT 0x20aec0 and codename
select P_CNSELECT 0x212ad0 (mode 2), codename loads C_SBNFCN 0x20b390 / C_SBCNSELECT
0x212f58 / C_RBMPCNAME 0x20d448 (mode 0), MP join P_MPJOIN 0x20ca00 (mode 2; it also writes
mode 5 which has no case in 0x1f7de8's switch — no card op [M 0x20ca00:158-163]), and the
boot-up card check `psiDoBootupCheck` 0x11228 from P_PS2MEMCARDINIT_Handler 0x218538 msg 0x4c
[M].

**Option changes**: the ONLY options page that writes the file is the codename menu's save
row (above) [M 0x213528]. The main options pages (audio/video/language) never request a save
— none of their handlers appears in the mode-writer set [M grep] — so a language or volume
change reaches the card only at the NEXT save trigger (the results commit or the codename
menu's save row; the values live in the GSET block, `fix-save-profile.md`).

**Autosave**: on this disc the flag `xbox_save_flag_272` (which would save at the results
page without asking) is only ever CLEARED, never set [M 0x208e78:69; grep finds no other
access] — it is the Xbox build's auto-save hook. The language bank does contain
"Autosaving data. Do not remove memory card…" (Txt 0x26f [D]), but no code binds it [M every
`Txt_BindLabel` call site grepped; no menu-def page references it either — the draw side
would show it only through a data-driven label]. The results-page save (prompted with the
0x1000206 box, then written under the "Saving data…" message) is the closest thing to an
autosave: it is the game's one automatic end-of-mission save, and the save HAPPENS without a
further confirm once Yes is chosen.

## 2. The save operation itself

`LS_Save(name, slot, mask)` 0x1ce0c0: builds the block chain into `DstData[slot·0x1000]`
(`LS_GetSaveStuff` 0x1cd0d8 walks `SaveIFFBlocks` 0x2c74d8 — six blocks, PLRS/MSSN/MSPG/GSET/
CHET/BNUS, masks 1..0x20 [M; L the table bytes in the snapshot]; mask 0xff = all six), copies
it to `InBufData[slot·0x1000]`, and hands it to `psiSaveData` 0x110df0, which appends the
CRC-32 and pushes CardState 9 (SAVE_DATA). `Save_State` 0x30cf16: 0 idle → 2 running →
7 when `psiSavingData()` 0x111160 (which just returns `PSIOP_State` 0x30d6d8) returns 2
(success) → 8 on 4 (failure) [M 0x1ce0c0]. `PSIOP_State`: 0 idle, 1 op started
(`psiSaveData`), 2 success, 4 aborted/failed [M all psi* writers]. Every new save resets
`OverWriteCheckDone` 0x30d6d4 and `PS2SaveGameFine` 0x30c894 [M 0x110df0].

`MemCard_SAVE_DATA` (CardState 9) 0x10fa08 runs the sub-state chain: card status → card
present? (type at 0x316948 != 2 → the ASK_FORMAT path) → chdir to the save's path →
**if the target already exists: ASK_OVERWRITE once per op** (`OverWriteCheckDone`) → create
the save "directory" if absent (-4) → open/write/flush/close with the "Saving data…" message
up → on completion verify the written cluster count (0x4e) → `PS2SaveGameFine = 1`,
`PSIOP_State = 2` [M 0x10fa08 sub-states 0-0x10]. Any negative step result pushes CardState 6
(SAVEERROR); a missing directory (-4) pushes 7 (CREATE_DIR) [M sub-7].

The machine is driven per frame by `HandlePS2CardState` 0x110b98, which dispatches
`CardState[StateStack]` (a stack; values: 1 sceMcSync wait, 2 FORMAT, 3 CHECK_FOR_CARD,
5 CHECK_FREE_SPACE, 6 SAVEERROR, 7 CREATE_DIR, 8 ASK_FORMAT, 9 SAVE_DATA, 10 GET_CARD_STATUS,
0xb LOADERROR, 0xc LOAD_DATA, 0xd FILE_CORRUPT, 0xe GET_SAVE_GAMES, 0xf SAVEERROR2,
0x10 ASK_OVERWRITE, 0x11 DELETE_DATA, 0x12 DELETE_DATA2, 0x13 DELETE_ERROR, 0x14 ASK_DELETE,
0x15 CHECK_FOR_CARD2, 0x16 BOOTUP_CHECK, 0x17 FORMAT_ERROR) [M 0x110b98:22-102].

Deletes: `psiDeleteData` 0x111168 pushes 0x11; `MemCard_DELETE_DATA` 0x110320 asks first
(pushing 0x14 ASK_DELETE) unless already confirmed, then deletes the save's files and
directory under the "Deleting data…" message; `MemCard_DELETE_DATA2` 0x110638 is the same
walk WITHOUT the question (it is what SAVEERROR's Retry pushes first, to clear the partial
write before retrying [M 0x10f2d8:28-36 → 0x110638 sub-9 resets the stack to 1]).

## 3. The messages (PS2_DoCardFlow 0x217bd8)

`PS2_DoCardFlow(page_manager, state, MENU_LS* ls)` shows one message per flow-state
`DAT_002462d4` on the dialog page (the 0x100000fb/0xfc/0xfd controls), sets the button row
from `PS2CardFlow` 0x2462d0 (a bit mask: bit 0x1 = OK Txt 0x3ce, 0x2 = Yes 0x3cf, 0x4 =
Cancel 0x3cd, 0x8 = Retry 0x3d0, 0x10 = Format 0x3d1, 0x20 = Delete 0x3d2 [M 0x217bd8:178-207;
D the strings]), and while a dialog is up blocks input (`Manager_SendMessage 0x68,1`).
The pressed button's value comes back through `DAT_002462da` when the dialog page reports the
selection (`C_LBERROPTIONS_Handler` 0x2185e8 msg 0x4b reads it and raises `DAT_002462d9` =
"answer ready" [M]); each MemCard_* state interprets the value (the answer value = the
button's bit: OK 1, Yes 2, Cancel 4, Retry 8, Format 0x10, Delete 0x20 [M each state's
`DAT_002462da ==` tests]).

The message table [M 0x217bd8:76-152; D USATxt.dat strings; the button masks from the
MemCard_* functions]:

| id | label | text ([D] USATxt.dat, %s = "MEMORY CARD slot 1", %d = 0x52 = 82) | buttons | set by |
|---|---|---|---|---|
| 0 | — | none | — | idle |
| 1 | 0x26b | "No memory card (8MB)(for PlayStation®2) in MEMORY CARD slot %s." | 0xc Cancel+Retry | CHECK_FOR_CARD 0x10e858 (card type != 2) |
| 2 | 0x269 | "Memory card … in MEMORY CARD slot 1 is unformatted. Format memory card …?" | 0x14 Format+Cancel | ASK_FORMAT 0x10ed60 |
| 3 | 0x26a | "Insufficient space on memory card … in MEMORY CARD slot %s. NightFire requires %dKB of free space to save data." | 0x1 OK | CHECK_FREE_SPACE 0x10ef00 (free < 0x52 KB) |
| 4 | 0x27d | "This codename already exists. Would you like to overwrite it?" | 0x6 Yes+Cancel | ASK_OVERWRITE 0x10ee38 |
| 5 | 0x26d | "Save failed! Check memory card … in MEMORY CARD slot %s and please try again." | 0xc Retry+Cancel | SAVEERROR 0x10f2d8 / SAVEERROR2 0x10f3c0 |
| 6 | 0x278 | "Memory card … in MEMORY CARD slot %s error! Data may be corrupt." | 0x1 OK | FILE_CORRUPT 0x10efc0 |
| 7 | 0x273 | "Load failed! Check memory card … in MEMORY CARD slot %s and please try again." | 0xc Retry+Cancel | LOADERROR 0x10f138 |
| 8 | 0x275 | "Loading data. Do not remove memory card …" | — (progress) | LOAD_DATA 0x110870 |
| 9 | 0x26c | "Saving data. Do not remove memory card …, reset, or switch off the console." | — (progress) | CREATE_DIR 0x10f4a8 + SAVE_DATA's write phases |
| 0xa | 0x27a | "Formatting memory card …" | — (progress) | FORMAT 0x10eab8 |
| 0xb | 0x277 | "Checking memory card … Do not remove memory card …" | — (progress) | BOOTUP_CHECK 0x10f778 / GET_SAVE_GAMES 0x110188 |
| 0xc | 0x3e6 | "Deleting data. Do not remove memory card …" | — (progress) | DELETE_DATA 0x110320 |
| 0xd | 0x3e5 | "Are you sure you want to delete this codename?" | 0x24 Delete+Cancel | ASK_DELETE 0x10ec98 |
| 0xe | 0x3e7 | "Delete failed! Check memory card … in MEMORY CARD slot %s and please try again." | 0xc Retry+Cancel | DELETE_ERROR 0x10f068 |
| 0xf | 0x268 | "Insufficient space on memory card … in slot %s. Insert a memory card … with at least %dKB of free space or create some space using the console's internal browser." | 0x9 OK+Retry | BOOTUP_CHECK (free <= 0x51) |
| 0x10 | 0x267 | "No memory card … inserted into MEMORY CARD slot %s. You must insert a memory card … with at least %dKB of free space available to be able to save your progress in this game." | 0x9 OK+Retry | BOOTUP_CHECK (no card) |
| 0x11 | 0x27c | "Format failed! Please check memory card … and try again." | 0xc Retry+Cancel | FORMAT_ERROR 0x10f208 |

Terminal handling [M 0x217bd8:26-72]: a flow state ≥ 4 (LS_* states 7 success / 8 failure —
the values `Menu_UpdateMessageBox` passes on are the `Save_State`/`Load_State` returns, not
the message ids) closes the dialog, sends the page's pending message `MENU_LS+8` (+0xc param)
if one is queued, raises MENU_LS+0x3a (`DAT_002e1b0a` — "the flow finished"), calls
`LS_FlushStates` 0x1ce1b0 + `Menu_FlushEnum`, and re-enables input. State 8 additionally sets
MENU_LS+0x3b (the error flag), clears delayed messages and re-opens the dialog if the page
had one queued. The page message is how the flow hands control back: codename create queues
0x4000001d (reopen the wheel) in MENU_LS+8 [M 0x213a98:58], the options save row queues the
row's own next-page message [M 0x213528:61], the results save queues none (the results page
watches `DAT_002e1b0a` itself [M 0x208e78:192-216]).

The results page's completion routing [M 0x208e78:192-216]: when the flow's finished flag
stands and `PS2SaveGameFine` is still 0 while the answer was NOT "No" — the save did not
verify — the page re-opens the 0x1000206 box (ask again). Otherwise it routes on: a new
reward → page message 0x40000038, the final mission → 0x40000053, else 0x4000001c (main
menu).

## 4. The port's honest mapping

The port has no memory card: the "card" is one `<codename>.profile` file in the profiles dir
(fix-save-profile), and the host writes it through `Profile::save_to_dir` /
`FrontEnd::save_profile`. The flow is ported as a small state machine on `FrontEnd`
(`CardFlow`, `crates/nf-view/src/menu.rs`) with the game's ids so the draw code and tests can
pin numbers:

- **Shown, because they mean the same thing on a file save**: 9 "Saving data…" while the
  write runs (the do-not-remove text kept verbatim for fidelity — [S] the host may swap it
  for a PC-flavoured line later; the draw side renders the game's string); 4 the overwrite
  question (a file exists); 5 "Save failed!" on any write error; 0xd the delete question;
  0xc "Deleting data…"; 0xe "Delete failed!" on a remove error; 7 "Load failed!" when a
  profile file is missing/corrupt (the game shows it for a failed load [M LOADERROR's push
  from LOAD_DATA's read/CRC failures 0x110870:29,49,76]). The boot-time "Checking memory
  card…" (0xb) is NOT shown — the port's enumeration is a synchronous directory read with
  nothing to wait for [S; the game shows it during GET_SAVE_GAMES 0x110188's async walk].
- **Card-full is honest when the OS says so**: a write failing with
  `std::io::ErrorKind::StorageFull` shows message 3 (0x26a, the "NightFire requires %dKB"
  text) — the same meaning; any other error kind shows 5. [S] mapping choice, documented.
- **Never shown (no PC equivalent — a fake card error would be a lie)**: 1/0x10 no-card,
  2/0x11 unformatted/format-failed, 0xa formatting, 6 "data may be corrupt" (the port's CRC
  check refuses the file with the load-failed message instead), 0xf insert-a-card. The port's
  machine cannot enter their states.
- The triggers (§1) map 1:1: results commit (via the 0x1000206 prompt), codename create, the
  codename menu's save row, the delete confirm. "Play without saving"/Default never write a
  file [M §1]. There is no background/autosave timer on either side; `xbox_save_flag` stays
  unused.
- The flow is driven by `FrontEnd::card_tick()` (the host calls it per frame, like
  `HandlePS2CardState`); the write itself runs inside the tick (the port's card IS the local
  file — there is no async device), with the getters `card_message()` /
  `card_buttons()` / `card_awaiting()` for the draw code and `card_answer(row)` for input
  (row values = the button bits, as on PS2).

## 5. What this changes in the port

- `crates/nf-view/src/menu.rs`: `CardMsg`/`CardButtons`/`CardFlow` + the `FrontEnd.card`
  field; `request_save`/`request_delete`/`card_tick`/`card_answer`/the getters; the results
  page's save moved behind the game's prompt (the save used to run inside `results_enter`);
  `create_codename` keeps its API but the keyboard/create path routes through the flow so an
  existing codename asks 0x27d instead of failing hard (the old `CodenameError::Exists`
  stays for direct callers).
- The host (nf-view's front-end loop) pumps `card_tick` and renders the message; no other
  host change — the file IO stays in the existing `Profile` API.
- Tests: `crates/nf-view/tests/memcard_save_flow.rs` — the id tables (message id → label →
  buttons, numbers only), each trigger writing exactly once with the right contents, the
  failure paths surfacing the game's messages, and an ISO-gated pin of the USATxt strings.
