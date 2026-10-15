# subtitles-data — the Subtitles option's data path, ported

Branch `s/subtitles-data`. Everything here is [M] decomp/disassembly read in `work/decomp/ACTION.ELF`
unless tagged otherwise. No drawing was added — the main agent draws every page; the port delivers the
data, the state, the flow and the getter (`SubtitleTrack::active_subtitle(t)`).

## What the option switches

The AV options' Subtitles row (control 0x10000136, `P_CNAVOPTIONS_Handler` 0x214818) writes the profile
byte `GameState+0x60` = **DAT_002a37c8** (the port's `Profile::gset.subtitles`). Three game readers
switch on it:

1. **`Text_AddMsg` (0x1b0c30)** — the gate: a type-4/5 message passes only when `DAT_002a37c8 != 0`,
   except the id-0xff bypass `Sound_DoSubtitle` uses after its own gate. Disasm-checked
   (`bVar1 || (DAT_002a37c8 != 0) || (1 < type − 4U)`).
2. **`Sound_DoSubtitle` (0x1dde58)** — the drone barks' spoken lines (the `Snd2Lbl` table, the
   0x80000000 "always show" escape, duration = `g_fSubtitleDurationModification × SFXOutputData[id].f10 × 60`).
   Documented, not ported (the port's drones do not render bark text).
3. **`ResetMap_Load` (0x1d6360) phase 0** — whether the FMV briefing's subtitle ScriptPlayer is created.

## What was delivered

**The data model** — `crates/nf-game/src/subtitle.rs`:

- `SubtitleTrack { enabled, lines }`: the game's TXT_MSG queue as an absolute timeline of
  `SubtitleLine { label, start, end }`. `push(now, label, time)` mirrors `Text_AddMsg(-1, 0, 4, …)`:
  the flag gate (Off → dropped), the **verbatim u16 duration** (disasm 0x1b104c-0x1b10ac: the
  `max(60, …)` clamp at 0x1b1060 skips types 4/5 — `bnel v0,zero,001b10b4` at 0x1b1050 — and with one
  word-wrap part the length ratio is 1), the FIFO chain (start = `max(push, previous end)` —
  `Text_UpdateMsg` 0x1b11d8 runs only the displayed head's timer, so a line pushed while a head shows
  waits), and a 0-time head never displays (deleted on the next update).
- `active_subtitle(t) -> Option<label>` — the per-tick query the draw code calls, for both kinds.
- `flush(now)` — `Text_FlushAllSubtitles` (0x1b15c8, `Script_FFwd`'s skip arm 0x1dac78:28-29): the
  displayed head stops at `now`, every queued line dies.
- `from_script(nis, enabled)` — the FMV kind: walks a script's Wait + Text commands on `Script_Run`'s
  clock (a command after `Wait(t)` releases on tick t+1 — `Script_Update` 0x1da998 runs the stream with
  the pre-increment clock, then adds `FRAME_RATE_MUL` 1.0 [L]).
- `intro_script_for(movie_code)` — the movie-code → script chain [M 0x1d6360:40-99, no jump table
  [L]]: 0x7100001→0x060095a, 05→961, 09→963, 0c→965, 11→967, 14→969, 18→96b; 0x7300004→0x060095d,
  08→962, 0b→964, 0d→966, 13→968, 17→97a, 1b→96c. 0x710004a has no row (level 0700004a gets no intro).

**The FMV briefing flow** — `crates/nf-view/src/movie.rs::briefing_subtitles(iso, code, enabled)`:
reads the global pack `07000500.bin` from FILES.BIN, parses the code's script, returns the track. The
game creates the ScriptPlayer (`SP_Create(..., script, 0, 1, 1, 0, 0)` — mode 1 = play at once) while
`psiStartBackgroundMovie` plays the briefing, so the track's tick axis is the movie's.

**The NIS cut-scene wiring** — `drone.rs`: `Drones::subtitle_track` (new field, own line); the
`Request::Text` arm pushes at the script tick and — matching the game's AddMsg gate — fills
`script_msgs` (the HUD drain) only when the flag is on; `Request::TextFlush` flushes the track.
**The host must wire `subtitle_track.enabled` from `Profile::gset.subtitles` / the AV row**: the fresh
profile's default is Off [M profile.rs], so nothing shows until it is wired. The draw code then reads
`sim.drones.subtitle_track.active_subtitle(sim.drones.frame)` per tick and binds the label through the
Lang table (`Txt_BindLabel` 0x1cad60; the label ids live in the op stream — the game stores the bound
string, the port binds at draw time like `drones.taken`).

**Who draws them** (docs/systems/subtitles.md has the full layout): `Game_Run` (0x1c9790:20-24) picks
the presenter per tick — `ScriptCam != 0 || movie playing` → `Text_Update` (0x1b1510, type 4, one
line, `SubTitleSpr` from the SubtitlesInfo template at 0x319940), else `Text_Update2Line` (0x1b09f0,
type 5, two lines at x 256 / y 402 and 422, colour 0x7d6d59ff, the centre-justify fmt 0x30cdb0, wrap
widths 276/460 px from `TextMsgFormats[5]` [L .data 0x2bf108: entry k = {FMT* fmt; u16 width} —
entry 4 {0x30c6b0, 480}, entry 5 {0x30c6b0, 460}]). A type-4 add while the movie has finished is
retyped 5 (0x1b0c30's `psiMovieFinished` arm), so the remaining briefing lines continue on the 2-line
presenter.

## How it was verified

- **6 unit tests** (nf-game `--lib subtitle`): FIFO chaining, the verbatim duration (no 60 floor), the
  0-time head, Off → nothing, flush, `from_script`'s Wait → t+1 walk. 212/212 nf-game lib green.
- **`crates/nf-view/tests/subtitles_live.rs`** (ISO-gated, 4):
  - `briefing_track_is_pinned_to_the_disc_script`: 0x06000961's full timeline read from the disc [D]
    and pinned — ticks **184/1340/2113/3570**, labels **0x06000068/69/6a/6b**, times
    **1132/756/988/555**; the first pair matches the independently-recorded bytes
    `1e 68 00 00 06 6c 04 1e` after `Wait(183)` (fix-script-commands). Sister assertions: 183 dark,
    1339 (the script's own gap) dark, 4125 dark.
  - `codes_without_a_table_row_build_nothing` (0x710004a → None; 0x07100009 resolves).
  - `subtitles_off_drops_the_script_lines_in_the_sim`: through the real Sim with the flag at its
    fresh-profile default — no `script_msgs`, no HUD line, empty track, `active_subtitle` None.
  - `subtitles_on_queues_the_line_on_the_track_at_the_right_tick`: the ON run — the track active at
    the script tick, the HUD drain consumed the 1132-tick line.
- `fix_script_commands_live::text_command_queues_the_subtitle_label` now sets the flag explicitly and
  pins the ON behaviour (4/4 green).
- **Suite runs** (tools/test/suite.py, `--test-threads=1`, NF_ISO set):
  - nf-game lib **212/212**; nf-formats + nf-audio green except the **3 disc_golden** tests
    (bigf_header/bigf_tables/directory_tree — the documented slim-ISO set).
  - The HUD/message-affected view binaries (`fix_hud_live`, `fix_objective_banner_live`,
    `fix_objective_messages_live`, `fix_pickups_ammo_live`, `fix_pickups_trace_live`,
    `hud_playtest_live`, `screen_adjust`): **34/34**.
  - The suite runner over **104 further binaries** across five foreground chunks: round 1 the
    nf-view level/render set (`levels_sweep_live`, `portal_sweep_live`, `walkreal_*`, `render2_live`,
    `menu_movies`, `mp_menu_flow`, …); then `menus_flow`, `level_live`, `mp_arenas_live`,
    `objective_state`, `progress_live`, `pickup_live`, `rng_live`, `trig_live`, `tuning_live`,
    `weapon_live`, `weapon_switch_live`, `input_ramp_live`, `input_styles_live`, `facial_live`,
    `sound_coverage_live`, `searchlight_live`, `stun_grenade_live`, `grenade_live`, `doors_live`,
    `ledge_live`; `codename_entry`, `campaign_flow_live`, `cutscene_draw_live`,
    `cutscene_models_live`, `fields_player_live`, `fields_actors_live`, `driving_golden`,
    `facial2/3_live`, `dev_menu_flow`, `fire_modes3_live`, `explode_live`, `casings16_live`,
    `ai_traces_live`, `ammo_pane_live`; `fix_actors_live`, `fix_06_alarm_live`,
    `fix_aims20_lights_render`, `fix_08_collision_live/2`, `fix_08_walls_live`, `fix_0c_hatch_live`,
    `fix_0c_vent2_live`: **0 failed** (75 + 63 + 22 tests in the chunked runs alone).
  - Two failures are environmental: `fix_0c_pylon_live`'s 2 tests panic at the wgpu
    `request_adapter` (this sandbox has no Vulkan ICD — pre-existing on any branch, levels.md §6).
  - **The full 356-binary sweep did not fit the window**: background jobs are killed between tool
    calls here, and one runner invocation paid a 336 s rebuild. ~109 of 356 binaries ran (listed
    above); the unrun remainder is the rest of the ISO-gated `fix_*`/`gadget_*`/probe set — none of
    which touch the subtitle path (the affected set above is green).

## What's left

- **Host wiring of the flag**: the main agent should set
  `scene.sim.drones.subtitle_track.enabled = profile.gset.subtitles` (or the AV row's
  `opts[0x10000136] == 1`) when a level/briefing runs, and drive the briefing track from
  `movie::briefing_subtitles(iso, movie_code, enabled)` while state 6 plays the movie.
- **The draw code** (the main agent's): read `active_subtitle(t)`, bind the label, present it the way
  `Game_Run` chooses the presenter (1-line while `ScriptCam || movie playing`, 2-line otherwise); the
  type-5 geometry is pinned above, the type-4 sprite's exact screen position comes from the
  SubtitlesInfo template the font load fills (not decoded — see live checks).
- The spoken-line barks (`Sound_DoSubtitle`, source 3) — documented, not ported.
- The word-wrap part split (a long line becomes several consecutive messages in the game) — the port
  keeps one message per op; the draw code's wrapping owns the layout.

## Live checks for the main agent

1. **The type-4 presenter's position**: during a briefing (movie 07100005) with subtitles ON, watch
   `SubTitleSpr` (the pointer at 0x30cd9c) fields +0x24/+0x26 (x/y) and +0x10 (colour) while a line
   shows — read type: u16. Expected: the template at 0x319940 is .sbss-zero at rest [L], so either the
   font load writes the position or the draw uses fmt 0x30c6b0's own fields (then `Font_ParseFormat`
   0x1ee3e0's decode pins it). This decides where the port's draw code puts the 1-line subtitle.
2. **The retype arm**: start 07100005's briefing, skip the movie near the end (action 0x19), then
   watch `Text_AddMsg`'s type store (0x1b0c30's `param_3 = 5` arm at 0x1b0f00) — expected: the
   remaining lines arrive as type 5 and `Text_Update2Line` draws them after the pop.
3. **The flag write-back**: toggle the AV row (control 0x10000136) and watch `DAT_002a37c8` (u8) —
   expected 0/1 following the row; then run a briefing with it 0 — expected: no
   `SP_Create` for 0x06000961 (breakpoint 0x1d6360's `SP_Create` call at 0x1d63xx) and no type-4 adds.
4. **A drawn comparison spot** for the 2-line presenter: during 07100005's intro at tick ~1400 the
   line 0x06000069 shows centred at (256, 402)/(256, 422) in the 512×448 HUD space — the render at
   `--movie 07100005 --frame 700` (≈ tick 1400) should show the line's second row at y 422 when the
   text wraps past 276 px.

## ntfy failed

All three fallbacks ran at 2026-10-15T22:39Z; the daily message quota was already exhausted by the
earlier session's notifications:

- curl: `{"code":42908,"http":429,"error":"limit reached: daily message quota reached; increase your limits with a paid plan, see https://ntfy.sh","link":"https://ntfy.sh/docs/publish/#limitations"}`
- python3 urllib: `urllib.error.HTTPError: HTTP Error 429: Too Many Requests`
- wget: exit 0, empty body (ambiguous — the quota 429 page; no message delivered beyond the earlier ones)
