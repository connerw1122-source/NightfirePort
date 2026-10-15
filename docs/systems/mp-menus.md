# The multiplayer front end: the page flow, the lists, the match handover

mp-menu-flow (2026-10-15, branch `s/mp-menu-flow`). Everything here is first-hand: [M] = decompile
read in this task (`work/decomp/ACTION.ELF/<addr>_<name>.c`, line numbers refer to those files),
[L] = a byte dump or capture made in this task, [S] = suggested/inferred. The port side is
`crates/nf-view/src/menu.rs` (the `MP_*` tables, the `Page::Mp*` variants, `FrontEnd`'s `mp_*`
fields and methods) with `mp_menu_flow.rs` (headless) and `mp_menu_lists_live.rs` (ISO-gated).

**Atlas correction this task settles** — the decompiler prints struct offsets in DECIMAL, so the
multiplayer atlas's "MPSettings+0x420/+0x424/+0x428/+0x432" rows are +0x1A4/+0x1A8/+0x1AC/+0x1B0
in hex. `MPSettings._424_4_ == 0x700004b` [M P_MPOPTIONS 0x217678:20-25] reads the LEVEL code at
0x2A4948, and `MPSettings._420_4_` (0x2A4944) is BOTH the scenario word and the gametype bitmask
the atlas lists as two fields (the scenario ids' low bits are the gametype bits).

## 1. The page graph

All 13 pages are PAGE records of the front-end menu-def (kind-8 entry 08000002 in
FILES.BIN's 07000048.bin), all kind 0 with aux movie 0x07350048 [L the menu-def dump 2026-10-15].
Every page header below is the menu-def's header control (50,40,376×17) msg-0x18 b field; the hint
bar (50,419,540×21) binds 0x217 where listed.

```
Main menu (0x40000002) row "Multiplayer"
  └─ C_GOMULTIPLAYER 0x216a78: delayed msg 0x44 (0x13 frames) →
JOIN GAME 0x40000019 (P_MPJOIN 0x20ca00; header 0x29e)
  ├─ ✕ on an idle panel: C_RBMPSTART 0x20d048:4b (state 0 → 1) — "Choose Codename" 0x37b
  ├─ ◀ ▶ on the codename list (control 0x1000022b; C_RBMPCNAME 0x20d448)
  ├─ ✕ on the list: C_RBMPCNAME:4b — codename into MPSettings+slot·0x30, state → 5
  ├─ ✕ once joined: C_RBMPSTART:4b (state 5) — "Player Ready" 0x39c, mpjoin+1 = 1
  ├─ all joined ready: P_MPJOIN:50 → Menu_MPAreWeReady → msg 0x44 →
  │
SELECT SCENARIO 0x4000001a (P_MPSCENARIO 0x217348; header 0x2c3)
  ├─ Back/msg 0x63 → 0x40000002 (main menu) [M 0x217348:29]
  ├─ Confirm row 0 (Quick Game): C_SBMPSCEN 0x20c600:48-145 — the fixed quick-start config,
  │   3 bots, → 0x40000049 (confirm) directly [M disasm 0x20c978-0x20c984]
  ├─ Confirm row n>0: MPSettings+0x1A4 = mp_scenario[n].id [M disasm 0x20c9c8] →
  │   Menu_ChangePageCloseIris(0x40000013) [M disasm 0x20c9a8-b4] →
SELECT MAP 0x40000013 (P_MPMAP 0x217468; header 0x158)
  ├─ Confirm: MPSettings+0x1A8 AND GameState+0x08 = mp_level[n].id [M disasm 0x217654/0x21765c]
  │   → 0x40000051 [M disasm 0x217638-0x217644] →
PLAYER SETUP 0x40000051 (P_MPSETUP 0x20d7a8; header 0x29e)
  ├─ per agent: team game (scenario & 0x20000000) asks "Choose Team" 0x37c (state 2), FFA asks
  │   "Choose Character" 0x37d (state 3) [M 0x20d7a8's state writes]
  ├─ C_RBMPSETUP 0x20de58:4b steps 2 → 3 → 4 (team into mpjoin+4, character into mpjoin+8)
  ├─ all picked →
OPTIONS HUB 0x40000012 (P_MPOPTIONS 0x217678; header 0x2c4)
  ├─ row 0 Continue: C_SBMPOPTIONS 0x20e818:50-130 start checks → 0x40000049, or an error box
  ├─ row 1 AI Bots → 0x40000027 [M :116]      (locked when the map is Ravine [M 0x217678:20-25])
  ├─ row 2 Game Rules → 0x40000014 [M :119]
  ├─ row 3 Player Mods → 0x40000017 [M :122]
  ├─ row 4 Enviro-Mods → 0x40000028 [M :125]
  │   (Rules/PlayerMods/EnviroMods close with Manager msg 0x5f back to the hub [M their :14-17])
  │
AI BOTS 0x40000027 (P_MPBOTS 0x217790; header 0x294)
  ├─ row 0 Continue: Manager 0x5f → the hub [M C_SBBOTS 0x210a08:15]
  ├─ rows 1-4 Setup Bot n: mp_stuff[0x37a] = row−1 [M :20] → 0x4000003f [M :23] →
BOT CHOOSE 0x4000003f (P_MPBOTCHOOSE 0x210cd8; header 0x294)
  ├─ Confirm: record+0x10 = character, +0x0F team = Menu_IsBotGood, name into
  │   MPSettings+0xC0+slot·0x30, +0x0E enabled = 1 [M C_SBMPBTCHOOSE 0x210f08:26-43] →
  │   0x4000002c →
BOT SETUP 0x4000002c (P_MPBOTSETUP 0x211270; header 0x294)
  ├─ Confirm: values store, +0x11 dirty = 1, Manager 0x5f twice → the Bots list [M :138-169]
  │
MATCH SUMMARY 0x40000049 (P_MPCONFIRM 0x20ec00; no static header — runtime binds)
  ├─ Confirm = THE MATCH START (§4) — no other page leads into a match.
  │
POST-MATCH DEBRIEFING 0x40000033 (P_MPDEBRIEFING 0x212198)
  ├─ reached from C_GCPAUSE 0x204408:137-153 (quitting a match with MPSettings+0x180 != 0 sets
  │   GameState+0x28 = 0x40000033 after ResetMap_LevelToLoad(0x7000048,0,0) + the 80.0f fade)
  ├─ C_MPDBG 0x217860: 0x4b confirm → Menu_RestoreMPSettings + msg 0x44 arg 1 → 0x4000001a
  │   (the scenario page) [M :8-11]; msg 0x5d rematch → ResetMap_LevelToLoad(+0x1A8,0,0) +
  │   GameFlow_PushState(80.0f,7,0xff) + MenuManager_Delete [M :12-16]
```

## 2. The lists (the game's own tables)

Every wheel table is 0x18-byte records `{+0x00 preview texture, +0x04 name label, +0x08
description label, +0x0C id, +0x10 unlocked flag, +0x14 locked-description}` [M Menu_UpdateWheel
0x1f8388:94-234 renders exactly these]. All five tables are in ACTION.ELF's .data (one PT_LOAD:
file offset = vaddr − 0x106100) and are pinned row-for-row by `mp_menu_lists_live.rs`.

### Select Scenario — `mp_scenario` @0x2df508 (file 0x1d9408), 13 rows

| row | preview | name (id) | scenario word (+0x0C → MPSettings+0x1A4) | unlock |
|---|---|---|---|---|
| 0 | 0x0300013a | 0x010002dd Quick Game | 0 (special — §5) | always |
| 1 | 0x03000099 | 0x1cb Arena | 0x00000001 Deathmatch | always |
| 2 | 0x0300009a | 0x1cd Team Arena | 0x20000002 | always |
| 3 | 0x0300009b | 0x1cf Capture The Flag | 0x20000004 | always |
| 4 | 0x030000a4 | 0x1d1 Uplink | 0x60000008 | reward 0x39 |
| 5 | 0x0300009c | 0x1d3 Top Agent | 0x00000010 (the death-limit FFA) | always |
| 6 | 0x0300009d | 0x1d7 Demolition | 0x20000040 | reward 0x3b |
| 7 | 0x0300009e | 0x1d9 Protection | 0x20000080 | reward 0x3c |
| 8 | 0x0300009f | 0x1db Industrial Espionage | 0x20000100 | always |
| 9 | 0x030000a0 | 0x1dd GoldenEye Strike | 0x20000200 | reward 0x3d |
| 10 | 0x030000a1 | 0x1df Assassination | 0x00000400 | reward 0x38 |
| 11 | 0x030000a2 | 0x1e1 King of the Hill | 0x40000800 | always |
| 12 | 0x030000a3 | 0x1e3 Team King of the Hill | 0x60001000 | reward 0x3a |

Locked rows show 0x010000ab "This scenario is locked." (every row's +0x14) and refuse Confirm [M
C_SBMPSCEN:40-42]. The unlock source is `Menu_UnlockMPSettings` 0x1ff698:53-104: it ZEROES rows
4/6/7/9/10/12's flag bytes and re-derives them from the campaign's type-2 rewards (0x38→row 10,
0x39→row 4, 0x3a→row 12, 0x3b→row 6, 0x3c→row 7, 0x3d→row 9) via PlrStarts_ProcessRewardCounter.
Note row 12 SHIPS a 1 byte [L the .data dump] and is still locked at runtime — the port models the
runtime behavior (the reward column in `MP_SCENARIO_ROWS`).

### Select Map — `mp_level` @0x2df400 (file 0x1d9300), 8 rows

| row | preview | name (id) | desc (id) | level code |
|---|---|---|---|---|
| 0 | 0x03000086 | 0x03000024 Skyrail | 0x03000026 | 0x07000024 |
| 1 | 0x0300008b | 0x03000025 Fort Knox | 0x03000027 | 0x07000027 |
| 2 | 0x030000ba | 0x03000028 Snow Blind | 0x03000029 | 0x07000029 |
| 3 | 0x030000b9 | 0x0300002a Phoenix Base | 0x0300002b | 0x07000026 |
| 4 | 0x030000b8 | 0x0300002c Atlantis | 0x0300002d | 0x07000023 |
| 5 | 0x030000b7 | 0x0300002e Missile Silo | 0x0300002f | 0x07000028 |
| 6 | 0x030000b6 | 0x03000030 Sub Pen | 0x03000031 | 0x07000025 |
| 7 | 0x030000b5 | 0x03000032 Ravine | 0x03000033 | 0x0700004b |

Every +0x10 ships 1 and every +0x14 is 0 — no map is ever locked on retail [L the dump; the lock
check C_SBMPMAP:12-14 is a no-op]. The codes are exactly the on-foot ISO's arenas (0x21/0x22/0x4c
are absent from the table). The wheel is NOT filtered per scenario [M nothing in P_MPMAP /
C_SBMPMAP reads MPSettings+0x1A4] — the only map-dependent gates in the flow are Ravine's.

