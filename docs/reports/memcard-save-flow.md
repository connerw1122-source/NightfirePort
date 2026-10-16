# memcard-save-flow — the save flow around the profile

Branch `s/memcard-save-flow`. The task: port WHEN the game saves (the triggers), the messages
it shows ("Saving…", the card-full / no-card errors — which apply to a PC file save and which
don't), and the autosave behaviour; map the memory-card states onto the port's file save
honestly; state + messages on `FrontEnd`, the file write staying in the Profile API the host
already points at the profiles dir.

## What's done

1. **The doc first**: `docs/systems/memcard-save-flow.md` — every trigger with its handler and
   address, the LS_*/psi state numbers, `HandlePS2CardState` 0x110b98's CardState table, the
   full `PS2_DoCardFlow` 0x217bd8 message → Txt-label → buttons table with the USATxt.dat
   strings [D], the MENU_LS 0x2e1ad0 layout (+0x3a finished / +0x3b error / +8 the queued page
   message), the completion routing, and the autosave findings.
2. **The triggers** ([M] evidence in the doc §1): the mission results commit
   (P_NFRESULTS 0x208e78 — the page's Confirm opens "Do you want to save your progress?"
   Txt 0x1000206 when `PlrStats_DoneBetter` and a SAVED codename is in play; the answer Yes
   saves, No routes on; a failed save re-asks), the codename create (C_KEYBOARD 0x213a98 ✓ →
   mode 1 → the completion queues the wheel page 0x4000001d), the codename menu's Save
   Codename row (C_SBCNOPTIONS 0x213528 case 6), and the codename delete (C_SBCNSELECT
   0x212f58 case 0x5e → the ASK_DELETE confirm first). The boot/list/MP enums are reads, not
   saves. The main options pages never save — the language/volume edits ride the next trigger
   (the GSET block). Autosave: `xbox_save_flag` is never set on this disc [M] and the
   "Autosaving data…" string (Txt 0x26f) has no code binding [M — every Txt_BindLabel site
   grepped]; the prompted results save IS the game's one automatic end-of-mission save.
3. **`CardFlow` on `FrontEnd`** (`crates/nf-view/src/menu.rs`): `CardMsg` (id/label/buttons =
   the game's numbers), the `BTN_*` constants (the answer value IS the bit, as on PS2),
   `CardState` Idle/Ask/Run/Error, and the machine — `request_save` / `request_create` /
   `request_delete` / `card_tick` (the port's `HandlePS2CardState` pump, called from
   `tick()`) / `card_answer(row)` / `card_message()` for the draw code. Every save to an
   existing codename asks the overwrite question [M 0x10fa08 sub-7 + psiSaveData's
   `OverWriteCheckDone` reset]; Retry re-runs a failed op; Cancel aborts (the terminal
   failure); the completion applies the queued page message [M 0x217bd8:62-64] or routes the
   results page (with the game's re-ask after a failure [M 0x208e78:196-201]). A confirmed
   overwrite-create writes the fresh template, exactly like `Menu_MapDefaultCodename` [M
   0x1f7fa8].
4. **The honest mapping** (doc §4): shown = Saving/Deleting progress, the overwrite and
   delete questions, Save failed / Delete failed / Load failed, and card-full (msg 3) when
   the OS reports `io::ErrorKind::StorageFull` — the file analogue of CHECK_FREE_SPACE 0x10ef00.
   Never shown (no PC equivalent, no fake card errors): no-card (1/0x10), unformatted/format
   (2/0xa/0x11), corrupt-data (6 — the port's CRC refusal surfaces Load failed instead),
   insert-a-card (0xf), Checking (0xb).
5. **The load failure path**: a missing/corrupt profile file now surfaces "Load failed!"
   (msg 7, Retry+Cancel) from both codename loads; Retry re-runs the load [M 0x10f138]. This
   completes fix-save-profile's "What's left" item 2 (the overwrite flow; the delete flow was
   already half-wired and now runs through the machine with the game's question).
6. **Tests** (`crates/nf-view/tests/memcard_save_flow.rs`, 7 tests): the message tables
   (numbers only), the USATxt.dat strings pinned against the disc via NF_ISO (including the
   never-shown card set, so they were read not assumed), the results commit writing ONCE with
   the synced contents (the unlock bits + the profile's progress in the file), the No answer
   never writing + the non-improving commit never prompting, the write failure surfacing
   "Save failed!" with the Retry/Cancel loop and the results re-ask, the corrupt load
   surfacing "Load failed!" with a working Retry, and the enums/defaults never writing. Every
   test has a failing sister (the ops_done counters, the file-state asserts). The older
   codename_entry / campaign_flow_live contracts moved to the flow model (the write now runs
   in the flow's tick, the results page prompts first) — the campaign chain test now answers
   the prompt + the overwrite question like the game.

## How it was verified

- `cargo test --lib` per crate on the final tree: nf-game **212/212**, nf-formats **79/79**,
  nf-audio **9/9**, nf-view **15/15**.
- `tools/test/suite.py` on the final tree (origin/master merged — it was already up to date,
  0f5f07f4), `--only` the touched + neighbouring binaries: codename_entry, menus_flow,
  memcard_save_flow, fix_save_profile_live, campaign_flow_live, mp_menu_flow, dev_menu_flow,
  objective_hints_flow, credits_data, dossier_pages, screen_adjust, fix_menus_live,
  fix_pause_menu_live, fix_codename_live, progress_live, fix_progress_live → **119 passed,
  0 failed, 0 ignored (1 min)**, `build ok: 367 test binaries`. The suite ran foreground (this
  sandbox reaps background jobs — same as the previous workers' notes); the full 367-binary
  sweep did not fit the window after the 333 s cold build + the campaign run; the build cache
  is warm for the next window.
- The evidence chain for the message texts: `work/decomp` + the symtab gave the state machine;
  the strings came out of `USATxt.dat` inside `FILES.BIN` (offsets from
  `crates/nf-formats/golden/files_bin.tsv`, bank layout per `Txt_LoadLanguage` 0x1caad0), and
  the ISO-gated test re-derives them through the port's own `Lang` parser.

## What's left (honest list)

1. **The draw pass** — the card dialogs need the main agent's rendering: the results save
   prompt (`results_box`, Txt 0x1000206, the game's box 0xb on page 0x40000036), and the card
   dialog (`card_message()` + `card.buttons`) over the keyboard/codename pages. The game
   renders these on the LBERROPTIONS dialog page with the buttons row; `--page` hooks for
   them are trivial once the draw lands (the state is all on `FrontEnd`).
2. **`io::ErrorKind::StorageFull` → card-full is untested end-to-end** (needs a filesystem
   that returns ENOSPC deterministically); the mapping is a one-line arm in `card_tick` next
   to the tested `SaveFailed` path.
3. **The MP join's mode-5 write** (`P_MPJOIN` 0x20ca00 sets `DAT_002e1b09 = 5`, which has no
   case in `Menu_UpdateMessageBox`) — documented, not ported; MP saves are out of this task.
4. **The Driving disc's own card flow** (`DRIVING.ELF`) — out of scope; the port's driving
   hand-off keeps the ACTION-side behaviour.
5. The overwrite question fires on EVERY save to an existing codename [M — the game's literal
   reading]. If playtesting finds the real cart skips it on the results save, the gate is
   `OverWriteCheckDone`'s reset in `psiSaveData` 0x110df0 — a live check settles it.

## Live checks for the main agent

In PCSX2 (NTSC-U):
1. Complete mission 1 with a saved codename: does the results page ask "Do you want to save
   your progress?" and then ALSO ask "This codename already exists. Would you like to
   overwrite it?" before "Saving data…" (my reading of 0x10fa08 sub-7 + 0x10ee38), or does
   the overwrite question only appear on the codename-create path? gdbwatch on
   `DAT_002462d4` 0x2462d4 during the save shows the message-id sequence (expected 9 during
   the write; 4 if the question fires).
2. Change the language on the main options page, then complete a mission: the GSET block in
   `InBufData` 0x3416e0 during the save should carry the NEW language id (the options pages
   never save on their own [M — no mode writers among the CH* handlers]).

## ntfy

(not run yet — sent after this commit; the exact output lands here if all three fallbacks
fail.)
