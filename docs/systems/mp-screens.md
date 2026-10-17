# The multiplayer front-end pages (mp-screens)

The user (2026-10-16 evening): "missing mp screens". This doc lists every multiplayer front-end
page in the game, its handler, and the port's state — data/flow ported vs the draw (the main
agent draws every page and compares against PCSX2). Evidence: [M] ACTION.ELF decompile
(function:line), [D] disc data, [L] an EE snapshot.

## How pages bind (the dispatch)

There is no static page table: every front-end page id dispatches through the giant if/else
switch on `M_CONTROL+0x18` inside **`Handler_HandleMessage` 0x2029e0** [M], called from
`Manager_HandleMessage` 0x21df10:6. Handlers are `int handler(u8 player, M_CONTROL* m, u32 a,
u32 b, int msg, int arg)`; the ids below are the menu-def PAGE records' own ids (the port parses
them from the front-end archive 07000048's kind-8 entry 08000002 — `crates/nf-view/src/menu.rs`
"56 pages, 1003 controls", parsed 2026-10-13). Message ids: 0x4c page open, 0x50 tick, 0x4b
accept, 0x6b back, 0x44 goto page, 0x5f pop, 0x5a activate control, 0x2b show/hide, 0x18 set
text, 0x24 set icon, 0x1e select, 0x17 clear, 0x10 add item, 0x35/0x34 get value/text, 0x39
find/create control [M, cited per handler below].

## The MP flow, page by page (13 pages, the walk order)

