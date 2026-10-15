# Menu SFX — the front end's and pause page's sounds

Build: ACTION.ELF `e7ffdcb3…` (work/decomp/ACTION.ELF), banks from SLUS-20579
(PS2/ENGLISH/SB_0/…). Port: the event list on `FrontEnd` (`crates/nf-view/src/menu.rs` —
`MenuSfx`, `sfx_out`, `drain_sfx()`); the host plays the drained ids through `nf-audio`
(`Driver::start`, the 2-D path) exactly where it drains. Tags: [M] code (function@addr, line
numbers = the decompile file), [D] our disc, [L] EE snapshot (work/ee_start_*.bin; file offset =
EE address), [S] suggested. Companion docs: docs/systems/sound.md (the sound engine), sound-
coverage.md (the on-foot audit; its "Front-end (N/A)" row is this system).

## 1. `Menu_PlaySound(MENU_SOUND)` 0x1f3278 — the engine's one sound door

Every engine-level menu sound funnels through `Menu_PlaySound` 0x1f3278 (size 176): a switch maps
the enum to a bank id, four values check `enabled_sound_flags` 0x30d184, then a non-zero id plays
`Sound_Play(100.0f, id, 0, 0)` (volume 100, 2-D) [M]. The full map (the ids [D] in the banks, §3):

| MENU_SOUND | id | Gate (flags &) | Played by | When |
|---|---|---|---|---|
| 0 | 0x1da | 1 | Button_Update 0x1ef948:106,110; Label_Update 0x218cb8:236; Checkbox_Update 0x1efef8:65; Combo_Update 0x1f0498:124; Memo_Update 0x21e490:300; Scroll_Update 0x221b20:472 | ✕ (action 0x1f edge) on the focused control — the select blip; the widget sends its message (0x4b select / 0x61) with it [M] |
| 1 | 0x1da | 2 | the same widgets (:123/:252/:…; Scroll :468) | □ (action 0x20 edge) — msg 0x5d [M] |
| 2 | 0x1da | 4 | the same widgets (:119/:243; Scroll :461) | ○ (action 0x21 edge) — msg 0x5e [M] |
| 3 | 0x1d8 | — | Scroll_Update 0x221b20:204 | list/spinner up ◀ / ▲, first step [M] |
| 4 | 0x1d9 | — | Scroll_Update :188 | list/spinner down ▶ / ▼, first step [M] |
| 5 | 0x1d8 | — | Scroll_Update :228 | up, held auto-repeat (every 5th frame; every 2nd once the hold counter passes 0x1fa — :127-130, :162) [M] |
| 6 | 0x1d9 | — | Scroll_Update :177 | down, held auto-repeat [M] |
| 7 | 0x1da | — | Page_Update 0x21edc8:51 | the cursor moved onto a different control of the SAME page (the focus sprite pos vs the controls, `Menu_GetControl` 0x1f14f8 walks the page's controls by cursor xy) [M] |
| 8 | 0x1d9 | 8 | MenuManager_Update 0x21c3d8:101 | △ (action 0x22 edge) and the focused control runs no 0x5b script: the buzz, then `Manager_SendMessage(0x5f, 0, pad)` pops the page stack (0x21c858 case 0x5f → msg 0x44 with flag 5) [M] |
| 9 | — | — | Scroll_Update :457 | the △ arm calls `Menu_PlaySound(9)`, the switch default leaves id 0 → **silent**; msg 0x6d still goes out [M] |

`enabled_sound_flags` 0x30d184 = 0x0b at the boot/EE snapshots — MENU_SOUND 0, 1 and 8 enabled,
2 (○) disabled, so ○ is silent on the disc while □ blips and △ buzzes [L ee_start @0x30d184;
M the gates]. The flags also feed the in-game "sound" options ([S] no writer found in the
decompile — the store is dropped; live check below).

The actions: 0x1f = ✕, 0x20 = □, 0x21 = ○, 0x22 = △, 0x1e = Start, 0x1a/0x1b = d-pad ▲/▼,
0x1c/0x1d = ◀/▶ [M docs/systems/input.md §3, style-0 table; `Menu_InputAction` 0x1f1f40's mutual
suppression — △ reads 0 while ✕/Start are held and vice versa]. `Menu_InputAction` itself plays
nothing: it is the input poll (it aggregates `Input_Action` 0x17f208 over the pads and swallows
Start/✕ edges while the manager status > 3, returning the claimed edge [M 0x1f1f40:17-157]).

`Manager_SendMessage` 0x21c858 plays no sound; it routes what the sounds accompany: 0x44 = page
show/hide (the page's open script 0x4c / close script 0x4d run through `Script_PlayDefault`
:346-374), 0x5f = the △ pop (:567-611), 0x68 = input disable, 0x42 = widget construction [M].
The spinner (`Spin_Update` 0x222948:50,55) and every other value row delegate their input to
`Scroll_Update` (an embedded `M_SCROLL`), which is where the 3/4/5/6 sounds come from; the
direction split keys on the widget's flag bit 2 (+0x13c): clear → ◀/▶ are the steps (0x1c dec =
0x1d8, 0x1d inc = 0x1d9), set → ▲/▼ are (:111-260). Both agree semantically: decrement 0x1d8,
increment 0x1d9. `List_Update` 0x21a598, `Radio_Update` 0x21ff80, `Text_Update` 0x1b1510,
`Window_Update` 0x224270 call no sound function [M grep over work/decomp/ACTION.ELF].

