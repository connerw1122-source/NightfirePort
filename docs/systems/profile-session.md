# The codename session and the window wiring (profile-session / create-save)

The user, 2026-10-16: "you wont let me create a save" — Select Codename → New Codename should
open the name entry, take a name, save it to the port's memory card (the profile files), and
offer it on the next boot; finishing a mission under it saves progress. The front end's own
flow is complete and tested (docs/systems/codename-entry.md, docs/systems/memcard-save-flow.md,
docs/reports/fix-save-profile.md). What was missing is the WINDOW layer: `crates/nf-view/src/lib.rs`
never wired the card, so every flow that needs it failed silently in the real game window while
every headless test (which calls `set_profile_dir` itself) passed.

## The game's session globals [M]

The codename "in play" is NOT menu-manager state. C_KEYBOARD's ✓ writes three plain globals
[C_KEYBOARD_Handler 0x213a98:53-63, decompile]:

- `DAT_002e1ae0 = 999` — the save slot (999 = a fresh saved codename);
- `strcpy(&DAT_002e1ae8, &buffer)` — the working codename (the name IS the MC file name
  [M Menu_UpdateMessageBox 0x1f7de8:19/28 passes 0x2e1ae8 to LS_Load/LS_Save]);
- `DAT_002e1ad8 = 0x4000001d` (the continuation) + the box-mode byte `DAT_002e1b09 = 1`.

P_NFRESULTS reads the same globals later: the save-prompt gate `1 < DAT_002e1ae0` (the Default
slot and play-without-saving never prompt) [M P_NFRESULTS_Handler 0x208e78:30/194] and the
medal line's `sprintf(&medal_buffer_271, …, 0x2e1ae8)` [M :145-152]. The game's menu manager is
created and deleted around every page change and level load (MenuManager_Create/Delete), but
these globals stand — the session outlives every manager exactly like the music statics
`DAT_002e1b11` / `MusicMasterVolume` (docs/reports + frontend-music, menu-audio-state).

The port models one manager as `FrontEnd`, so the session must ride the App across every
`FrontEnd` create/drop the window does (boot, the gameplay pause's `get_or_insert_with`, the
results handover `demo_results`, the MP debriefing `mp_results`, `net_fail`) — the same shape
as `stash_music_statics`/`inherit_music_statics` [M lib.rs, the menu-audio-state pattern].

## The port's memory card [M + S]

`nf_game::profile` stores one IFF chain per codename at `<dir>/<codename>.profile`
[docs/reports/fix-save-profile.md; the chain PLRS/MSSN/MPSG/GSET/CHET/BNUS [L ee_start
InBufData 0x3416e0]]. The card's directory is the port's stand-in for the PS2 memory card:
`settings::config_dir()` — `%APPDATA%\NightfirePort` on Windows, `XDG_CONFIG_HOME|~/.config +
/NightfirePort` elsewhere, `NF_CONFIG_DIR` overriding for the tests [S settings.rs; the task's
named stand-in path]. The options file `settings.toml` already lives there (settings-persist),
so the card and the settings share the one directory.

Boot enumeration is `Menu_EnumSaves` 0x1f7b78's walk [M]; the port does it in
`FrontEnd::set_profile_dir` → `reload_profiles` (the row order 0 "New Codename", 1 "Default",
2+ the card's names [M Menu_PutCodenamesIntoControl 0x1f79c0]).

## The three window breaks this task fixes

1. **No card at boot.** `run()`'s front-end boot built `FrontEnd::new()` and loaded the option
   file but never called `set_profile_dir` — `profile_dir` stayed `None`, so on the name page
   `cn_confirm` → `request_create` returned false with no box and no page change (the ✓ was
   silently dead), Select Codename listed no saved codenames, and the results page's
   `saved_codename_in_play` never held, so progress never saved. Every `set_profile_dir` call
   in the tree was in a test. Fix: the boot seeds the card (the ProfileSession's dir) and the
   same seeding runs at every other fe create.
2. **Typed letters never reached the name grid.** The window's menu key branch mapped only
   `key_action`'s keys (arrows/WASD, Enter/Space, Esc/Backspace, Tab/E); the MP join/host pages
   had their own typing branches, the name page had none — `cn_type_char` had no caller outside
   tests, so the user could not enter a name from the PC keyboard. The game's own path sends
   every key through the grid control's msg 0x4b [M C_KEYBOARD 0x213a98:15-85], so the port's
   window branch maps a typed character to the same cell press (`cn_press(cell = the byte)`)
   and Backspace to cell 1000 [M :39-42], which keeps `del_when_keypressed`'s first-press clear
   (the proposal "BOND" clears, the pressed letter appends; the space CELL alone is swallowed
   [M :18-27]) and the 8-char cap [M :75-77] in one place. Letters/digits only: the grid's own
   alphabet [M the cells type the first byte of their text; the port's CN_GRID packing is [S]].
   Other keys keep `key_action` (Space = Confirm = press the focused cell — the game's X).
3. **The results handover dropped the session.** `demo_results` (the mission-end → results
   glue, F8/`pending_mission_end`) and `mp_results` build a FRESH `FrontEnd` carrying only the
   music statics and the option file — `profile_dir`/`active_profile`/`profile` were lost, and
   `sync_profile_progress` (the host merging the sim's tally into the profile before the save
   [M the game writes its MSSN/BNUS from the same live state the results page reads]) had no
   caller at all. Fix: the session stashes on the App at every fe drop and inherits at every
   fe create; `demo_results` also runs `sync_profile_progress` after `results_enter`.

## The port's surface

- `menu::ProfileSession` — the session the game keeps in globals: `dir` (the card),
  `name` (0x2e1ae8), `profile` (the loaded chain). `ProfileSession::from_fe(&fe)` is the stash
  side (the manager delete), `session.apply_to(&mut fe)` the create side (dir via
  `set_profile_dir` — which re-enumerates, the boot/card enums [M 0x1f7b78]).
- `menu::cnname_char_key(KeyCode) -> Option<u32>` — the typed key's grid cell id (letters,
  digits, the space bar's cell 1001); None keeps the key on `key_action`. The window consumes
  `Some(cell)` with `cn_press` before the action mapping.
- `App::stash_profile_session` (in `menu_after` and at every `fe = None` site) +
  `App::inherit_profile_session` (boot, pause, results/debriefing handovers, `net_fail`).
- The boot seeds `cn_reserved` from the language bank's two def_codename labels —
  0x29b "New Codename", 0x211 "Default" [M Menu_IsValidCodename 0x1f9930 strstr's the
  uppercased name against them; the labels [D USATxt.dat]] — the validator's reserved list,
  previously only ever populated by tests.

Tests: `crates/nf-view/tests/create_save_live.rs` — the create through the real action path
(Main → Select Codename → New Codename → ✓ → the card flow's tick), the typed-letter leg
through the window's key mapping, the next-boot offer + load, and the mission-end save under
the codename through the session handover. The reproduce run (before the fix) failed at the
create assert: the ✓ ran nothing — no file, no box, no page change.