| # | Page id | Handler | Purpose | Port state |
|---|---|---|---|---|
| 1 | 0x40000002 | P_MAIN 0x20a7b0 | main menu; the Multiplayer row (control 0x10000003 `C_GOMULTIPLAYER` 0x216a78) delays 0x44 → 0x40000019 [M 0x216a78:36-37] | ported, drawn |
| 2 | 0x40000019 | P_MPJOIN 0x20ca00 | join lobby: memsets `mpjoin` 0x2dee68 (4×0x10: +0 joined, +1 ready, +4 team, +8 character), per-pad START (`C_RBMPSTART` 0x20d048: state 0→1, ready at 5 via `Menu_MPFadeWhenReady`), name entry (`C_RBMPCNAME` 0x20d448 writes MPSettings+p·0x30, default "Player N" label 0x1c3); all ready → 0x44 0x4000001a [M 0x20ca00:106-193] | ported, drawn (`MpJoinAgent` mirrors mpjoin) |
| 3 | 0x4000001a | P_MPSCENARIO 0x217348 | scenario wheel (`C_SBMPSCEN` 0x20c600 over `mp_scenario` 0x2df508, 13×0x18); pick writes +0x1A4 → 0x40000013; **row 0 = Quick Game**: random map off mp_level[0..6], frags 10, time 10, 3 bots (chars 1/3/2), skins 0/6/7/8 → 0x40000049; 0x63 timeout → main [M 0x20c600:29-145] | ported, drawn (wheel + locked rows); Quick Game = `mp_quick_start` (headless takes map row 0 [S — the game samples Rand%7]) |
| 4 | 0x40000013 | P_MPMAP 0x217468 | map wheel (`C_SBMPMAP` 0x217538 over `mp_level` 0x2df400, 8×0x18, lock byte +0x10); accept writes GameState+8 and +0x1A8 → 0x40000051 [M 0x217538:11-17] | ported, drawn |
| 5 | 0x40000051 | P_MPSETUP 0x20d7a8 | per-agent setup: 4 panels (finish 0x1000022a, list 0x100001a1, prompt 0x100001a2, plate 0x100001a0); team game → prompt 0x37c + Phoenix 0x1c7/MI6 0x1c8 list (state 2), FFA → 0x37d + `Menu_GetMPSkins` (state 3); `C_RBMPSETUP` 0x20de58 steps 2→3→4 (the handicap .data strings 0x30d390..)→5 writing MPSettings+p·0x30+0x2c; all ready (`Menu_MPAreWeReady` 0x1f78d8) → 0x40000012; page-enter defaults PlayerSetting+0x156 = 1/3/2/0 [M 0x20d7a8:15-89] | flow ported (`mp_setup_state` 2/3/4/5); **draw was EMPTY — data now in `FrontEnd::mp_setup_panels()`** |
| 6 | 0x40000012 | P_MPOPTIONS 0x217678 | options hub (`C_SBMPOPTIONS` 0x20e818 over `mp_options` 0x2dfbb0): 0 START GAME (team-balance + ≥2 checks → 0x40000049 or the error box), 1 → bots 0x40000027, 2 → rules 0x40000014, 3 → player mods 0x40000017, 4 → enviro mods 0x40000028; "no map" flag at +0x1A8 == 0x700004b [M 0x20e818:20-129] | ported, drawn (`mp_start_checks`) |
| 7 | 0x40000014 | P_MPRULES 0x20f790 | points (0x10000008, 5..60) + duration (0x100000f6, 5..120 min) combos → +0x1A0/+0x19C; row 1 re-binds 0x3d3 "Lives" for scenario 0x10 [M 0x20f790:12-95] | ported, drawn |
| 8 | 0x40000017 | P_MPPLAYERMODS 0x2104d0 | 5 combos → +0x198 (friendly fire), +0x1B4 (weapon set), +0x1BC, +0x1C8, +0x1C4 (radar) [M :12-16] | ported, drawn |
| 9 | 0x40000028 | P_MPENVIROMODS 0x210030 | 5 combos → +0x1C0/+0x1B8/+0x1D4/+0x1D0/+0x1CC; the GoldenEye-gun item gated by mp_stuff+0x370 [M :12-38] | ported, drawn |
| 10 | 0x40000027 | P_MPBOTS 0x217790 | bot-count wheel (`C_SBBOTS` 0x210a08 over `mp_bots` 0x2deea8: count at +1, 0x12-byte slots); select → mp_stuff+0x37a slot → 0x4000003f [M 0x217790:27, 0x210a08:20-22] | ported, drawn |
| 11 | 0x4000003f | P_MPBOTCHOOSE 0x210cd8 | character wheel (`C_SBMPBTCHOOSE` 0x210f08 over `mp_characters` 0x2df640, unlock filter); accept writes mpbots[slot]+0 + the team-captain logic (mp_stuff+0x378/0x379) → 0x4000002c [M 0x210cd8:17-56] | ported, drawn |
| 12 | 0x4000002c | P_MPBOTSETUP 0x211270 | 8 combos per bot (char 0x10000116, skill 0x10000186, team 0x10000185, limit 0x10000187, aggression 0x1000018c, weapon 0x10000195, limit 0x10000188, 0x1000018a) → the mpbots slot [M :139-158] | ported, drawn (`MpBot`) |
| 13 | 0x40000049 | P_MPCONFIRM 0x20ec00 | match summary; accept: `Menu_StoreMPSettings`, compaction, +0x1A0 ×60, +0x1D4 &= 1, **+0x180 = 1**, GameState+0x40 = 1, GameState+8 = map, `ResetMap_LevelToLoad` + `GameFlow_PushState(80,7,0xff)` + `MenuManager_Delete` [M :53-156] | flow ported (`mp_confirm_start`); **draw was EMPTY — data now in `FrontEnd::mp_confirm_summary()`** |
| — | 0x4000004b | C_GCPAUSE 0x204408 | in-match pause; quit routes to 0x40000033 when MPSettings+0x180 [M :142-146] | ported, drawn |
| 14 | 0x40000033 | P_MPDEBRIEFING 0x212198 | results: `Menu_GetMPScore` over players+bots, sorted into rows 0x10000156-0x1000015c; winner 0x1000023d ("%s wins" 0x100029f / draw 0x10002a0); team games compare MPGame+0x384/+0x388 → 0x1c7/0x1c8; rematch `C_MPDBG` 0x217860 → 0x4000001a (0x5d restarts the level) [M :92-262] | ported, drawn (`MpDebriefing`) |