## 2. The page-open pair, `menu_movie`, and the banks

The twelve top-level page handlers play the open pair on msg 0x4c: `SFXRemove(0, 0x1d7)`,
`SFXRemove(0, 0x470)`, then `SFXStart(0x1d7, 0)` + `SFXStart(0x470, 0)`, gated by the static
`menu_movie` (plays only when it is 0, sets it 1 after) [M P_NFRESULTS 0x208e78:63-65, P_INTRO
0x209e88:23-28, P_ESTHERO 0x20a060:34-39, P_ATTRACT 0x20a1f0:48-49, P_START 0x20a408:12-21,
P_MAIN 0x20a7b0:36-47, P_FMVPLAYER 0x20ad30, P_NFMAP 0x20b608, P_MPDEBRIEFING 0x212198,
P_TRAILER 0x214f10, P_WINGAME 0x215100, P_CREDITS 0x2152b0]. `menu_movie` resets to 0 in the
movie-end paths of P_INTRO/P_ESTHERO/P_ATTRACT (0x209e88:44,58 et al.) — so the pair plays once
per "menu session": at boot, and again only after a movie page ran [M]. The sub-pages (P_CNSELECT,
P_CNMENU, the options pages, the pause C_GCPAUSE 0x204408) play nothing on open [M grep].

Which of the pair actually sounds is the bank's decision:
- The front end is level 07000048; `Sound_Ready` 0x1dceb8:91-96 loads bank hash **0xC or 0x38**
  for it, flipping `DAT_002e1b11` each time (`if (DAT_002e1b11 == 0) SFXLoadSoundBank(0xc) else
  SFXLoadSoundBank(0x38); DAT_002e1b11 = 1 - DAT_002e1b11`) — strict alternation per (re)load [M].
- Bank hash 12 = slot 0 = `PS2/ENGLISH/SB_0/SB_0.SFX`: effect table {0x1d7, 0x1d8, 0x1d9, 0x1da,
  0x1db} — 5 effects [D the file's (id, offset) pairs]. Bank hash 56 = slot 1 = `SB_1.SFX`:
  {0x1d8, 0x1d9, 0x1da, 0x1db, 0x470} [D]. So **0x1d7 lives only in SB_0 and 0x470 only in SB_1**:
  each visit sounds exactly one half of the open pair, alternating with the bank [S the combined
  reading; the per-start failure is silent — the IOP's (hash, offset) lookup "not found → return 1",
  docs/systems/sound.md §2].
- Mission banks duplicate the UI ids so the pause page sounds over a paused mission: SB_4.SFX
  (07000005) and SB_5.SFX (07000009) both carry 0x1d8/0x1d9/0x1da and neither 0x1d7 nor 0x470 [D].
  The pause page never plays the pair anyway [M].
- `SFXStart` 0x115a68 dies when the global `switch_CONFIG_SOUND` 0x30ceac is 0 (checked first,
  returns 1 doing nothing) [M 0x115a68:8-10]; it is 1 in the snapshots [L ee_start @0x30ceac].
  `Sound_Play`'s pool path is the other entry (Menu_PlaySound → Sound_Play → the EE pool →
  SFXStart's batch) [M docs/systems/sound.md §1-2]. No writer of `switch_CONFIG_SOUND` shows in
  the C (dropped store) — [S] it is the options' sound toggle; live check below.

## 3. The port

`crates/nf-view/src/menu.rs`:
- `MenuSfx { sound: u8, id: u32 }` + the `SFX_MENU_*` / `MENU_SOUND_*` constants — `sound` is the
  MENU_SOUND value (0xff marks the open pair, which is the handlers' own `SFXStart` path, not a
  MENU_SOUND); `id` is the bank id [M the mapping in §1-2].
- `FrontEnd.sfx_out: Vec<MenuSfx>` accumulates in play order; `drain_sfx()` hands it to the host.
  The host plays each id through `nf-audio`'s 2-D start; a bank miss stays silent exactly like the
  game's lookup [S the host wiring — this task delivers the events, not the host loop].
- `FrontEnd.enabled_sound_flags: u8` = 0x0b [L] gates MENU_SOUND 0/1/2/8 in `sfx()` [M 0x1f3278].
- `FrontEnd.menu_movie: bool` gates `open_pair_sfx()`; `enter_page()` (every page change funnels
  through it) emits the pair on transitions into Start/Main/MissionSelect/Results/Credits/
  MpDebriefing — the port's stand-ins for the twelve pair-playing handlers [M the caller list;
  S the page correspondence: P_NFMAP = MissionSelect, P_NFRESULTS = Results, P_CREDITS = Credits,
  P_MPDEBRIEFING = MpDebriefing]. The boot emits it once for Start [M P_START's 0x4c]. The intro
  movie ends clear `menu_movie` [M 0x209e88:44,58] — the port does it at both `intro_playing`
  clear sites. The game's other resets (ESTHERO/ATTRACT) have no port counterpart yet (attract
  mode unported) [S].
