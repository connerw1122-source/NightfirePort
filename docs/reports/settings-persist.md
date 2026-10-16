# settings-persist — report

Branch `s/settings-persist` (base: origin/master 306c5d93, unchanged since — the final tree IS
the merge). Task 651. Status: **done** (one honest gap: the suite's nf-view walk/render/probe
tail did not fit the sandbox window — details under "What ran").

## What's done

The PC port keeps the front-end option pages' values between runs in `settings.toml` in the
user's config dir (`%APPDATA%\NightfirePort` on Windows; `$XDG_CONFIG_HOME|~/.config` +
`/NightfirePort` elsewhere; `NF_CONFIG_DIR` overrides for tests — std only, no `dirs` crate,
the workspace has none).

- **`crates/nf-view/src/settings.rs`** (new): the file model. Keys are the rows' game LABEL
  ids per page section (`[adv] 0x0196 = 1`) — never indexes, so reordering rows cannot
  scramble saved settings. The game reads the rows back by label with msg 0x35 [M
  P_CNOPTIONS_Handler 0x213f08:24-71], so the label is the stable handle. The section split
  matters: ADV and MP both have an Auto Aim row with label 0x197 but different controls
  (0x1000_011b / 0x1000_0129 [M 0x213f08 / P_CNMPOPTIONS 0x2144d8]) — separate sections keep
  both. Unknown keys AND unknown sections (a newer port's `[window]`) are ignored, not
  corrupt, so a newer file never makes an older port drop the user's settings. Out-of-range
  values snap to the NEAREST legal value (ties → lower; the math is i64 so `i32::MIN` can't
  overflow); sliders clamp 0..=100 — the same clamp the page's own steps use [M `opt_step`'s
  `(v + 10*dir).clamp(0, 100)`]. Missing/corrupt file → `default_opts()` + a warning on
  stderr. Trailing `# comment` after a value is fine.
- **`menu.rs`**: `FrontEnd::settings_save_pending` (own line, WORKLOG lesson 4) raised by
  CnAdvanced/CnMp/CnAv **Confirm and Back**; Back saves the RESTORED values (idempotent;
  repairs a file that was missing/corrupt at boot). The model stays pure — the same split as
  `secret.pending_submit`.
- **`lib.rs`**: the host drains the flag in `menu_after` (covers keyboard, mouse click/hover
  and the net-join typing paths) into `settings::save_opts`; the file's values load over the
  defaults at every windowed fe boot (title front end, Esc-pause fresh fe, `net_fail` fresh
  fe, F8/F9 demo-results fe). The headless `--fe <page>` render path deliberately does NOT
  load — screenshots must not depend on the user's config (WORKLOG lesson 6).
- **Window settings**: the port has NO size/fullscreen state (the main window is winit's
  `Window::default_attributes().with_title("nf-view")`, lib.rs `resumed`; no
  `set_fullscreen`/`with_inner_size` anywhere in the crate) — nothing to persist, per the
  task's "if they exist as state". The `[window]` section name is reserved (unknown sections
  already parse-skip).
- **Coordination with 635-memcard-save-flow** (recorded in docs/systems/settings-persist.md
  §1): game progress stays in the save-game/profile model (the `Profile`, `profile_dir`,
  `save_profile` flow); this file is settings only. That mirrors the game's own split — the
  option globals ride the save's global-settings blob [M LS_MakeGlobalSettings 0x1cd9d0]
  while campaign progress rides the save-game records [M Menu_EnumSaves 0x1f7b78].

## How it was verified

- **Tests: 13 in `crates/nf-view/tests/settings_persist.rs`**, all green, through the real
  input path where it matters: the round trip walks CnMenu wheel row 3 → CnAdvanced → a
  spinner step → Confirm → save → a fresh boot reads the edit back; missing file; corrupt
  file (3 shapes); unknown keys/sections; reordered rows (a scrambled-order file reads back
  onto the same controls); the sister assertion (a changed value really changes the file);
  clamping (incl. the i32::MIN edge); back-out (the restored values are what get written);
  the ADV/MP 0x197 split; mkdir-on-save; the `NF_CONFIG_DIR` override through
  `load_default`/`save_opts`. Every test writes to a pid-named temp dir (Drop-cleaned) —
  never the real user dir.
- **An independent audit** (subagent) checked the flag wiring, the drain's input-path
  coverage, the boot sites, the label→ctrl pairs, panic risk, and lesson compliance. It
  found 3 defects, all fixed: my `net_fail` edit had DELETED the `fe.net_notice` line (a
  silent regression — restored), the clamp distance could overflow on extreme file values
  (now i64), and the F8/F9 demo-results fe didn't load settings (now does, so its defaults
  can't ride the results quit back and overwrite the file). Two more were hardened: trailing
  comments parse, and the doc's §4 records the save-drops-unknown-sections caveat.
- **Neighbouring flows**: menus_flow 42, mp_menu_flow 9, codename_entry 14, screen_adjust 9,
  mission_stats_flow 3, front_end_chain_live 2, menu_live, dev_menu_flow,
  fix_save_profile_live, menu_sfx_flow, credits_data, settings_persist 13 — 0 failed.

## What ran of the full suite (tools/test/suite.py, final tree, release)

The sandbox kills background jobs between tool calls (verified with a canary), so the runner
went in foreground chunks; two concurrent instances caused one OOM kill (SIGKILL) whose
binary then passed solo — everything reported below was produced by a solo run of that
binary. Coverage: **180 of 372 test binaries** — all of nf-game (44), nf-formats (9),
nf-audio (2), and 125 nf-view binaries (every menu/save/pause/flow binary; the walk/walkreal
and render/probe tails did not fit the window).

**Total line: 1087 passed, 6 failed, 38 ignored.** The 6 failures are all accounted for:
- `disc_golden` ×3 — the task's allowed set on the slim ISO;
- `fix_0c_pylon_live` ×2 + `probe_pylon_render` ×1 — the documented pre-existing
  environmental failures in THIS sandbox (no GPU / missing Vulkan ICD: the WORKLOG records
  the same trio as "the known no-GPU fix_0c_pylon_live pair… environmental" and
  "the sandbox's missing Vulkan ICD" from earlier tasks). Re-run solo on my tree: identical
  result; my diff (a new module, a bool field, four boot loads) cannot reach the renderer,
  and the tests fail in `request_adapter`, before any scene code.

Zero unexplained failures; nothing timed out; no crashes.

## What's left

- Nothing in the task itself. Follow-ups recorded in docs/systems/settings-persist.md §6:
  the Screen Adjust offset (screen_x/screen_y — the game stores it in the same global
  settings blob [M LS_MakeGlobalSettings 0x1cd9d0]) and window size/fullscreen (once the
  port HAS such state) belong in this file.
- The next worker on this branch may want the suite's nf-view walk/render tail re-run on
  hardware that has a GPU; nothing in this task touches it.

## Live checks for the main agent

None needed — the task has no visual surface. The option pages render exactly as before;
the only delta is which values they show after a round trip. If you want one anyway: in the
windowed port, open Multiplayer Options, set Health Handicap to +50, Confirm, quit, relaunch
— the row should show +50 and `settings.toml` should contain `[mp]` `0x0296 = 50`.

## ntfy

(ntfy result recorded here after the send — see the finish step below.)