### Options hub — `mp_options` @0x2dfbb0 (file 0x1d9ab0), 5 rows

| row | preview | name | desc | id |
|---|---|---|---|---|
| 0 | 0x0300013a | 0x171 Continue | 0x0100001c | 0 |
| 1 | 0x03000136 | 0x1ad AI Bots | 0x01000020 "Add computer-controlled bots to your game." | 1 |
| 2 | 0x03000139 | 0x19b Game Rules | 0x0100001d | 2 |
| 3 | 0x03000138 | 0x19d Player Mods | 0x0100001e | 3 |
| 4 | 0x03000137 | 0x19e Enviro-Mods | 0x0100001f | 4 |

Row 1's +0x10 byte is cleared at page open when MPSettings+0x1A8 == 0x700004b [M
P_MPOPTIONS:20-25], with locked-desc 0x010000ae "You can't have bots play the Ravine map."

### AI Bots — `mp_bots` @0x2dfd30 (file 0x1d9c30), 5 rows

Rows 0-4 = (0x0300013a, 0x171 Continue) + (0x03000099, 0x284..0x287 "Setup Bot 1".."Setup Bot 4");
no descriptions [L the dump]. Rows 1-4 are REBUILT at scroll time: the status suffix
sprintf("%s : %s" @0x30d3e8, 0x295 "Playing", 0x181 Yes / 0x182 No) from record+0x0E, and the row
preview becomes the configured character's texture [M C_SBBOTS 0x210a08:40-59]. There is no
bot-count control anywhere: the count is however many records are enabled (compacted by
`Menu_PrepareBots` 0x1f94a0 into mpbots[1] @0x2DEEA9 and MPSettings+0x1B0, or 0 on Ravine [M
:82-88]).

