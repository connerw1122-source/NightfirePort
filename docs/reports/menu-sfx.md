# menu-sfx — the front end's and pause page's sounds

Branch `s/menu-sfx` (from origin/master @ 2bda7b46). Task 623.

## What's done

**Docs first: `docs/systems/menu-sfx.md`** (addresses + evidence tags throughout):

- `Menu_PlaySound(MENU_SOUND)` 0x1f3278 is the engine's one sound door. The full enum→bank-id map
  with the per-caller sites: MENU_SOUND 0/1/2 → 0x1da gated by `enabled_sound_flags` 0x30d184 bits
  1/2/4 (✕ select / □ / ○ side actions, played by Button_Update 0x1ef948, Label_Update 0x218cb8,
  Checkbox_Update 0x1efef8, Combo_Update 0x1f0498, Memo_Update 0x21e490, Scroll_Update 0x221b20 on
  the raw action edges, before the page handler decides); 3/5 → 0x1d8 and 4/6 → 0x1d9 ungated (the
  spinner/list ◀▲▼▶ first-step and auto-repeat ids — the spinner's `Spin_Update` 0x222948:50
  delegates to `Scroll_Update`); 7 → 0x1da (the cursor moved between two controls of the same page,
  Page_Update 0x21edc8:38-53); 8 → 0x1d9 gated bit 8 (△ back with no 0x5b script on the focus:
  MenuManager_Update 0x21c3d8:78-105, then msg 0x5f pops the page stack through Manager_SendMessage
  0x21c858 case 0x5f); 9 → **silent** (Scroll_Update's △ arm calls it, the switch default starts
  nothing [M 0x1f3278]).
- The open pair: the twelve top-level page handlers play `SFXStart(0x1d7)` + `SFXStart(0x470)` on
  msg 0x4c, gated by the `menu_movie` static (set after playing, cleared by the intro/attract
  movie-end paths) [M P_START 0x20a408:12-22, P_MAIN 0x20a7b0:36-47, P_NFRESULTS 0x208e78:63-65,
  P_INTRO 0x209e88:23-58 + 7 more].
