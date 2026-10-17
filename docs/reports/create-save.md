# create-save — the player could not create a save

The user, 2026-10-16: "you wont let me create a save". Select Codename → New Codename should
open the name entry (P_CNNAME 0x40000020, the letter grid C_KEYBOARD 0x213a98), the ✓
confirms, the codename is created and saved to the port's memory card (the settings/save
files under %APPDATA%/NightfirePort), and progress then saves after each mission. Done:
the window wiring the front end's (already complete, already tested) flows were missing —
found by walking the real input path, fixed with the game's own session model, pinned by
live tests through the real action path. Branch `s/create-save`, base 31dd6161.

## Where it actually failed (reproduce, on current master)

The front end's codename flows are complete and were fully tested
(docs/systems/codename-entry.md, docs/systems/memcard-save-flow.md,
docs/reports/fix-save-profile.md) — but every `set_profile_dir` call in the tree was in a
TEST. The real window (`crates/nf-view/src/lib.rs`) never wired a card, so in the game:

1. **The ✓ was silently dead.** On the name page Enter runs `cn_press(1002)` → `cn_confirm`
   → `request_create` → `profile_dir` is `None` → returns false → the press consumes
   nothing: no box, no save, no page change. This is exactly the user's symptom.
2. **Typed letters never reached the grid.** The window's menu key branch mapped only
   `key_action`'s keys; the MP join/host pages had typing branches, the name page had none —
   `cn_type_char` had no caller outside tests, so the player could not enter a name at all.
3. **Select Codename listed nothing** (`profiles` never enumerated), and **the results
   handover dropped the session**: `demo_results`/`mp_results` build a fresh `FrontEnd`
   carrying only the music statics and the option file, so even with a created codename the
   results page's save gate (`1 < DAT_002e1ae0`, [M] P_NFRESULTS 0x208e78:30/194) could
   never hold — progress never saved after a mission. `sync_profile_progress` had no caller
   at all.

The reproduce test (`crates/nf-view/tests/create_save_live.rs`) drives the REAL window path:
the front end built the way the window builds it (no card seeding — that is the bug), then
keys only through `key_action`'s actions (Main → Codename page → row 0 → the name entry →
the ✓) and the card pump the window's redraw runs. Master output (pre-fix, unmodified tree):

```
test creating_bond_from_a_fresh_boot_saves_it_to_the_card ... FAILED
test the_saved_codename_is_offered_on_the_next_boot ... FAILED

---- creating_bond_from_a_fresh_boot_saves_it_to_the_card stdout ----
thread 'creating_bond_from_a_fresh_boot_saves_it_to_the_card' panicked at
  crates/nf-view/tests/create_save_live.rs:72:5:
the ✓ accepts [M 0x213a98:45-65]
```

The walk to the name entry passed (the page flow works); the ✓ itself returned false. After
the fix the same tests pass, with the fe seeded through the same `ProfileSession` the window
boot now runs.

## The fix (docs/systems/profile-session.md has the full evidence)

The game keeps the codename session in plain globals OUTSIDE every menu manager — the save
slot `DAT_002e1ae0` and the working codename buffer `DAT_002e1ae8`, written by the ✓
[M C_KEYBOARD 0x213a98:53-63, decompile read] and read by the results' save gate and the
medal line's sprintf [M P_NFRESULTS 0x208e78:30/145-152/194, decompile read]. The port
models one manager as a `FrontEnd`, so the session must ride the window across every
create/drop — the music statics' pattern (menu-audio-state), now applied to the session:

