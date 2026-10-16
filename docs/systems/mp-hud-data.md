# mp-hud-data — the multiplayer HUD's data model

Status: implemented (the snapshot struct + getters + the headless tests). Evidence tags: [M]
code (function + address), [D] disc, [L] EE snapshot, [S] suggested / port-only design.

## 1. What this is

In an MP match the HUD shows per-agent info (score / frags, the match timer, the scenario's
objective status, the radar's blips). The game reads all of it straight out of the MP globals
every frame: `HUD_Update` 0x19a088 walks the pane list and calls each pane's update fn pointer
[M 0x19a088:60-63; the MP set is `MPPaneList`, picked by `HUD_Init` when `MPSettings+0x384 != 0`
[M 0x199e88:36]], and the panes read `MPGame`/`MPSettings`/the scenario globals directly. This
task ports that read side as a **per-frame snapshot struct with getters** — `nf_game::mp::hud`
(crates/nf-game/src/mp/hud.rs) — that the main agent's draw code consumes. Nothing here renders.

## 2. The reads, and what the snapshot carries

### The score pane (`HUD_MPUpdatePane` 0x19f0a0)

The pane is **one per viewer** (keyed `BLData+0x94e` [M 0x19f0a0:31]) — there is NO in-match row
sort. The text is the team line over the player's own line [M 0x19f0a0:56-70; the Ghidra vararg
loss at :68 — the second `%s` is the player string], tinted `0xff0000ff` for team 0 [M :53-55].

- `MP_GetPlayerScore` 0x189730: scenario 0x400 (Assassin) and 0x40000800 (KOTH FFA) print THREE
  lines — score `MPGame+slot·0x30+0x18`, kills `+0x04`, deaths `+0x08` [M :13-19]; every other
  scenario prints two — kills, deaths [M :20-27]. → `MpScoreRow::score_lines(sc)`.
- `Menu_GetMPScore` 0x203af8 (the debrief's sort key, and the pane's score read for the
  kill-less scenarios): the slot's score for the scoring scenarios, KILLS for 0x1 (Arena) and
  0x20000002 (Team Arena) [M 0x203af8:44-54]. → `MpScoreRow::display_score(sc)`.
- `MP_GetTeamScore` 0x18b338: `"%d - %d"` from `MPGame + team·4 + 0x180` [M :20-34]. →
  `snapshot.team_scores`.
- The **sort**: `P_MPDEBRIEFING_Handler` 0x212198's bubble pass, descending by the display score
  [M 0x212198:112-130]. → `snapshot.sorted_rows()` (descending by `display_score`; equal scores
  keep slot order — the bubble's no-swap case). The in-match rows stay in slot order (humans
  0..human_count, bots 4..4+bot_count — the game's walk [M 0x212198:92-111, 0x204408:327-399]).

### The match timer (`MP_CheckForEndCondition` 0x184268 + `Timer_Seconds2String` 0x1deaa8)

- **The clock is `MPGame+0x190`** (the decomp's `MPGame._400_4_` is a DECIMAL print — hex
  +0x190; `MPGame` is 0x1d0 bytes so +0x400 cannot exist). `limit` is `MPGame+0x194`, copied
  from `MPSettings+0x1a0` [M 0x183438:39] — **seconds**; the menu's minutes are ×0x3c at the
  confirm [M P_MPCONFIRM 0x20ec00:146]. The port's menu path already stores seconds in
  `MpSettings.time_limit` (mp_confirm_start's ×60); the `deathmatch()` default (10) is the
  pre-menu number — the report's deviations note.
- The HUD string: `Timer_Seconds2String((int)((limit − clock)·100), 2)` → mode 2 rounds to whole
  seconds, `ss = secs % 60`, `mm = (secs / 60) % 60`, `"%02d:%02d"`; negatives clamp to 00:00;
  hours never show in mode 2 [M 0x1deaa8:17-33]. → `snapshot.time_left` (+ `timer_string()`).

### The radar (`MP_GetRadarObjects` 0x189840 → `HUD_RadarUpdate` 0x19f6e8)

`MP_GetRadarObjects` fills the global `RObj` @0x318e70 (0x20-stride entries: 4×f32 position,
packed RGBA at +0x10, u16 kind at +0x14) [M]; `HUD_RadarUpdate` is its only caller and the
port's `nf_view::hud::radar_project` already implements its transform (0x19f6e8:108-153 — the
inverse viewer matrix, the unit direction, the `len·0.02` rim clamp, the dead height cue) [M,
checked against the disassembly]. The projection takes `pos/argb/kind` records — **this task
builds those records**:

- **Gate**: the viewer's per-player radar toggle `MPSettings + viewer·0x30 + 0x28` — 0 returns
  an EMPTY list and hides the ring [M 0x189840:22-24, 0x19f6e8:102-105]. The port's compact
  `MpPlayer` skipped +0x28 (its `armor_bonus` is +0x2c); the new `MpPlayer::radar_on` carries it
  (own line, WORKLOG lesson 4).
- **Player blips** (kind 0): every live slot but the viewer [M 0x189840:37-40]; team games tint
  team 0 `0xd22d35ff` / team 1 `0x2d61d2ff`, FFA tints everyone `0x7f7f7fff` [M :54-62]. The
  game's dead/limbo gates ride the `obj+0xff` state byte + BLData reads [M :41-92;
  `MP_playerIsDead` 0x18a938 pins the states: 0x01/0x11/0x12 dead, 0x02 dead unless the
  BLData+0x4f8-bit-0x100 + health exception] — the port's `MpSlot::dead` stands in for them [S,
  the BLData pipeline isn't modelled per-player yet].