- Emissions in `action()`: the widget-layer select/back sounds fire pre-dispatch on the remapped
  action (Confirm → MENU_SOUND 0, Back → MENU_SOUND 8, Side → MENU_SOUND 2) when the cursor sits
  on a control (`sfx_selectable()`: the rowed pages' row, the CnName grids' cell; ScreenAdjust's
  bars select nothing [L page 0x40000047's controls — no label rows; S the no-blip reading]). Dead
  rows blip in the game too — the widget plays before the handler decides [M Button_Update plays
  on the raw edge; S the port's dead rows behave the same]. Cursor moves emit MENU_SOUND 7 through
  `focus_moved()` at each handled move [M Page_Update]; the grid `cn_move` emits only when the
  cell changes (a clamped edge move keeps the focus, and Page_Update only sounds on a change) [M].
  Value steps emit 3/0x1d8 (decrement) and 4/0x1d9 (increment) in `opt_step` only when the value
  moved [M Scroll_Update's min/max guards; the List rows wrap, matching the game's wrap-flag rows].
  The held-repeat ids (5/6) are the host's key-repeat concern — the FrontEnd has no repeat clock
  [S]. The pause page's Esc-resume is the host's shortcut before `action()` and emits nothing (the
  game's △ on the pause root would buzz+pop through the manager [M 0x21c3d8]; live check below).

## 4. Tests

`crates/nf-view/tests/menu_sfx_flow.rs` — numbers only (the MENU_SOUND values and the SFX ids),
no disc reads (no ISO gate), nothing opens an audio device (the FrontEnd never touches nf-audio;
the runner's NF_SILENT stays unused). Each assertion has a sister that can fail: the gate test
clears `enabled_sound_flags` and asserts silence; the move test asserts the top-row Up emits
nothing; the value tests assert a clamped slider and an Action row emit nothing; the pair test
asserts the second open is silent.

## 5. Live checks for the main agent

1. **The open pair's alternation**: boot to the title, then `gdbwatch` `SFXStart` 0x115a68's `$a0`
   (first arg = the id) through boot: expect 0x1d7 AND 0x470 attempted, with only the loaded
   bank's half sounding; hard-reset twice — the sounding half should flip (Sound_Ready's
   `DAT_002e1b11` alternation [M 0x1dceb8:91-96]). Also confirms whether the pair really sounds on
   the FIRST boot or only after the intro page (the game's boot opens P_INTRO first [S]).
2. **The △ buzz**: on the main wheel and on a sub-page (e.g. Codename options), press △ with
   gdbwatch on `Menu_PlaySound` 0x1f3278's `param_1`: expect MENU_SOUND 8 → 0x1d9 played [M
   0x21c3d8:101] — and check whether any page carries a 0x5b script that suppresses it (the buzz
   should not fire there).
3. **`enabled_sound_flags` / `switch_CONFIG_SOUND` writers**: poke nothing; gdbwatch writes to
   0x30d184 and 0x30ceac while toggling the AV options' sound rows: the writer addresses name the
   option plumbing the decompile's stores dropped [S both are options-driven].
4. **The completed-results blip**: finish a mission; on P_NFRESULTS (no row list [L page
   0x40000036]) press ✕ — the port blips MENU_SOUND 0 (rows()==1 stand-in); confirm the real page
   blips too (the hint's ~A control is the only candidate [S]).
5. **The value rows' wrap flag**: on any options list row that wraps (e.g. a List row), step past
   the end: the game replays the first-step id on wrap only when the widget's wrap flag (+0x144)
   is set [M 0x221b20:170-193]; the port always wraps its List rows and always sounds. Confirm a
   no-wrap row exists (a clamped row would make the port's wrap a deviation).