- `menu::ProfileSession { dir, name, profile }` + `from_fe` (the stash side) +
  `apply_to` (the create side, which re-enumerates the card [M Menu_EnumSaves 0x1f7b78]).
  `App::stash_profile_session` (non-clobbering: a dirless fe never erases the standing
  session) runs in `menu_after` and at every `fe = None` site (the pause resume,
  `close_front_end`, the mission-load drop, the handovers' `fe.take()`).
  `App::inherit_profile_session` runs at every create: the boot, the gameplay pause's
  `get_or_insert_with`, the MP debriefing's re-create, `mp_results`, `demo_results`,
  `net_fail`.
- The boot seeds the card from `settings::config_dir()` — `%APPDATA%\NightfirePort` on
  Windows (`NF_CONFIG_DIR` overrides) [S the port's card-dir stand-in, the same directory
  the settings file already uses] — and the validator's reserved names from the two
  def_codename labels through the language bank: 0x29b "New Codename", 0x211 "Default"
  [M Menu_IsValidCodename 0x1f9930 strstr's the uppercased name against them; the strings
  [D USATxt.dat]]. (`cn_reserved` was also only ever populated by tests.)
- `menu::cnname_char_key(KeyCode) -> Option<u32>` maps a typed letter/digit to the grid
  CELL ID (uppercase byte), and the window's new CnName branch presses it through the same
  `cn_press` the grid uses [M 0x213a98 msg 0x4b: the cell types the first byte of its own
  text] — so `del_when_keypressed`'s first-press clear (the proposal "BOND" clears, the
  pressed letter appends; only the space CELL is swallowed [M :18-27, decompile read]) and
  the 8-char cap [M :75-77] stay in one place. Backspace maps to the ← cell 1000 [M :39-42].
  Space stays Confirm (the game's X presses the focused cell); arrows/Enter/Esc/Tab keep
  `key_action` (Enter presses the ✓, which holds the focus on open [M 0x213790:47-48]).
- `demo_results` inherits the session AND runs `fe.sync_profile_progress(&sim.drones.progress)`
  after `results_enter` — the game writes its MSSN/BNUS from the same live state the results
  page reads [M PlrStat_GetScore's DoneBetter commit rides the enter 0x1984b0:301-308;
  Menu_UpdateMessageBox case 1 then saves it].
- `nf_game::profile::Profile` gained `#[derive(Clone)]` (the handovers copy the loaded
  chain; all four block structs already derived Clone).

Nothing was renamed or repurposed; every new field/const sits on its own line; no shared
file was reordered. The draw code is untouched (the main agent draws every page); this task
delivers the data, state, flow and getters the draw code already reads (`cn_typed_name`,
`cn_cursor_cell`, `card_message`, `rows`).

## How it was verified

- `crates/nf-view/tests/create_save_live.rs` — 4/4:
  1. `creating_bond_from_a_fresh_boot_saves_it_to_the_card` — the create through the real
     action path on a boot-seeded card: BOND.profile on disk, `active_profile = BOND`, the
     flow lands on the wheel 0x4000001d [M 0x213a98:58].
  2. `the_saved_codename_is_offered_on_the_next_boot` — the session stash → a fresh fe →
     rows() = 3 → row 2 loads to the difficulty page (the restart half of the pin).
  3. `typed_letters_reach_the_grid_and_the_name_saves` — typed B-O-N-D over the proposal
     (the first press clears [M :18-27]), erase to empty via Backspace [M :39-42], type
     "JAMES0", the seventh/eighth fit and the NINTH is refused [M :75-77] (sister), the ✓
     saves the uppercased JAMES0XA.profile.
  4. `finishing_a_mission_under_the_codename_saves_progress` — the stash → the results
     handover's fresh fe → results_enter + the tally merge → the save prompt through the
     real confirms → ASK_OVERWRITE (the codename exists [M 0x10ee38]) → Yes → the card's
     file decodes with `nightfire_status == 0b111` (the boot 0x3 + row 1's award bits
     [M 0x1ffa10:62-66; save_progress_roundtrip's anchors]).
- Regressions, all green: `codename_entry` 14, `memcard_save_flow` 7,
  `save_progress_roundtrip` 2, `options_persist_roundtrip` 3, `menus_flow` 42,
  `front_end_chain_live` 2, `pause_results_flows_live` 4, `fix_save_profile_live` 2,
  `menu_graph_walk` 2 (the BFS invariant walk), `menus_e2e` 4 + `menu_sp_e2e` 9 +
  `menus_backout_after_character` 2 + `menu_movies` 5, `nf-game --lib profile::` 6,
  `nf-formats --lib` 79, `nf-audio --lib` 79.
- The full four-crate suite (tools/test/suite.py's per-binary shape, run as resumable
  batches because this sandbox reaps background jobs — every test binary of
  nf-game/nf-formats/nf-audio/nf-view ran with `--test-threads=1`):
  **2012 passed, 6 failed, 73 ignored.** The 6 failures:
  - `disc_golden` 3 — allowed on the slim ISO (the task's Rule 5).
  - `fix_0c_pylon_live` 2 + `probe_pylon_render` 1 — both panic at their
    `request_adapter().expect("adapter")` (wgpu `NotFound { active_backends: 0x0,
    supported_backends: VULKAN|GL }`): this sandbox has no GPU adapter. The tests never
    construct a `FrontEnd` and touch none of this task's code; they need a rerun on a
    GPU machine (the main agent's environment) to confirm green.

## What's left

- The in-mission pause → quit-to-front-end path inherits the session through the same
  `ProfileSession` seam, but no test walks the FULL window loop (pause fe → mission end →
  results) headlessly — `App` needs a winit window to construct, so the handover seams are
  tested at their boundaries (from_fe/apply_to + the results flow) instead. A
  `pause_results_flows`-style test driving the App's fe plumbing end-to-end would close it.
- The wheel's "Save Codename" row and the ~X Delete flow were already ported and tested
  (codename-entry doc); nothing added there.
- The cheat entry (Secret Unlocks) still creates nothing (reported, pre-existing).

## Live checks for the main agent

1. On a FRESH boot (no save-state resume), Select Codename → New Codename, type a name with
   the keyboard and confirm: the ✓ should save and land on the Secret Unlocks wheel. Address
   to watch if it misbehaves: `DAT_002e1ae0`/`DAT_002e1ae8` (0x2e1ae0/0x2e1ae8) — the ✓
   (C_KEYBOARD 0x213a98:53-54) writes 999 + the name; a gdbwatch on 0x2e1ae0 (write) during
   the confirm should catch the 999 store.
2. With the port, create a codename, then check `%APPDATA%/NightfirePort` holds
   `<NAME>.profile` next to `settings.toml`, and the next boot's Select Codename lists it as
   row 2 (a render of `--page cnselect` on a card with one codename should show the name).
3. Finish a mission under the codename: the results page's Confirm should open "Do you want
   to save your progress?" and Yes should update the file's mtime (gdbwatch: the port's
   `save_profile_kind` write; the game's LS_Save 0x1cd698).

## Usage

not visible to me — this sandbox's CLI shows no token, cost or context-window figures.