- **Scenario markers** [M 0x189840:97-232]: CTF 0x20000004 → the two `Flags` entries, kind 1,
  red then blue; Uplink 0x60000008 → the `Uplinks` entries, kind 2, the terminal's live RGB
  packed (team 0 `(0xd2,0x2d,0x35)`, team 1 `(0x2d,0x61,0xd2)` — what `MP_UplinkUpdate`
  0x186d98:41-47 mirrors via `SP_SetColour`); Demolition 0x20000040 / Protection 0x20000080 →
  the site, kind 3, white; GoldenEye 0x20000200 → the items not yet picked up (`ext+0xf0 & 0x10`
  skips a held one), kind 4, white; Blueprint 0x20000100 → the blueprint kind 6 white plus the
  two espionage bases kind 7, red then blue. Kind 5 exists in the UV sheet but is written by
  nothing [M — no kind-5 arm in 0x189840].
- → `snapshot.radar: Vec<MpRadarObj>` (world space; `nf_view::hud::radar_project` does the
  viewer-relative projection — the blips are relative to the viewer through it, `x = −view.x`,
  `z = −view.z`, `len·0.02 ≤ 1` [M 0x19f6e8:108-153]).

### The objective status

- **Flags held**: the pane's flag icon asks `MP_HasTeamFlag(obj, team)` 0x18b430 — the flag
  ext's `+0x1c` CARRIER pointer, not the slot's 0x26 bit0 (only bot AI reads that bit [M
  0x126b88:133-153]). → `snapshot.flags: [Option<carrier slot>; 2]` from the port's
  `MpObjState::carrier` (the pointer's stand-in).
- **Uplink progress**: there is no progress counter — ownership is the ext's `+0xf4` word
  (0/1/2, `MP_getUplinkStatus` 0x18ba80 returns it verbatim) and the "progress" is the score
  drip (+0.2/s to the controller's personal and the owning team's total [M 0x186d98:59-80]).
  → `snapshot.uplinks: Vec<MpUplinkHud>` (owner, controller, world pos, live RGB) — the pane
  colors its icons from exactly this [M 0x19f0a0:134-180: status 0 → `0xd22d35ff`, 1 →
  `0x2d61d2ff`, 2 → `0x7f7f7fff`].
- **The Top Agent / Assassin target** (scenario 0x400): the globals are inverted vs their names
  — `DAT_0030d774` holds the ASSASSIN, `Assasin` @0x30d770 holds the TARGET [M 0x18b538:6,
  0x18b558:6]; the target's `MPSettings+slot·0x30+0x20` word is overwritten as the is-target
  marker [M 0x18aa70:38-41]. The pane shows the assassin/target icons [M 0x19f0a0:102-113]. →
  `snapshot.assassin` / `snapshot.target` (the port's `MpMatch::assassin`/`assassin_target`,
  already mapped) + `MpScoreRow::is_target`.
- **KOTH**: no hill meter and no radar blip (kind 5 unused) — the HUD shows only the 3-line
  score (scenario 0x40000800/0x60001000) and the 0x10 on-hill status bits (bot voice [M
  0x186ae0:95-115]). The port's rules.rs already skips the bit as presentation; the snapshot
  needs nothing extra.
- **Name tags**: gated on `MPSettings+0x1c4` (the port's `settings.radar`) or scenario 0x400,
  within 100 units + LOS; FFA `0x1e504bff`, teams `0x5a1414ff`/`0x14145aff` [M 0x19f6e8:175-264].
  → `snapshot.names_on_radar` (the colors are constants in the doc — the draw reads them here).

## 3. The struct

`MpHudSnapshot::build(&MpMatch, viewer_slot, viewer_radar_on)` → one struct per frame:
`viewer_slot`, `scenario`, `time_left`, `rows: Vec<MpScoreRow>` (+ `sorted_rows()`), 
`team_scores`, `flags`, `uplinks`, `assassin`/`target`, `names_on_radar`, `radar: Vec<MpRadarObj>`.
Getters: `MpScoreRow::display_score` / `::score_lines`, `MpHudSnapshot::timer_string`,
`radar_team_color`. The draw side converts `MpRadarObj` → `nf_view::hud::RadarObj`
(pos/argb/kind — the same three fields) and feeds `radar_project`.

## 4. Tests

`crates/nf-game/tests/mp_hud_data.rs` — headless (no ISO), a match's snapshot matched against
the sim state, each with a sister assertion: the rows read the sim's own kill/death counters in
slot order and sort descending by display score (Arena = kills); the score-line count per
scenario family; the timer is `limit − clock` in seconds (`08:30` at 90 s of 600) with the
00:00 clamp and the minute wrap; the radar list excludes the viewer, drops dead slots, tints
teams red/blue and FFA white, and goes EMPTY with the toggle off; the flag carriers / uplink
owner+controller+RGB / the Assassin target marker ride the snapshot.

## 5. Deviations ([S], for the report)

- The per-player radar toggle is a NEW port field (`MpPlayer::radar_on`, the game's +0x28).
  Which menu row writes it (the Setup page's Radar row) is 611/611b's page state — the port's
  `FrontEnd.mp_radar` currently feeds `MpSettings.radar` (+0x1c4, the NAME-tag gate). A live
  check can settle which record the menu writes (below).
- Dead/limbo blip gates: the port's `MpSlot::dead` stands in for the `obj+0xff` state machine +
  BLData exception (0x189840:41-92); the port's players have no per-player BLData yet.
- `MpSettings.time_limit` semantics: the port's menu path stores SECONDS (the confirm's ×60
  folded in); the game stores minutes and converts at the confirm. The doc comment in mod.rs
  says minutes. Not changed here (shared struct) — flagged for the owners.

## 6. Live checks for the main agent

1. **The radar toggle's record**: on a 2-player split-screen MP match, poke
   `MPSettings + 1·0x30 + 0x28` (0x2a47a0 + 0x58 = **0x2a47f8**) to 0 with gdbwatch — expect
   player 2's radar ring to disappear while player 1's stays (access: write; the readers are
   MP_GetRadarObjects 0x189840:22 and HUD_RadarUpdate 0x19f6e8:102).
2. **The names gate**: poke `MPSettings+0x1c4` (**0x2a4964**) to 0 in an FFA match — expect the
   name tags to vanish while the blips stay (read at 0x19f6e8:177).
3. **The timer**: in a match with a 1-minute limit, watch `MPGame+0x190` (**0x2a4b10**)
   accumulate and the HUD string flip at 10 s left — confirms the seconds semantics and the
   mode-2 format (read at 0x184268:144-147).
4. **The assassin marker**: in scenario 0x400, check `MPSettings + target·0x30 + 0x20` = 1 after
   `MP_assassinReset` (write at 0x18aa70:38-41) — the port models the marker via
   `MpMatch::assassin_target` instead; the game's field is shared with the team word.