- The bank story: level 07000048 loads bank hash 0xC or 0x38, flipping `DAT_002e1b11` on every
  (re)load [M Sound_Ready 0x1dceb8:91-96]. On the disc, SB_0.SFX (hash 12) holds exactly
  {0x1d7, 0x1d8, 0x1d9, 0x1da, 0x1db} and SB_1.SFX (hash 56) holds {0x1d8, 0x1d9, 0x1da, 0x1db,
  0x470} [D the files' effect tables, read from our ISO] — so exactly one half of the open pair
  sounds per front-end visit, alternating. Mission banks duplicate 0x1d8/0x1d9/0x1da (never
  0x1d7/0x470) [D SB_4.SFX, SB_5.SFX], which is why the pause page's blips play over a paused
  mission while the open pair cannot. `SFXStart` 0x115a68 also dies on the global
  `switch_CONFIG_SOUND` 0x30ceac (= 1 [L]) [M :8-10].
- Boot values: `enabled_sound_flags` 0x30d184 = 0x0b [L] — MENU_SOUND 0, 1, 8 enabled, 2 (○)
  disabled, so the disc's ○ is silent.

**The port** (`crates/nf-view/src/menu.rs`): `MenuSfx { sound: u8, id: u32 }` (sound = the
MENU_SOUND value, 0xff = the open pair's SFXStart path) + the `SFX_MENU_*`/`MENU_SOUND_*`
constants; `FrontEnd.sfx_out` + `drain_sfx()` (the host plays the drained ids through nf-audio's
2-D start — a bank miss stays silent exactly like the game's lookup); `enabled_sound_flags: u8`
= 0x0b [L] gating inside `sfx()` [M 0x1f3278]; `menu_movie: bool` gating `open_pair_sfx()`, with
`enter_page()` funnelling all 65 page transitions and emitting the pair into Start/Main/
MissionSelect/Results/Credits/MpDebriefing (the port's stand-ins for the twelve pair pages
[M the caller list; S the page correspondence]); the intro-movie ends clear `menu_movie`
[M 0x209e88:44,58]. Emissions in `action()`: the widget select/back sounds fire pre-dispatch on
the remapped action when the cursor sits on a control (`sfx_selectable()`; dead rows blip in the
game too — the widget plays before the handler decides [M; S the port's dead rows]); 30 cursor-move
arms emit MENU_SOUND 7 via `focus_moved()` (wrapping wheels tick too — a wrapped cursor is still a
focus change [M Page_Update sounds on a change]); `cn_move` ticks only when the cell changes;
`opt_step` emits 3/0x1d8 (decrement) and 4/0x1d9 (increment) only when the value moved
[M Scroll_Update's min/max guards]. No drawing; nothing in the FrontEnd touches the audio stack.

**Tests**: `crates/nf-view/tests/menu_sfx_flow.rs` — 9 tests, numbers only (MENU_SOUND values +
SFX ids), no disc reads (so no ISO gate), no audio device opened. Each has a sister that can fail:
the gate tests clear the flag bit and assert silence; the move test asserts refused moves and
clamped grids emit nothing; the value test asserts a slider pinned at 100 and an Action row emit
nothing; the pair test asserts the second open is silent.

## How it was verified

- Merged origin/master (8d0244aa — the PC-menus mouse support, which refactored the host's
  dispatch into `menu_input` and added `mouse_move`/`mouse_click`): resolved both conflicts
  (WORKLOG both entries kept; lib.rs took master's `menu_input` refactor and the sound drain now
  lives inside it, so keyboard, wheel and right-click paths all play). Master's mouse paths emit
  too: `mouse_move`'s hover-select is the game's cursor-over-focus change and ticks MENU_SOUND 7
  [M Page_Update 0x21edc8:38-53 — Menu_GetControl picks the control under the cursor sprite];
  `mouse_click`'s wheel-neighbour scroll ticks, its bare-row click blips MENU_SOUND 0, and every
  other route goes through `action()`'s hook.
- `cargo test` (NF_SILENT) on the final merged tree: **menu_sfx_flow 9/9**, menus_flow 37/37
  (master added one), codename_entry 14/14, mp_menu_flow 9/9, objective_hints_flow 7/7,
  campaign_flow_live 5/5 — 81/81, no regressions.
- The background full suite (`tools/test/suite.py`, started 6 min into the session) ran the
  pre-edit tree; it was still running at the deadline. Its failures are pre-existing and
  environmental, none in code I touched: the 3 `disc_golden` tests (expected on the slim ISO),
  `walkreal_07000001_live::walk_from_the_spawn_to_the_exit_with_the_contact`
  (assertion "the contact must walk his route to the LoadPointActivator plate" — sim AI, nf-game),
  and `fix_0c_pylon_live` ×2 (`no_adapter_backends` — this sandbox has no GPU; the test needs a
  render surface).

## What's left

- The host's play loop: `drain_sfx()` exists but `nf-view/src/lib.rs` does not yet play the
  drained events through `nf-audio` (the task's "sound event list the host plays" — the list and
  getter are delivered; the wiring is ~5 lines at the `fe.action(a)` call site plus a bank state
  for the front-end level). Tests stay silent either way.
- The held-repeat ids (MENU_SOUND 5/6) need the host's key-repeat clock; the FrontEnd emits the
  first-step ids only [S docs/systems/menu-sfx.md §3].
- The port's front-end bank state (Sound_Ready's 0xC/0x38 alternation) is not loaded in the host's
  menu boot; the spares mechanism makes both open-pair ids resolvable, so the port plays both
  halves where the game alternates — documented, not modelled.
- Esc-resume on the pause page is the host's shortcut before `action()` and emits nothing; the
  game's △ on the pause root buzzes+pops through the manager [M 0x21c3d8].

## Live checks for the main agent

1. **Open-pair alternation**: gdbwatch `SFXStart` 0x115a68's `$a0` through two hard boots to the
   title: expect 0x1d7 AND 0x470 attempted each time, with only the loaded bank's half sounding
   and the sounding half flipping between boots [M Sound_Ready 0x1dceb8:91-96].
2. **The △ buzz**: press △ on the main wheel and on a sub-page with gdbwatch on `Menu_PlaySound`
   0x1f3278's enum arg: expect MENU_SOUND 8 (→ 0x1d9) [M 0x21c3d8:101]; check whether any page
   carries a 0x5b script that suppresses it.
3. **Flag writers**: gdbwatch writes to 0x30d184 (`enabled_sound_flags`) and 0x30ceac
   (`switch_CONFIG_SOUND`) while toggling the AV options' sound rows — names the option plumbing
   whose stores the decompile dropped [S].
4. **The completed-results blip**: on P_NFRESULTS (no row list [L page 0x40000036]) press ✕ — the
   port blips via its rows()==1 stand-in; confirm the real page blips.
5. **The value rows' wrap flag**: step a wrapping List row past its end — the game replays the
   first-step id only when the widget's wrap flag (+0x144) is set [M 0x221b20:170-193]; the port
   always wraps and always sounds. Confirm whether a no-wrap row exists.

## ntfy

(sent after this report; any failure output goes here)