### Bot Choose — `mp_characters` @0x2df640 (file 0x1d9540), 29 rows

+0x0C == the row index == the `default_bot_stats` preset index (already lifted in
`crates/nf-game/src/mp/bots.rs`). Rows 0-11 ship unlocked: 0 Bond 0x21a, 1 Drake 0x21b, 2 Rook
0x21c, 3 Kiko 0x21d, 4 Alura 0x21e, 5 Dominique 0x21f, 6 Snow Guard 0x220, 7 Black Ops 0x221, 8
Yakuza 0x222, 9 Phoenix Commando 0x223, 10 Phoenix Soldier 0x224, 11 Ninja 0x225. Rows 12-28 are
reward-locked (`Menu_UnlockMPSkins` 0x1ff2f8:113-187: 0x27→12, 0x29→27, 0x2a→25, 0x2b→24, 0x2c→26,
0x2d→20, 0x2e→21, 0x2f→15, 0x30→23, 0x31→22, 0x32→19, 0x33→18, 0x34→16, 0x35→28, 0x36→13,
0x37→14; row 17 = reward 0x28, row 24 = 0x2b), each with locked-desc 0x010000ad "This character is
locked." [L every row's +0x14]. Previews are per row in the table [L the dump — e.g. 0x0300010a
Bond, 0x0300010e Drake, …]. Confirm also derives the team: record+0x0F = `Menu_IsBotGood(sel)`
0x2027a8 = `BOT_getDefaultStats(sel)[9] == 0` — the preset's +0x09 byte is the side (0 = MI6:
Bond, Alura, Dominique, Bond Tux, Bond Spacesuit, Pussy Galore, Christmas Jones, Wai Lin; the
other 21 are Phoenix) [L the dump; `MP_CHARACTER_SIDE_MI6` in menu.rs].

