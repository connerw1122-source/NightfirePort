# fe-gaps-g048 — the front end's open gaps (docs/GAPS.md G-048)

Branch `s/fe-gaps-g048`, 2026-10-17. Four of the six G-048 items are closed; the row in
docs/GAPS.md now carries only (3) P_CNCONTROLS 0x40000022 and (4) the results page's 0x5e key.

## 1. Menu_ChangePageCloseIris 0x202190 — the 10-tick delayed page change (closed)

Decompiled [M, disasm 0x202190-0x20220c]: `Menu_StartIris(1, …)` at once, a delayed 0x68/1 at
tick 1, a delayed `0x44` at tick 10 with lparam `DAT_002e1adc` (loaded at call time; 0 in .data
[L ACTION.ELF file offset 0x1dbadc] and zeroed by P_PARISENUM's 0x4c [M 0x20a648:21] — lparam 0
on every walk the port models, so the arrival pushes the opener for △ [M 0x21c858:262-263]).
The 0x44's own processing lifts the block [M 0x21c858:314-315]. Every caller listed in the
decompile xrefs (13 handlers; the full list is in docs/systems/frontend.md §G-048 and in the
`close_iris_page` doc): C_SBNFDFCTY 0x216ca8:16, P_NFSELECT 0x20aec0:103/:108, C_SBNFCN
0x20b390:47, C_SBNFMAP 0x20b8d8:48, C_SBMPSCEN 0x20c600:45/:142, C_SBMPOPTIONS 0x20e818:116,
C_SBBOTS 0x210a08:22, P_CNSELECT 0x212ad0:105/:112, C_SBCNSELECT 0x212f58:41, P_CNMENU
0x213288:49, C_SBCNOPTIONS 0x213528:58, C_SBDOSSIER 0x216eb0:20/:26, C_SBMPMAP 0x217538:17.

Port: `FrontEnd::close_iris_page` + `CLOSE_IRIS_PAGE_TICKS = 10` + `delayed_iris`/`delayed_block`
(the block lands the tick AFTER the call, lifted at the arrival), `close_iris_in_flight()` for the
draw. Converted arms: the difficulty confirm → the SP list, the codename play-without-saving →
Difficulty, the SP list's □ → Dossier, the Dossier wheel rows, the MP scenario → map. NOT
converted (their game paths are not CloseIris or need arrival-side work): the Back/0x5f pops
(immediate [M]), the pause/fail quit arms (MenuManager_Delete + a load), MP map → setup
(C_SBMPMAP:17 is CloseIris, but the port's `enter_mp_setup` work must move to the arrival first —
left as-is with the queue in the doc).

## 2. Menu_EnableSounds 0x1f31f0 — per-page sound flags (closed)

[M 0x21c858:267-268] every page open re-keys `enabled_sound_flags` via Menu_EnableSounds; the
switch is ported as `menu_enable_sounds(page)` and runs in `enter_page`. The boot value is
P_LANGUAGE's default arm → 9 (the manager-create 0x43 sends a direct 0x44 for the first page
[M 0x21c858:252]). The ee_start 0x0b [L ee_start 0x30d184] is re-homed: it is the value a MISSION
inherits from the SP list (0x4000001c → 0xb [M 0x1f31f0]), not a boot value — the old field doc
and the menu-sfx comment said "boot value" and were wrong. Back on Main is now silent (flags 1)
[ sister test ]; Back on a default-arm page buzzes.

## 3. The results page order — nothing opens until the load's MenuManager_Create (closed)

[M Mission_Update 0x182c60:72-75] the complete arm only sets GameState+0x1c = 0x40000036, loads
and pushes the fade — no MenuManager_Create until the front-end map's load tail [M 0x1d6360:160-163].
The FAIL arm is the opposite: `MenuManager_Create(0x80000004, 0x40000042, …)` runs at once over
the paused mission (:77-80) — unchanged. MP: `MP_Update` 0x184bd0 case 4 sets +0x1c = 0x40000033
(P_MPDEBRIEFING [M the handler table 0x2029e0:434-437]), loads 0x7000048, pushes the fade — the
debriefing opens at the load's create, NOT over the arena; `MenuManager_Monitor` 0x21d508's
0x4000004b create is the controller hot-swap monitor, a different path. Port: `page_open` /
`up_page()` model the manager's page-up flag; `show_mp_debriefing` queues the page id; the load
tail's existing `reopen_queued_page` lands both.

