# Settings persist — the PC port's options file

The port keeps the front-end option pages' values between runs in a small TOML file — the PC
stand-in for the PS2 memory card's global settings. Port: `crates/nf-view/src/settings.rs`
(the file model), `FrontEnd::settings_save_pending` (`crates/nf-view/src/menu.rs`), the host
drain in `App::menu_after` and the three front-end boots (`crates/nf-view/src/lib.rs`). Tests:
`crates/nf-view/tests/settings_persist.rs`. Tags: [M] code (function@addr), [D] disc,
[L] EE snapshot, [S] suggested.

## 1. What is persisted, and what is not

The three codename option pages' value rows [M `P_CNOPTIONS_Handler` 0x213f08:24-71,
`P_CNMPOPTIONS_Handler` 0x2144d8:13-40, `P_CNAVOPTIONS_Handler` 0x214818:37-60; the port's
tables `ADV_ROWS`/`MP_ROWS`/`AV_ROWS` in menu.rs]:

- **Advanced** (8 rows): Vibration, Auto Aim, Crosshairs, Crouch, Manual Aim, Weapon Auto
  Switch, and the two 0x226/0x23b rows.
- **Multiplayer** (3 rows): Radar, Health Handicap, Auto Aim.
- **Audio/Video** (6 value rows of 10): the two volume sliders, Subtitles, Split Screen,
  Speaker, Widescreen. The page's four action rows (Screen Adjust, Restore Defaults, Credits,
  the trailer) carry no value and are never written.