### The other pages' rows (menu-def controls, not wheel tables)

- **Join 0x40000019**: four agent panels. Agent 1's states bind 0x1e8 "Press ~A to join" / 0x37b
  "Choose Codename" / 0x1e9 "Press ~A\nwhen ready" / 0x39c "Player Ready" [M C_RBMPSTART's
  0x5c/0x4b arms]; the codename list (0x1000022b) holds the two def_codename rows then the card's
  saves [M Menu_PutCodenamesIntoControl 0x1f79c0], defaulting to "Player N" (Txt 0x1c3 through
  sprintf "%s %d" @0x30d388 with slot+1) [M C_RBMPCNAME 0x20d448].
- **Player Setup 0x40000051**: per-agent list controls (0x100001a1); team games add Phoenix
  0x1c7 / MI6 0x1c8 items, FFAs fill the unlocked `mp_characters_small` @0x2df8f8 skins [M
  P_MPSETUP 0x20d7a8 + Menu_GetMPSkins].
- **Game Rules 0x40000014**: two list rows — Duration (control 0x10000008, title 0x156 [L the
  menu-def's msg-0x18 b fields]) 1..60 minutes then 0x3ca "Unlimited" (index −1) [M P_MPRULES
  0x20f790:20-42], and Points/Lives (0x100000f6, title 0x24c, re-bound to 0x3d3 "Lives" for
  scenario 0x10 [M :78-95]) 1..120 [M :65-76]. Confirm reads both into MPSettings+0x1A0
  (minutes, −1 = none) and +0x19C [M :12-13] and closes.
- **Player Mods 0x40000017**: five rows (titles from the menu-def [L]): 0x1ae Friendly Fire →
  +0x198, 0x24d Weapon Set → +0x1B4 (11 sets: Normal 0x1a0, Pistols 0x1a2, Automatic 0x1a4,
  Sniping 0x1a6, Explosives 1 0x1a8, Explosives 2 0x24f, MI6 Operative 0x3db, Phoenix Weapons
  0x3dc, State of the Art 0x3dd, Cloak and Dagger 0x3de, Random 0x1b2 [M 0x2104d0:26-49]), 0x1ac →
  +0x1BC, 0x24e → +0x1C8, 0x1e5 Radar → +0x1C4 [M :12-16].
- **Enviro-Mods 0x40000028**: five rows [L the menu-def titles]: 0x1af View Distance → +0x1C0
  (0x1b0 Near/0x1b1 Far/0x1b2 Random), 0x1ab → +0x1B8, 0x1aa → +0x1D4 (single 0x1000076 "Locked"
  item unless the type-3 reward 0x3e set mp_stuff[0x370] @0x2df2d0 [M :35-43]), 0x328 → +0x1D0,
  0x5b9 vehicles → +0x1CC (0x1b6 Off/0x10002d8 Tanks/0x10002d9 Helicopters/0x1b2 Random) [M
  :52-61].
- **Bot Setup 0x4000002c**: eight list rows in y order (labels [L the menu-def]): 0x295 Enabled →
  record+0x0E (Yes 0x181/No 0x182), 0x247 Speed → +0x06 (0x23c Slow 0/0x23d Normal 1/0x23e Fast
  2), 0x232 Trait → +0x0B (None 0x10002d0→0; MI6 chars: 0x233 Collector 1, 0x234 Guardian 2,
  0x235 Team Player 3, 0x24a Judge 4; Phoenix chars: 0x237 Berserker 5, 0x238 Greedy 6, 0x239
  Vengeful 7, 0x24b Assassin 8), 0x246 Aim → +0x02 (0x3bd Normal 2/0x31b High 3/0x31c Very High
  4), 0x15e Skill → +0x00 (0x241 Poor 8/0x242 Average 5/0x243 Good 3/0x244 Very Good 1 — lower is
  smarter), 0x1c6 Hit points → +0x04 (50..300 in 50/25 steps [M :69-80]), 0x23a Recovery 1 →
  +0x07 (50..200), 0x249 Recovery 2 → +0x08 (50..200) [M P_MPBOTSETUP 0x211270:17-135]. For
  characters ≥ 15 (the reward presets) all seven stat rows lock [M :23-48] and the page hints
  0x10002d1 "This Bot's statistics and personality are fixed and cannot be modified." [M
  :172-175]. record+0x0A (preferred weapon class) has no menu row — it comes only from the preset.
- **Match Summary 0x40000049** (page enter, P_MPCONFIRM 0x20ec00:95-260): blue/red player panels
  (controls 0x10000235-0x1000023a), the bot panels 0x10000233/0x10000234, and the lines Map
  (0x3d4 + the mp_level name), Scenario (0x3d5 + the mp_scenario name), Weapon Set (0x24d + the
  +0x1B4 name), Points/Lives (0x3d8/0x3d7 + the +0x19C value; 0x3ca "Unlimited" for the −1/KOTH
  cases), Duration (0x3d6 + +0x1A0 or 0x3ca), Friendly Fire (0x3da + 0x1b7/0x1b6) — each sprintf
  "%s : %s" @0x30d3e8 [M].

## 3. Defaults

- Shipped .data: MPSettings zeros (+0x19C frag, +0x1A0 time, +0x1A4 scenario, +0x1A8 level, +0x1B0
  bots), `mpjoin` zeroed [L the dump].
- The options hub's first open (msg 0x51) seeds the four bot records' characters to 6/7/8/9
  (Snow Guard, Black Ops, Yakuza, Phoenix Commando) [M C_SBMPOPTIONS 0x20e818's init writes
  DAT_002deeba=6, 0x2deecc=7, 0x2deede=8, 0x2deef0=9] — enabled stays 0, so a fresh run starts
  with 0 bots.
- Wheel cursors: scenario opens on the row matching MPSettings+0x1A4 (fresh 0 → Quick Game) [M
  Menu_SelectItemInControl 0x2020f8's scan + fallback row 0]; map on +0x1A8's match (fresh → row
  0 Skyrail); the Bots list on mp_stuff[0x37a]+1; Bot Choose on the slot's current character.
- Quick Game (§5) is the game's own "no choices" preset.

## 4. What the confirm hands to the game

P_MPCONFIRM 0x20ec00 msg 0x4b (:53-94), in order:
1. `Menu_StoreMPSettings()` 0x2027d0 — MPSettings 0x1DC bytes → the backup @0x2dec68 plus the 4
   PlayerSetting records (0x158 each) → @0x2de708 [M]. (Menu_RestoreMPSettings 0x202848 reverses
   it — that's the debriefing's Confirm.)
2. The joined players compact: for each mpjoin[i] != 0, PlayerSetting[i·0x158] is overwritten from
   the template and the record lands at the front; the teams/skins write into
   MPSettings+0x20+i·0x30 (+0x20 team, +0x24 skin) [M].
3. `MPSettings+0x1A0 != -1 → *= 0x3c` — the time limit becomes SECONDS here.
4. `+0x1D4 &= 1`, `+0x180 = 1` (the MP master enable), `GameState+0x40 = 1` (skip draw).
5. `GameState+0x08 = MPSettings+0x1A8` (the arena), `+0x1AC = joined count`.
6. `ResetMap_LevelToLoad(MPSettings+0x1A8, 0, 0)` + `GameFlow_PushState(80.0f, 7, 0xff)` (the
   fade) + `SFXFadeDown(1)` + `MenuManager_Delete`.

**Tie to the port's MP core** (`crates/nf-game/src/mp/`): `MpMatch::init(MpSettings)` consumes
`scenario` (+0x1A4 — and derives team_game/bit30 from it exactly as MP_Init 0x183438 does),
`level_code` (+0x1A8), `frag_limit` (+0x19C), `time_limit` (+0x1A0 — the menu hands SECONDS per
the confirm's ×0x3c; the existing `--mp` path hands raw minutes, which looks wrong against
MPGame+0x404 — flagged to the main agent), `human_count` (+0x1AC), `bot_count` (+0x1B0, clamped ≤
4 by MP_Start 0x183a50 which also re-reads the teams into the per-player records), `weapon_set`
(+0x1B4 → the PickupMatrix row), `radar` (+0x1C4), `master` (+0x180), `player_count` (+0x1AC) and
the per-player `team`/`skin`. The menu's handover is `FrontEnd::mp_confirm_start` →
`mp_match_settings: Option<MpSettings>` + the level request through `requested_level` /
`GameFlow::reset_map_level_to_load` — the load machinery reaches the arena code (verified headless
in `mp_menu_flow::quick_game_lands_on_confirm_with_three_bots`), and the host wires
`mp_match_settings` into the sim's `MpMatch` when the front end closes.

## 5. Quick Game

C_SBMPSCEN row 0 (:48-145 + disasm 0x20c72c-0x20c864): `Menu_PrepareBots`, then the fixed config —
+0x1A0 = 10 (time), +0x19C = 10 (frags), +0x1A4 = 1 (Deathmatch), +0x180 = 1, +0x198 = 0 (no
friendly fire), +0x1C4 = 1 (radar), +0x1B4 = 0 (Normal weapons), +0x18C = 0 (FFA), +0x1CC = 2
(Helicopters — corrected 2026-10-15 from the store at 0x20c734; Car_Create 0x1305b0:29-40 spawns
choppers on 2), +0x1D4 = 0, +0x190 = 0, +0x1D0 = 0 (+0x1C0 is NOT written — the boot value 2
persists). The map is `mp_level[Rand_Random() % 7]` — rows 0..6, never
Ravine [M 0x20c76c-0x20c7b4]. The joined players get fixed teams/skins (slot 0 → MI6/Bond, slots
1-3 → Phoenix skins 6/7/8 [M disasm 0x20c7f8-0x20c834]); 3 bots enabled (characters 1 Drake, 3
Kiko, 2 Rook — all Phoenix, preset stats, count byte 0x2DEEA9 = 3, MPSettings+0x1B0 = 3); then
straight to 0x40000049. The port samples map row 0 headlessly (the shared RNG is not wired into
the menu) — flagged as a deviation [S].

## 6. The port (what exists, what the draw code gets)

`crates/nf-view/src/menu.rs`:
- `Page::MpJoin | MpScenario | MpMap | MpSetup | MpOptions | MpRules | MpPlayerMods |
  MpEnviroMods | MpBots | MpBotChoose | MpBotSetup | MpConfirm | MpDebriefing` with the PAGE_MP_*
  id constants; `aux_movie_for` covers all 13 (aux 0x07350048).
- `FrontEnd` fields (all on their own lines): `mp_join: [MpJoinAgent; 4]` (the game's mpjoin
  layout + mp_stuff states), `mp_scenario`/`mp_level` (the MPSettings words), the five wheel
  cursors, `mp_bot_slot`/`mp_bot_char`, `mp_bots: [MpBot; 4]` (the 0x12-byte records), the
  rules/mods values (`mp_time_limit`, `mp_frag_limit`, `mp_friendly_fire`, `mp_weapon_set`,
  `mp_radar`, `mp_player_mods`, `mp_envmods`), `mp_error`, `mp_match_settings`.
- `action()` handles every page (the graph in §1); the Left/Right remap now exempts the value
  pages (Join/Setup/Rules/PlayerMods/EnviroMods/BotSetup) the way the option pages always did.
- Getters for the draw code: `rows()`/`row_label()` per page (the join states' prompts, the wheel
  names, the composed "Setup Bot n : Playing Yes/No" rows), `mp_row_preview(row)`,
  `mp_row_desc(row)` (with the locked-desc swap), `mp_row_locked(row)` (the dim), plus
  `mp_confirm_lines(lang)` for the summary page's six lines, `mp_human_count()`, `mp_bot_count()`.
- Keyboard-only, agent 1: Confirm joins / picks the codename / readies on the Join page; agents
  2-4 keep their "Press ~A to join" prompt and can never join (the game needs their pads — msg
  0x6a [M P_MPJOIN]). The main menu's Multiplayer row now navigates (C_GOMULTIPLAYER).

Tests: `crates/nf-view/tests/mp_menu_flow.rs` (9 headless: the whole flow, back-outs, the lock
gates, the bots loop, the start checks, Quick Game, the team setup, the loader handover) and
`mp_menu_lists_live.rs` (5 ISO-gated: the five tables + the character sides byte-for-byte from the
disc's ACTION.ELF). Every flow step keeps a sister assertion.

## 7. Gaps and what's left (for the next pass)

- **Drawing**: the overlay's MP arms are empty — the main agent draws the 13 pages (the getters
  above are the intended surface; the wheel renderer `Menu_UpdateWheel`'s ribbon/preview/desc
  rules in §2 apply to all five MP wheels the same way they already do for the campaign's).
- **Rewards → MP unlocks**: `Menu_UnlockMPSettings`/`UnlockMPSkins` map the campaign rewards
  (§2) — the port gates on `unlock_all` only; wiring the profile's reward bits is not done.
- **Pads / agents 2-4**: no join path (no input devices); msg 0x6a's pad mapping
  (`MapPlayerToSlot`, the DAT_002a3b76 group) is untouched.
- **The Join page's codename keyboard**: rows beyond the two defaults open the game's name-entry
  page (not ported — the defaults "Player N" are what Quick Game uses anyway).
- **Menu_StoreMPSettings/Restore** (the MP memory-card block @0x2dec68/0x2de708) — not persisted
  by the port; the debriefing's Confirm returns to the scenario page without the restore.
- **The debriefing's rematch arm** (msg 0x5d) needs the in-match host loop; the page itself is
  reachable only once a match can end (nf-game's MP layer owns that).
- **Quick Game's map sample** is deterministic row 0 (§5).
- **`--mp` time units**: the existing `MpSettings::deathmatch` path hands minutes where the game's
  menu hands seconds (§4) — the main agent may want `--mp` to ×60 for parity.

## Live checks for the main agent (PCSX2; none runnable here)

1. `fieldprobe` MPSettings+0x1A4 (0x2A4944, w) after confirming Arena on the scenario wheel —
   expect 0x00000001; then +0x1A8 (0x2A4948, w) after confirming Skyrail — expect 0x07000024.
2. Watch MPSettings+0x1A0 (0x2A4940, w) on the confirm page's ✕ after setting Duration 10 —
   expect 600 (the ×0x3c).
3. Watch mpbots[1] @0x2DEEA9 (w) during a Quick Game — expect 3; then MPSettings+0x1B0 (0x2A4950)
   — expect 3.
4. Watch GameState+0x08 @0x2A3770 (w) on the confirm's ✕ — expect the selected arena code; then
   the loader's ResetMap_LevelToLoad breakpoint — the level argument must equal it.
5. On the Join page with one pad: press ✕ on panel 1 — DAT_002e1b08 (w) should become 1 and
   mp_stuff+0x360 (@0x2def60+0x360) should read 1, then 5 after the codename, and mpjoin+1
   (@0x2dee69) 1 after the second ✕; the page must change to 0x4000001a only after the last
   joined agent is ready.
6. Render check (lesson 6): the scenario wheel should show 5 rows with the selected one centred,
   the preview texture at the left (ring-masked 0x030000bf), and "This scenario is locked." with
   the half-alpha row when the cursor sits on Uplink on a fresh profile.