**Existing tests that contradicted the decompile were fixed** (each now cites the address):
menus_flow `the_results_pages_split_complete_from_failed` (pinned the immediate open — the test
was wrong), menu_audio_flows `an_sp_mission_start_stops_the_menu_music_for_the_level` (its own
comment marked the old behaviour `[S port]`), mp_debriefing_flow's handover test, menu_sp_e2e's
two results legs, campaign_flow_live's front-end leg, gameflow_stack_sp_run, create_save_live.

## 4. P_PARISENUM — the boot card check waits on the card machine (closed)

[M 0x20a648:12-47 + 0x1f7de8:53-58 + 0x1f7b78 + 0x110b98 + 0x110188 + 0x10e798 + 0x111038 +
0x217bd8:68]: the box waits on `DAT_002e1b0a`, set by PS2_DoCardFlow's completion arm after the
HandlePS2CardState stack machine walks GET_SAVE_GAMES → GET_CARD_STATUS (sceMcGetInfo) → the sync
waiter → the one-codename skip → the GetDir → the pop. The port's `ParisCardCheck` walks the same
states with the game's bounded in-tick loop; P_PARISENUM's 0x50 fires the 0x44 to P_ESTHERO only
on completion. **[S unverified]** the sceMc completion latency (`mc_ticks = 1` — the result lands
the next tick) and therefore the box's exact tick count (~4-5); no decompile or disc byte pins the
IOP mcman timing.

### Live checks for the main agent
1. Fresh PCSX2 boot, NO memory card: count the frames P_PARISENUM 0x4000004a is up (from the
   title's START to P_ESTHERO's first frame). Expected ~4-5 ticks with the [S] latency model; the
   measured number becomes `ParisCardCheck::mc_ticks`' pin.
2. Same boot WITH a card holding a BOND save: the box should last longer (the GetDir returns
   entries and the enum walks them) — a second pin for `next_save`'s non-empty path.
3. Difficulty → Single Player on a fresh boot: confirm the iris closes and the SP list appears
   ~10 ticks (≈0.17 s) after the ✕, input dead in between.

## Tests

`menu_sfx_flow.rs` (19: +page flags walk, +Back-on-Main sister, +CloseIris delay walk with the
in-window refusal sister, +the card machine's state walk) · `menus_flow.rs` (42) ·
`menu_graph_walk.rs` (2) · `menu_audio_flows.rs` (9) · `menu_movies.rs` (4) ·
`mp_menu_flow.rs` · `mp_net_lobby.rs` · `mp_debriefing_flow.rs` (6) · `menu_sp_e2e.rs` (9) ·
`campaign_flow_live.rs` (5) · `gameflow_stack_sp_run.rs` (4) · `create_save_live.rs` (4) ·
`menus_e2e.rs` (5) — all green on the final tree (merged with origin/master 4baed751).
`pause_results_flows_live`, `menus_e2e_mp`, `menus_backout_after_character`, `menus_mouse_parity`,
`menu_live`, `briefing_plays_live`, `codename_entry`, `menu_backdrop_still`, `menu_woman_live`,
`sfx_rejected_start_live`, `menu_music_flow` — green earlier on the same code paths.

**Full suite** (`tools/test/suite.py`, foreground; the sandbox kills background jobs): ran to
completion against the mid-fix tree — its failure list is exactly the walk-timing set fixed in the
last three commits, plus `net_e2e_loopback_live` (2 tests) and `probe_pylon_render` (2 tests).
The affected binaries were then re-run individually on the final tree, all green (list above).
`net_e2e_loopback_live` is SIGKILLed by this sandbox (signal 9, both standalone and in the suite —
a resource limit, not an assertion; the code paths it covers are untouched by this task) and
`probe_pylon_render` fails identically on master here (pylon rendering is not touched by this
task — left for its owner). **The final-tree full-suite total line is therefore not reproducible
in this sandbox; the next session should re-run `tools/test/suite.py` on this branch and expect
only those two binaries to fail.**

## Docs

docs/systems/frontend.md §"G-048" (the four mechanisms with addresses), docs/GAPS.md (the row
trimmed to items 3+4).

## Usage

not visible to me — this sandbox exposes no token/credit/context figures.