**Game progress is NOT in this file.** On PS2 the split is the same shape: the option globals
ride the save's global-settings blob (`LS_MakeGlobalSettings` 0x1cd9d0 — the screen offset too
[M 0x1cd9d0; docs/systems/screen-adjust.md §3/§5]) while campaign progress rides the save-game
records the front end enumerates (`Menu_EnumSaves` 0x1f7b78). This port keeps exactly that
split: **progress stays in the save-game/profile model (task 635-memcard-save-flow's
`Profile`, the `profile_dir` flow, `save_profile`); `settings.toml` is settings only** and
nothing in it can advance a campaign, unlock a reward, or touch a profile. The Screen Adjust
offset (`screen_x`/`screen_y`, the game's `PS2ScreenXPos` 0x30c75c/0x30c760) is written by the
AV page's Screen Adjust subscreen and saved in the game's global settings blob — the port
persists the option rows the pages show; the screen offset itself is not (yet) in the file —
listed in §6.

**Window settings: nothing to persist yet.** The task's "also persist the window settings the
port already has, if they exist as state" — they don't: the main window is created with
winit's default attributes (lib.rs `resumed`, `Window::default_attributes().with_title`), and
the port has no size/fullscreen state anywhere. When such state lands, its rows belong in this
file's `[window]` section (the parser already skips unknown sections — §4).

## 2. The file

`settings.toml` in the user's config dir:

- `NF_CONFIG_DIR` set (tests, sandboxes) → that directory;
- Windows → `%APPDATA%\NightfirePort`;
- else `$XDG_CONFIG_HOME/NightfirePort`, or `$HOME/.config/NightfirePort` (std only — the
  workspace has no `dirs` crate and got none [S the task's rule]).

Shape (a TOML subset written and read with std only):

```toml
# NightfirePort options (settings-persist). Keys are the game's row label ids ...
[adv]
0x0196 = 1     # Vibration — the row's LABEL id, hex
[mp]
0x0296 = 50    # Health Handicap
[av]
0x0194 = 80    # the music slider
```

**Keys are label ids per page section, never indexes** — the game reads each row's value back
by the row's label (msg 0x35, the DAT_002a38d1… profile bytes [M 0x213f08:24-71]), and label
ids are the stable handle a reorder can't break. The section split matters: ADV and MP both
have an Auto Aim row with label 0x197 (ctrls 0x1000_011b vs 0x1000_0129 — two different
settings in the game [M 0x213f08 / 0x2144d8]); `[adv] 0x0197` and `[mp] 0x0197` stay separate.

## 3. Load (boot) and save (page exit)

- **Load.** Every windowed front-end boot applies the file over `default_opts()`: the boot
  (lib.rs, where `FrontEnd::new()` runs for the front end), the Esc-pause fresh front end,
  the `net_fail` fresh front end (a `fe = None` recovery path must not show bare defaults)
  and the F8/F9 demo-results front end (its values would otherwise ride the results page's
  quit back into the front end). The headless `--fe <page>` render path (`front_end_menu`)
  deliberately does NOT load — screenshots must not depend on the user's config file (the
  render checks, WORKLOG lesson 6). Missing file → defaults + `[settings] no settings file
  at …` on stderr; corrupt → defaults + `[settings] corrupt settings file …` on stderr.
- **Save.** Leaving CnAdvanced/CnMp/CnAv through **Confirm or Back** raises
  `settings_save_pending` [S the game's own commit/cancel pair: ✕ keeps the edits, ○ restores
  the page-enter snapshot (the opts_saved restore, the hint's ~A Confirm / ~B Back; the msg
  0x4d restore [M 0x214818:79-85]) — both leave the page, and both leave the option globals
  in a state worth writing]. Back saves the RESTORED values (idempotent normally; repairs a
  file that was missing/corrupt at boot). The host drains the flag once per action in
  `menu_after` (the same model-pure split as `secret.pending_submit`) and writes the file;
  a failed write logs `[settings] …` and the game goes on. The in-page edits (spinner steps,
  Restore Defaults 0x100001a3 [M 0x214818:96-110]) save when the page is left, matching the
  game: the reset list only rewrites the globals and stays on the page.

## 4. Corrupt vs unknown; clamping

- **Corrupt** (defaults + warning): a line that is neither blank, a `#` comment, a
  `[section]` header nor `key = integer` (a trailing `# comment` after the integer is fine);
  a non-integer value; a key before any section.
- **Unknown** (ignored, no warning): a key matching no current row label in that section; a
  key for an Action row; **a section the port doesn't know** — a NEWER port's `[window]` or
  `[net]` must not make an older port throw away the user's whole file, so unknown sections
  and their keys are skipped. (One caveat, the price of std-only writing: the next SAVE
  rewrites the file from the port's own model, so an unknown section's keys don't survive a
  save — an older port saves what it knows.)
- **Clamping**: a value not in the row's legal set snaps to the NEAREST legal value (ties →
  the lower), so ON/OFF rows always land on a legal side and the Speaker row's {0,1,4} maps
  2→1, 3→4. Sliders clamp 0..=100 — the same clamp the page's own steps use [M `opt_step`'s
  `(v + 10*dir).clamp(0, 100)`; the controls' d3 0x64 = 100, P_CNAVOPTIONS 0x214818:37-60].
  A clamped value is written back on the next save, so the file self-heals to legal values.

## 5. Tests

`crates/nf-view/tests/settings_persist.rs`, all on temp dirs (or `NF_CONFIG_DIR`) — never the
real user dir: the real-input-path round trip (CnMenu wheel row 3 → CnAdvanced → a spinner
step → Confirm → save → fresh boot reads it back), missing file, corrupt file (3 shapes),
unknown keys/sections, reordered rows (a scrambled-order file reads back onto the same
controls), the changed-value sister assertion, clamping, back-out (restored values are what
get written), the ADV/MP 0x197 split, mkdir-on-save, and the `NF_CONFIG_DIR` override through
`load_default`/`save_opts`.

## 6. Not done / follow-ups

- The Screen Adjust offset (screen_x/screen_y) and the window size/fullscreen (once the port
  has such state) belong in this file; both are listed here, neither is in it.
- `default_path()` re-reads the env on every call (no cache) — deliberate, so a test can set
  `NF_CONFIG_DIR` at any time; the host calls it once per action at most.
