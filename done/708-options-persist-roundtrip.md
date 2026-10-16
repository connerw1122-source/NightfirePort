## Your task: `options-persist-roundtrip` — the options survive a restart (651 end to end)

651 (settings-persist) added the config file. Prove it by the real path:
- change every Advanced / Multiplayer / AV option row through the pages' Left/Right and Confirm;
- drop the FrontEnd;
- a fresh FrontEnd with the same config dir shows the same values;
- the AV volume rows reach the sound layer's volumes at boot;
- Back on a page throws the edits away (they're not saved).

Unknown keys and out-of-range values in the file are tolerated (clamped). Use `NF_CONFIG_DIR` / a temp dir, never
the user's.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/options-persist-roundtrip`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-16T17:37Z

---
## Report (done, 2026-10-16T18:58Z)

# options-persist-roundtrip — the options survive a restart (651 end to end)

**done, 2026-10-16T19:1xZ (branch `s/options-persist-roundtrip`)**

## What's done

651 (settings-persist) was **not merged into master** — its config file lived only on
`origin/s/settings-persist`. This task merged that branch in (601733f0: `settings.rs`,
its 13 tests, the docs; the three lib.rs conflict blocks resolved keeping both sides —
the frontend-music statics AND the settings load) and proved the file end to end by the
real input path:

1. **Every Advanced / Multiplayer / AV option row is changed through the pages' Left/Right
   and the page Confirm** — no `fe.page`/`fe.cursor` writes anywhere. The walk is actions +
   ticks only: Start → Main → NightFire → New Codename → the name entry (QWERTY typed, ✓
   holds the focus on open [M P_CNNAME 0x213790:47]) → the card flow completes into the Edit
   wheel (cursor 1 [L PCSX2 m30]) → wheel rows 3/4/5 [M C_SBCNOPTIONS 0x213528:32-60] →
   CnAdvanced/CnMp/CnAv. Every value row moves on Left (asserted per row against its
   pre-step value); sliders step twice so the music edit (80 → 70 → 60) can't land on the
   driver's own boot volume (70) by accident. Action rows are stepped too and asserted to
   change nothing. Each page's Confirm closes to the wheel and raises
   `FrontEnd::settings_save_pending` [M menu.rs, the option pages' Confirm/Back arms].
2. **The host drain** — the tests replicate `App::menu_after`'s settings arm exactly: the
   pending flag writes `settings.toml` through `settings::default_path()` (the
   `NF_CONFIG_DIR` route; a temp dir, never the user's).
3. **Drop the FrontEnd → a fresh FrontEnd on the same config dir shows the same values**:
   `drop(fe)` then `FrontEnd::new()` + `settings::load_default()` — the full option map is
   equal to the dropped front end's (file keyed by the rows' label ids per page section, so
   nothing scrambles).
4. **The AV volume rows reach the sound layer's volumes at boot** — this wiring was MISSING
   (nothing consumed 0x1000_0135/0x1000_0134 outside the menu model). Added
   `nf_view::apply_av_volumes` (lib.rs), called at the windowed `--fe` boot right after the
   option file loads onto the fe, and on the CLI-mission boot (the `else` arm): the driver's
   `music_master`/`master` and the menu-music model's `music_master` take the rows' values.
   Game evidence: the settings load applies the persisted volumes straight to the IOP
   [M LS_LoadGlobalSettings 0x1cdbc8:69-70 `SFXMusicSetVolume/SFXSetVolume`;
   Boot_LoadPTPData 0x1c8560:81-82]; the driver's fields are those statics —
   `ES_CombineVolumes` 0x1300 scales jingles by `music_master`, everything else by `master`
   [M nf-audio/src/sfx.rs `combine`; SFXSetVolume clamps 0..=100, SFX.IRX 0x11d0]. The
   option map stores the volume scale the restore-defaults list writes [M P_CNAVOPTIONS
   0x214818:97-110 — slider 0xe → volume 70], so rows map 1:1 onto the driver fields.
5. **Back on a page throws the edits away**: Back restores the enter snapshot [M the ~B
   Back hint], the save fires with the RESTORED values, and a fresh boot reads the
   pre-edit defaults — the edits are in neither the map nor the file.
6. **Unknown keys and out-of-range values tolerated (clamped)**: `0xffff`, `radar`, an
   action row's label and a newer port's `[window]` section change nothing; 9999 → the
   handicap's +100 end; 250/-7 clamp to the sliders' 100/0 ends (651's own 13 tests cover
   the rest: missing/corrupt file, reordered rows, the i32::MIN overflow guard, the ADV/MP
   Auto Aim label collision, `NF_CONFIG_DIR` routing).

## Verification

- New: `crates/nf-view/tests/options_persist_roundtrip.rs` — 3 tests, all green
  (`every_row_roundtrips_through_left_right_confirm_and_the_restart`,
  `back_on_a_page_throws_the_edits_away`,
  `boot_applies_clamped_file_volumes_to_the_sound_layer`). They serialize on a mutex so a
  bare parallel `cargo test` is as deterministic as the suite's `--test-threads=1`.
- **The crates the tests touch, before pushing** (all `--test-threads=1`, final tree):
  `nf-view` binaries `settings_persist` 13/0, `menus_e2e` 5/0, `codename_entry` 14/0,
  `screen_adjust` 9/0, `menu_sfx_flow` 14/0, `menu_music_flow` 14/0, `menu_audio_flows`
  9/0, `menus_flow` 42/0, `memcard_save_flow` 7/0, `fix_save_profile_live` 4/0,
  `pause_results_flows_live` 9/0, `options_persist_roundtrip` 3/0 — 143 passed, 0 failed.
- **The full fast suite on the final tree (merged with origin/master c10b530f)**, foreground
  in six chunks (this sandbox kills background jobs between tool calls — the task's nohup
  start died twice with empty output, the known pattern):
  **1908 passed, 7 failed, 72 ignored over 436/437 binaries** (`nf_cpd_child` is the guard's
  child runner). No TIMEOUT, no CRASH. The 7 failures are exactly the known environmental
  set of this sandbox: 3 × `disc_golden` (slim-ISO license), `driving_ai_follow` (slim-ISO
  data 360/811), `fix_0c_pylon_live` ×2 + `probe_pylon_render` (no GPU — wgpu
  `adapter: NotFound`). Zero real regressions.
- Each chunk ran with `NF_SILENT=1` set by the runner; NF_ISO pointed at the slim ISO.

## What's left

- Nothing for this task. (The windowed host doesn't wire `set_profile_dir` yet — the card
  is the memcard-save-flow tasks' surface; my tests point it at the config dir themselves,
  the documented host rule.)
- Live checks for the main agent (optional):
  - Boot nf-view `--fe` with `NF_CONFIG_DIR` pointing at a dir holding `[av]\n0x0194 = 30`
    and confirm the menu music is audibly quieter (driver.music_master = 30), then raise it
    in the file and restart.
  - On the AV page, drag the music slider and confirm the menu music volume changes live —
    the game applies the slider on the page's 0x50 arm [M 0x214818:71-74]; the port applies
    the rows at boot only (the live slider step is a natural follow-up, `MenuMusic::music_slider`
    already exists but takes the game's 0..20 units while the port's option slider is 0..100
    volume-scale — the two scales need reconciling first).

## ntfy
sent (see the driver's notification receipt).
