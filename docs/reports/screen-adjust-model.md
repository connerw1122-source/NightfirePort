# screen-adjust-model — report

Branch `s/screen-adjust-model`. Task 617: AV Options → Screen Adjust (`P_SCREENADJUST_Handler`
0x217a58 / `C_SBSCREEN_Handler` 0x217b08), the AV Restore Defaults reset list
(`P_CNAVOPTIONS_Handler` 0x214818:96-110), and the Widescreen / Speaker / Split-screen runtime
effects, with the widescreen 16:9 view aspect implemented.

## What's done

**docs/systems/screen-adjust.md** (evidence-tagged throughout):

- The AV Options page (0x40000031) — all ten rows with their controls, label ids, y positions and
  the disc's own texts [L menu-def walk 2026-10-15 + USATxt.dat]. Corrections it records: 0x10000137
  (label 0x2b6) is the **Screen Adjust** row and opens page **0x40000047** via Manager 0x44 with
  push (bit 0 of the arg gates `Stack_Push` [M 0x21c858 case 0x44]); 0x10000138 is **Credits** (the
  port's old comment lumped it with Screen Adjust); the speaker item 0x395 is **"Surround"**, not
  "Pro Logic" [D USATxt.dat].
- The Screen Adjust page's own data [L]: two slider controls both named 0x100001eb, d1 0 = the
  vertical bar (304,176,32,128), d1 1 = the horizontal bar (256,224,128,32), both d3 0xa; the
  reference picture TEX 0x3000074 fullscreen; the hint 0x67b "~A Confirm  ~B Cancel".
- The handlers [M]: page enter (msg 0x4c) snapshots the offset into PS2OldX/PS2OldY 0x30d578/57c
  (`psiGetScreenPos`); ✕ sends Manager 0x5f with arg 1, which **skips** the close notification
  [M 0x21c858 case 0x5f's `if (param_3 == 0)` guard] so the adjustment **keeps**; the msg 0x6b close
  path restores it (`psiSetScreenPos(PS2OldX, PS2OldY)`) — the hint's Cancel. C_SBSCREEN's msg 0x49
  reads the slider value (msg 0x40) and its d1 and calls `psiAdjustScreenPos(value, 0)` or
  `(0, value)`; msg 0x51 (sent by `MenuManager_Create` for every control [M 0x21b908]) answers with
  the slider range −1..1.
- `psiAdjustScreenPos` 0x10b890 [M]: X += dx·12 clamped ±0x1b0 (±432), Y += dy·4 clamped ±0x30
  (±48), update flag set. One slider step = **12 X-units / 4 Y-units**.
- What it changes on screen [M]: `VBlankInt` 0x124088 rewrites GS DISPLAY2 as
  `(X + 0x27c) | (Y + 0x32) << 12 | 0x1bf9ff02000000` when the flag stands. The constant decodes
  against the game's own `sceGsSetDefDispEnv` 0x226370 for the game's NTSC 512×448 interlaced mode
  (`CreateDB` 0x10b000): MAGH 5, DW 5·512−1 = 2559, DH 448·2−1 = 895, DX base 0x27c = 636, DY base
  0x32 = 50 — bit-exact. Units: X is 1/5 framebuffer pixel (2560 per line), Y is a field line
  (448 per frame); the doc gives the 640×480-space conversion (X/4, Y·15/14) and the picture
  fractions (X/2560, Y/448).
- The profile [M]: `LS_MakeGlobalSettings` 0x1cd9d0 pushes music(7b), sfx(7b), language(7b),
  subtitles `DAT_002a37c8`(1b), SFX mode(32b), widescreen `DAT_002a37c4`(1b), `DrawInfo`(32b),
  screen X(32b), screen Y(32b), 219×32b zero, then 4 as 16 bits; `LS_LoadGlobalSettings` 0x1cdbc8
  pulls the same and `psiSetScreenPos(x, y)`. The offset rides the GLOBAL settings blob (not the
  per-codename file). `Boot_LoadPTPData` 0x1c8560 also `psiSetScreenPos(DAT_0031cac0, DAT_0031cac4)`.
- Restore Defaults' full list [M 0x214818:96-110]: music slider 0xe → volume 70, sfx 0x14 → 100,
  subtitles On iff `LanguageToLoad < 0 || (3 < LanguageToLoad && != 6)` (NTSC-U's language 0 →
  Off), split 0 = Horizontal, speaker 1 = Stereo, widescreen Off, `psiRestoreScreenPos` → offset
  (0,0). (Distinct from the Default profile's music 80 — both are recorded.)
- Runtime effects: **Widescreen** — `Camera_CalcViewAngles` 0x1baf70:28-32 writes
  0x3fe38e39 = 16/9 at viewer+0x114 when `DAT_002a37c4` stands, else 0x3faaaaab = 4/3, with the
  two-viewer ×0.5/×2 split factors; **Split** (`DrawInfo`) — `Camera_CreateCameras` 0x1bb2b8
  viewports, `HUD_ValidateXY`/`HUD_FixMP` DrawInfo==1 paths, `MP_Start` branch (documented only);
  **Subtitles** (`DAT_002a37c8`) — `Sound_DoSubtitle` 0x1dde58:21 gate (unless voice id bit 31),
  `Text_AddMsg` 0x1b0c30:148 class gate, `ResetMap_Load` 0x1d6360:40 movie subtitle tracks;
  **Speaker** — `SFXSetMode` 0x117078 batch command 0x1c (0/1/4), documented (the port's mixer is
  stereo-only).

**The port** (`crates/nf-view/src/menu.rs`, `scene.rs`):

- `Page::ScreenAdjust` (0x40000047) with the page's two slider rows; `action()` steps
  `screen_adjust(∓1, 0)` / `(0, ∓1)` on Left/Right, keeps on Confirm (pop to CnAv, cursor back to
  the row), restores the enter snapshot on Back — the game's ✕/Cancel semantics.
- `FrontEnd`: `screen_x`/`screen_y` (PS2ScreenXPos/YPos), `screen_saved_x/y` (PS2OldX/Y),
  `screen_av_saved_x/y` (the AV page's 0x30d8e8/ec snapshot) — each field on its own line; methods
  `screen_adjust` (psiAdjustScreenPos verbatim), `screen_adjust_open`/`screen_adjust_cancel`
  (0x4c/0x6b), `screen_restore_defaults` (psiRestoreScreenPos), `av_screen_snapshot`/
  `av_screen_restore` (0x214818:29/135/82), `screen_offset_640()` → (X/4, Y·15/14),
  `screen_offset_frac()` → (X/2560, Y/448) — **the getters the draw code calls**, and
  `view_aspect()` → 16/9 vs 4/3 from the widescreen option.
- Flow wiring: CnAv row ✕ on 0x1000_0137 pushes ScreenAdjust (snapshot on open); returning
  re-snapshots the AV baseline (the game's re-enter, 0x214818:135); a value row's ✕ commits and
  re-snapshots; CnAv Back restores the values AND the offset; ✕ on 0x1000_01a3 runs the game's own
  reset list (replacing the old default_opts pass — music 70, not 80, is the game's value).
- `Sim.view_aspect` (default 4/3) feeds `gun_matrix`'s gy term — the port's one consumer of
  viewer+0x114 — so the widescreen option now models the game's 16:9 viewer headlessly; the host
  feeds it from `FrontEnd::view_aspect()` (one line in the frame path) and passes the projection
  aspect for the 3D view.
- A minimal ScreenAdjust draw arm (picture quad 0x3000074, the two bars as the page data's plates,
  the focused bar bright, hint 0x67b) so the page renders; the main agent owns the final look
  (compared against PCSX2).
- AV_ROWS' "Pro Logic" literal corrected to "Surround" [D USATxt 0x395].

## Verification

- `crates/nf-view/tests/screen_adjust.rs` — 9 tests, **9/9 green**:
  - headless flow/number tests with sister assertions: the 12/4 steps and ±432/±48 clamps against
    psiAdjustScreenPos's constants; the 640-space and fraction conversions; the ScreenAdjust page
    keep-on-✕ / restore-on-Back flow; the AV snapshot/restore/re-snapshot discipline; Restore
    Defaults' exact reset list; `view_aspect` 4/3 ↔ 16/9 against the game's two bit patterns
    (`f32::from_bits(0x3faaaaab)` / `0x3fe38e39`).
  - ISO-gated (NF_ISO): the menu-def page 0x40000047 walk (exactly two 0x100001eb sliders, d1 set
    {0,1}, d3 0xa, the 0x3000074 picture control); the label binds from USATxt.dat ("Screen Adjust",
    "Restore Defaults", "Credits", "Surround", "~A Confirm  ~B Cancel", "Multiplayer Split Screen",
    "Horizontal"/"Vertical"); `Sim.view_aspect` 4/3 default turning the gun-tilt matrix at 16/9.
- Adjacent suites on the same tree: `menus_flow` 3/3, `menu_movies` 3/3,
  `fix_hud_zoom_tilt_live` 36/36 (the gun_matrix consumer).
- The full suite's fast runner (`tools/test/suite.py`, NF_ISO = the on-foot ISO, `--release`,
  `--test-threads=1` per binary) — **this sandbox kills detached processes between tool calls**
  (the background start at 20:03 UTC died with an empty log, the same experience the playtest-spots
  worker recorded), so the suite ran in foreground slices against the tool cap, on the FINAL tree
  merged with origin/master (the merge commit after resolving docs/WORKLOG.md and menu.rs — the
  mp-menu-flow task's menu work landed in origin/master and both were kept):
  - `--only screen_adjust,menus_flow,menu_movies,mp_menu_flow,fix_hud_zoom_tilt_live,fix_viewmodel_zoom_live,fix_interp_ghosting_live,campaign_flow_live`
    → **70 passed, 0 failed** (the mp-menu-flow binaries run green against the merged menu.rs —
    the conflict resolution kept both tasks' pages).
  - `--only dev_menu_flow,devwin_flow,…` (menu-family slice) → **13 passed, 0 failed**.
  - `--only playtest_spots_live,fix_swim_live,fix_ladder2_live,fix_restart_health_live,fix_wallhug_prompt_live`
    → **30 passed, 0 failed** (a Sim-constructor smoke over 18 levels).
  - `--only nf_formats,nf_view,nf_game,nf_audio,hud_playtest_live,hud_radar_live,textures_live`
    → **301 passed, 0 failed** (the four crates' unit-test binaries + the HUD live binaries).
  - Total across the slices: **415 passed, 0 failed**, 0 TIMEOUT/CRASH; the build step compiled all
    **351** test binaries of the four crates clean. The remaining ~330 binaries (the long
    NF_ISO per-level suites) did not fit this sandbox's window — not run, no failure implied.
- Commands run: `cargo test --offline --release -p nf-view --test screen_adjust` (9/9), the three
  adjacent test binaries, `cargo check --offline -p nf-view`, the suite slices above. All ISO-gated
  tests ran against the disc.

## What's left / notes

- The perceived DIRECTION of the offset on the TV (whether a positive X shifts the picture left or
  right — DISPLAY's DX/DY select where the GS read starts) is not decidable from the decompile
  alone; the getters expose the raw game values and both conversions, and the renderer applies the
  sign. Live check below settles it.
- The slider's per-press cadence (one 0x49 per ◀/▶ press, spring-loaded range −1..1) is the
  reading of `Scroll_Update` 0x221b20:240 + C_SBSCREEN's 0x51 range set; the live check confirms
  the on-screen step count.
- Split-screen viewports/HUD and the speaker mode's audio routing are documented, not implemented
  (out of scope).
- Credits (0x10000138) and the Die Another Day Trailer (0x10000222) still open no page (unchanged
  from before this task).

## Live checks for the main agent

1. **Screen Adjust step direction and size** — PCSX2, AV Options → Screen Adjust. Watch
   `PS2ScreenXPos` 0x30c75c and `PS2ScreenYPos` 0x30c760 (4-byte signed each) with
   `tools/trace/fieldprobe.py` while pressing ◀/▶ on the horizontal bar and ▲/▼ (or ◀/▶) on the
   vertical bar. Expected: each press changes X by exactly ±12 or Y by exactly ±4 (fieldprobe
   before/after), the drawn picture shifts by 12/2560 of the width per X step (≈2.4 px of the
   512-wide framebuffer) and 4/448 of the height per Y step, and the DIRECTION on screen (left vs
   right for +12) tells the port which sign to apply in the renderer. Also confirm the bars
   nudge per press (the spring), not every other press.
2. **The keep/cancel split** — on 0x40000047, move the bars, press ✕, back out to the Edit wheel,
   re-enter AV Options → Screen Adjust: the bars should sit at the moved position (kept). Then
   move them again, leave with ○/△ (Cancel), re-enter: they should be back at the ✕-time position
   (restored). Watch 0x30c75c/0x30c760 across each transition; expected kept = the edited values
   survive the ✕ path, restored = the msg-0x6b path rewinds them to the page-enter snapshot.
3. **Restore Defaults on the real page** — PCSX2, AV Options → Restore Defaults: music reads 70,
   effects 100, subtitles Off, split Horizontal, speaker Stereo, widescreen Off, and the picture
   re-centres (0x30c75c/0x30c760 → 0). Confirms the :96-110 reading end to end.
4. **Widescreen viewer+0x114** — with a saved profile, toggle Widescreen On, enter any level,
   `gdbwatch.py` on the active viewer's +0x114 (the ee_start_05 viewer sat at 0x1fc6c10; find the
   live one via `Camera_CreateCameras` 0x1bb2b8's `glb_viewer` 0x2a3?? slots — watch the write of
   0x3fe38e39): expected a single 16/9 write per `Camera_CalcViewAngles` when the flag stands,
   0x3faaaaab when clear. Confirms the port's `view_aspect` feed.
5. **PAL bases note** (only if a PAL boot is ever checked): `sceGsSetDefDispEnv`'s PAL DX/DY bases
   are 0x290/0x48 but `VBlankInt` writes the NTSC 0x27c/0x32 unconditionally — if a PAL disc is
   ever probed, expect the picture mis centred; the port ignores PAL (NTSC-U only).

## ntfy failed

All three fallbacks failed with the ntfy.sh daily quota (2026-10-15 ~20:20 UTC):

1. `curl -sS -m 20 -H "Title: NightfirePort" -d "screen-adjust-model: done …" https://ntfy.sh/nightfire-yoda-8k3x`
   → `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
2. `python3 -c "…urlopen(Request('https://ntfy.sh/nightfire-yoda-8k3x', …))…"`
   → `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
3. `wget -q -O- --post-data "screen-adjust-model: done …" https://ntfy.sh/nightfire-yoda-8k3x`
   → silent body, exit 0 on the first run, but the `-S` probe shows `HTTP/1.1 429 Too Many Requests`
   — the delivery did not go through.