Non-MP bookends: 0x40000053 P_WINGAME 0x215100 (campaign win), 0x10000010 `C_LBPMMAP` 0x215ac8
(the message-board 1-player quick launch: +0x1AC = 1 + `ResetMap_LevelToLoad` + the 84-frame
fade + `MenuManager_Delete` [M :11-17]).

## The MPSettings front-end struct (0x2a47a0, 0x1DC)

Per-participant blocks 8 × 0x30 (humans 0-3, bots 4-7): +0x00 name (0x20), +0x20 controller,
+0x24 character, +0x2C handicap. Globals: +0x180 MP flag (set by P_MPCONFIRM:149 /
C_SBMPSCEN:53; read by `PlrStat_DoneBondMoment` 0x199448:13 and the pause quit routing), +0x184
the in-match copy (`MP_Start` 0x183a50:70), +0x18C/+0x190 the team/bit30 flags (`MP_Init`
0x183438:41-42), +0x194 total participants, +0x198 friendly fire, +0x19C time limit (min),
+0x1A0 score limit (×60 on launch), +0x1A4 scenario word (bit 0x20000000 = team), +0x1A8 map,
+0x1AC player count, +0x1B0 bot count, +0x1B4 weapon set, +0x1B8..+0x1D4 enviro/player mods
(citations in the research census; each offset's reader/writer cited in the handlers above).
The companion runtime structs: `PlayerSetting` 0x2a38c8 (4×0x158, +0x156 the handicap default),
`mpjoin` 0x2dee68, `mp_stuff` 0x2def60 (+0x360+p·4 the setup states, +0x378/0x379 the team
captains, +0x37a the bot slot, +0x370 the GoldenEye-gun unlock), `mpbots` 0x2deea8, `mp_level`
0x2df400, `mp_scenario` 0x2df508, `mp_characters`/`_small` 0x2df640/0x2df8f8, `mp_options`
0x2dfbb0, `mp_bots` 0x2dfd30.

## The spot-shot bug (the second symptom)

F12 (and the panel's "shot") on a front-end page saved an all-dark image
(nf-shots/001-07000048.png). The window draws every front-end page over its **aux FMV backdrop**
through the ONE movie plane (`menu::aux_movie_for(fe.page_id())`, the live loop's player); the
overlay skips its dark plate while `fe.backdrop_live` stands. `spot_shot` built the overlay but
passed **no movie frame** (`movie: None`) — the plane drew nothing, the plate was skipped, and
the front-end level's void rendered black: an all-dark png. The fix mirrors the redraw's movie
gates (`spot_movie_gates`): the state-6 briefing / intro / attract players blank the overlay,
the looping page backdrop draws under it, and the shot passes the live frame.

## What the draw code still needs (the main agent's renders)

- `Page::MpSetup` (0x40000051) and `Page::MpConfirm` (0x40000049) draw from
  `mp_setup_panels(lang)` / `mp_confirm_summary(lang)` — the four panel rects are the menu-def's
  own (the parse's controls 0x100001a0-0x100001a2/0x1000022a and 0x1000022d-0x1000023f carry the
  geometry), the data getters above carry the strings/ids/values.
- Render-check spots (front-end boot, `--page`): `--page mpscenario --cursor 0` (the Quick Game
  row), `--page mpmap --cursor 1`, `--page mpscenopts` (the hub), `--page mprules`,
  `--page mpbots`, `--page mpbotchoose`, `--page mpbotsetup`, and the walk into
  MpSetup/MpConfirm through the flow (the two formerly-empty pages).
- The 0x1000023f header strings (0x295 FFA / 0x1c8 team) and the "%s%d" handicap prefix
  (0x30d3e0 / 0x30d2d8) are .data/label ids recorded as-is — the PCSX2 compare settles the
  English.
