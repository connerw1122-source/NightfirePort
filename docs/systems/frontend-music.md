# The front end's music (frontend-music)

Which audio the menus play — title, main menu, the list pages, the results page, multiplayer
setup — and over the pause page: what starts, what stops, what hands over to what. This doc is
the data + state + flow the port implements (`crates/nf-view/src/menu_music.rs`, the host wiring
in `crates/nf-view/src/lib.rs`, `Driver::set_primary` in `crates/nf-audio/src/sfx.rs`); the
DRAWING of every page is the main agent's. The in-game music system (level music, sections,
crossfades) is docs/systems/music.md — the front end sits almost entirely OUTSIDE it (§1), with
one exception: the credits (§4).

Tags: [M] decompile (work/decomp/ACTION.ELF, address + function), [D] disc bytes, [L] ELF .data
/ EE snapshot, [S] suggested. Every test uses numbers/ids only.

## 1. The front end has no MapMusic track

`EventList` 0x7000048 = `FRONTEND_LEVEL` (the front end's own "level" hashcode). Two readers
settle what that hash means to the audio:

- `UpdateMusicalEvents` 0x114e70 skips it outright: `if (EventList == 0x7000048) goto
  LAB_00115038;` before the `MapMusic` table walk [M 0x114e70:17]. No music track is ever
  looked up for the front end.
- `Sound_Ready` 0x1dceb8's `case 0x7000048` only loads a sound bank (§2) — no `SFXStartMusic`
  [M 0x1dceb8:94-102].

So the menus' music is NOT the music system. It is a pair of music-class SFX loops (§2), plus
one music-system track that only the credits page starts (§4).

## 2. The jingle pair and the bank alternation

The only sounds the front-end code ever starts are `SFXStart(0x1d7, 0)` and `SFXStart(0x470, 0)`
— always both, always together, guarded by `menu_movie == 0` [M the six restart arms in §3].
`menu_movie` is the byte at 0x2e1c48 [M action-symtab], reset to 0 by every `MenuManager_Create`
[M 0x21b908:127], set to 1 by the restart arms, set to 0 by the credits and by every movie start
(§4/§5).

**Which of the pair sounds is decided by the loaded bank, not by the caller.** `Sound_Ready`'s
front-end case alternates the bank hash by the static byte `DAT_002e1b11` [M 0x1dceb8:94-101]:

```c
case 0x7000048:
  if (DAT_002e1b11 == 0) { SFXLoadSoundBank(0xc); }
  else                  { SFXLoadSoundBank(0x38); }
  DAT_002e1b11 = 1 - DAT_002e1b11;
```

`DAT_002e1b11` persists across boots through the PTP data: `Boot_GetPTPData` writes it out
[M 0x1c8198:59] and `Boot_LoadPTPData` reads it back [M 0x1c8560:61]. `P_START_Handler`'s
first-boot arm (`once_295`) pins it to 1 [M 0x20a408:31] — inside the SAME 0x4c arm, AFTER the
pair restart, so it runs after the boot's own `Sound_Ready` already consumed the flip: on a
fresh boot (`DAT_002e1b11` = 0 from .bss) it is a no-op (the boot load already flipped it to 1)
that only defeats a PTP-restored 0. The anchored alternation: the boot front end loads hash 0xc
(jingle 0x1d7), the second front-end load takes 0x38 (jingle 0x470). The port boots fresh, so
the anchor holds without modeling the force.

The disc settles which id each hash carries [D]:

- `SBINFO.SBI` slot 0 = hash **0xc** → `SB_0.{SHF,SFX,SBF}`; slot 1 = hash **0x38** →
  `SB_1.{SHF,SFX,SBF}` (`SFXLoadSoundBank` resolves the hash through `SoundBankInfo` = SBINFO.SBI
  [M 0x2070 FindSoundBankSlot]).
- `SB_0.SFX` carries effect **0x1d7** (and NOT 0x470); `SB_1.SFX` carries **0x470** (and NOT
  0x1d7). Both banks carry the menu blips 0x1d8–0x1da. A mission bank (e.g. SB_12) carries
  NEITHER jingle — it still carries 0x1da.
- Both jingles are polyphonic 2-subtrack loops [D SB_0.SFX id 0x1d7 / SB_1.SFX id 0x470:
  MultiSample 1, Polyphonic 1, SampleCount 2, both subtracks' samples loop-flagged; ~36.7 s /
  ~43.8 s of VAG at the bank's pitch 2731, plus a ~0.46 s tick layer].

The game requests BOTH ids every time and the bank gates the audible result; this port issues
the id the loaded bank carries (`MenuMusic::jingle_id`), keeping every bank resident as spares
(`Driver::load_spare_bank` + `set_primary`). The audible result is identical.

Their volume is music-class: `ES_CombineVolumes` scales ids 0x1d7/0x470 by `MusicMasterVolume`
instead of the SFX chain [M docs/systems/sound.md §5.5; port: nf-audio sfx.rs].
`MusicMasterVolume` @ 0x30c958 boots at **70** [L ACTION.ELF .data 0x30c958]; the neighbouring
`SfxMasterVolume` @ 0x30c95c boots at 100 [L] — the two defaults are distinct, which is a useful
test discriminator.

## 3. The page → music table

The menu manager delivers msg 0x4c (page enter) to the page's handler; a page change (msg 0x44)
re-sends 0x4c to the NEW page [M Manager_SendMessage 0x21c858 case 0x44, the
`Manager_SendMessage(param_1,0x4c,page,…)` + `Script_PlayDefault(page,0x4c)` at 0x21c858:345-346].
Every 0x4c arm falls into one of three groups:

**A. Restart the pair when it is down** — `if (menu_movie == 0) { SFXRemove(0,0x1d7);
SFXRemove(0,0x470); SFXStart(0x1d7,0); SFXStart(0x470,0); menu_movie = 1; }`:

| Page | Handler | 0x4c arm |
|------|---------|----------|
| Start (boot) | `P_START_Handler` | [M 0x20a408:17-22] |
| Main menu | `P_MAIN_Handler` | [M 0x20a7b0:42-47] |
| Mission list | `P_NFMAP_Handler` | [M 0x20b608:36-41] |
| Results | `P_NFRESULTS_Handler` | [M 0x208e78:60-65] |
| MP debriefing | `P_MPDEBRIEFING_Handler` | [M 0x212198:85-90] |

These are the pages with the looping-FMV backdrop. An enter while the pair is UP sends nothing
(the `menu_movie == 0` gate) — the pair simply continues.

**B. The credits swap** — see §4.

**C. No music messages at all** — Codename, Difficulty, Dossier, Records, Options, the Cn*
pages, the whole MP setup chain (MpJoin/MpScenario/MpMap/MpSetup/MpOptions/MpRules/MpBots/
MpBotSetup) and ScreenAdjust send no SFXStart/SFXRemove from their 0x4c arms [M the handlers'
decomp; e.g. the MP setup chain carries none], so the pair keeps playing across every list page
and the whole multiplayer setup. That is the "music over the multiplayer setup" answer: nothing
changes it.

**The pause page** is not a front-end page (§6) — its 0x4c arm sends no music messages and its
audio hold is the manager create's pause pair.

## 4. The credits swap (the one music-system track)

`P_CREDITS_Handler` 0x2152b0:

- **Enter** (the 0x4c arm) [M 0x2152b0:89-93]: `music_vol_524 = SFXMusicGetVolume();`
  `SFXRemove(0,0x1d7); SFXRemove(0,0x470); menu_movie = 0; SFXStartMusic(0x27, 0);` — the saved
  volume, the pair removed, and the credits' own music-system track **0x27** started at section 0.
  Track 0x27 resolves through `MFXINFO.MXI` → `MFX_<n>.SMF`/`MFX_<n>.SSD`, the same loader the
  level music uses [M docs/systems/music.md; D the files are on the disc, test
  `the_credits_track_files_exist_on_the_disc`].
- **Leave** (the 0x4d arm) [M 0x2152b0:56-63]: `SFXStopMusic(); SFXMusicSetVolume(music_vol_524);`
  then the usual `if (menu_movie == 0) { …restart… }` — the credits always leave `menu_movie`
  at 0, so the pair always comes back with the restored volume.

Note the leave restores the volume saved at ENTRY, not the slider's last value: moving the AV
options' music slider while the credits play changes the credits' volume, and leaving the
credits undoes it (§8).

## 5. The movie backdrops' own audio vs the music

`Menu_PlayMovie` 0x215990 removes the pair (lines 14-15: `SFXRemove(0,0x1d7); SFXRemove(0,0x470);`)
and starts the movie with its OWN audio: `psiStartBackgroundMovie(movie, …, SFXGetVolume())`
[M 0x215990:14-19] — the movie's audio replaces the jingles, there is no mix.

The fullscreen-movie pages' 0x4c arms route through it (each: `SFXRemove` both, `menu_movie = 0`,
then `Menu_PlayMovie`): P_INTRO [M 0x209e88:42-44], P_ATTRACT [M 0x20a1f0:20-22], P_ESTHERO
[M 0x20a060:20-22], P_FMVPLAYER [M 0x20ad30:22-24], P_TRAILER [M 0x214f10:16-18], P_WINGAME
[M 0x215100:16-18].

When a movie finishes, the page's finish arm restarts the pair while it is down:
`if (DAT_002e1c4a != 0 && menu_movie == 0) { SFXRemove both; SFXStart both; menu_movie = 1; }`
[M P_INTRO 0x209e88:23-28, P_ESTHERO 0x20a060:34-39, P_FMVPLAYER 0x20ad30:36-41, P_TRAILER
0x214f10:34-39, P_WINGAME 0x215100:29-34] — between the two intro movies the pair comes back,
and after the last one it is up when Main's 0x4c arrives (which then sends nothing).

## 6. The pause page

The in-game pause is a MenuManager with `pause_sound = 1`: `MenuManager_Create` runs
`SFXPause()` + `SFXPauseAllStreams()` when its `param_6` is set [M 0x21b908:121-124], and
`MenuManager_Delete` undoes it: `SFXUnPause(); SFXUnPauseAllStreams(); menu_movie = 0;`
[M 0x21c2b0:11-15] (the delete also forces the next enter to restart the pair). The pair is
never removed BY the pause — it is held under the pause and resumed after.

`GameState+0x50` is read by NO sound code — the pause rides these RPC ops entirely (docs/systems/
mission.md §5/§8). The port's hold is the pause page being up (or the viewer's P pause):
`menu_hold = self.paused || fe.page == Page::Pause` in `crates/nf-view/src/lib.rs` — NOT the
front end's own pages, whose manager passes pause_sound = 0 and whose pair keeps playing over
every page [M §3 group C].

## 7. Mission end → front end handover

Two paths lead back to the front end, and both hand the music over the same way:

1. **The fade down**: the pause page's RESTART and QUIT confirms, the mission-end page and the
   MP confirm all run `SFXFadeDown(1)` right before `GameFlow_PushState(…,7,…)` +
   `MenuManager_Delete` + `ResetMap_LevelToLoad` [M C_GCPAUSE 0x204408:149-151 — both arms
   converge; P_ENDMISSION 0x216618:28; P_MPCONFIRM 0x20ec00:155]. That is the 2 s music
   fade-down (`FadeStatus 1`, `FadeMusic` copies `FadeVolume` — docs/systems/music.md), the
   level's music dying out under the load. The port: `MenuMusic::quit_fade` → `MusicCmd::FadeDownMusic`
   → `Driver::fade_down(true)`, called from `FrontEnd`'s restart and quit arms [M same sites].
2. **The reload resets the state**: the front end load runs `Sound_Ready`'s `case 0x7000048`
   (bank flip, §2) and `MenuManager_Create` resets `menu_movie` [M 0x21b908:127], so the first
   page enter restarts the pair with the OTHER bank's jingle.

Over a MISSION (the results page opening before the front end reloads — mission end → results),
`P_NFRESULTS`' 0x4c still issues the restart pair [M 0x208e78:60-65], but the mission's bank
carries neither jingle [D SB_12 probe] so nothing sounds until the front end reloads. The port
mirrors exactly that: the state machine queues the commands, the host's gate only sounds them
when the front end is the loaded level [M lib.rs `menu_music_tick`'s `front_end_loaded` gate].

## 8. The volume knobs

- **AV options music slider** (P_CNAVOPTIONS 0x214818): the 0x50 arm runs
  `SFXMusicSetVolume(slider * 5)` [M 0x214818:72]; the slider's range is 0..20 steps
  [M the 0x27 slider message carries 0x14 = 20] → 0..100. The cancel arm restores the ENTRY
  values: `SFXMusicSetVolume(start_music_vol_497 * 5)` [M 0x214818:80]. (The same arm sets the
  sfx volume through `SFXSetVolume` — that one is the sfx chain, not music.)
- **Cheats page MUSIC toggle** (C_CHCHMUSIC 0x215bf0): `SoundInfo = 100 / 0;`
  `SFXMusicSetVolume(SoundInfo)` [M 0x215bf0:13-16] — 100 on, 0 off.
- `MusicMasterVolume` boots at 70 [L 0x30c958] — the menu music plays at 70% until a slider
  moves it.

## 9. The port

- **`crates/nf-view/src/menu_music.rs`** — `MenuMusic` (the state: `jingles_up` = `menu_movie`,
  `bank_flip` = `DAT_002e1b11`, `credits_saved_vol` = `music_vol_524`, `credits_music`,
  `music_master` = `MusicMasterVolume`) and `MusicCmd` (the IOP-side effects: `StopJingles`,
  `StartJingle(id)`, `StartMusic(track, section)`, `StopMusic`, `MusicVolume(v)`,
  `FadeDownMusic`). Methods: `front_end_sound_ready` (the load's bank choice + flip),
  `page_enter` (the §3 table), `credits_leave`, `movie_started`/`movie_finished` (§5),
  `quit_fade` (§7), `music_slider`/`cheat_music_toggle` (§8). Commands queue; the host drains.
- **The host** (`crates/nf-view/src/lib.rs`): `menu_music_tick()` runs per redraw beside
  `movie_tick` — applies the page-change transitions (Credits leave on the page changing away,
  then the new page's enter [M §3/§4]), drains the queue onto the sound driver, and pumps the
  driver once per frame while the menus are up. The boot (`run()`, the front-end level) and the
  async-load hook (`poll_async_load`'s swap — master's `start_async_load` worker lands the
  scene in `swap_in_scene`, the hook runs right after it) both consume the flip:
  `front_end_sound_ready()` → `Driver::set_primary(slot)` → `jingles_up = false` (the
  `MenuManager_Create` reset) → `fe_music_page = None` (the re-create re-runs the page's 0x4c).
  `demo_results` (the F8/F9 stand-in) inherits `bank_flip`/`loaded_bank`/`music_master` onto
  its fresh `FrontEnd` — the statics outlive any one manager [M 0x1dceb8's `DAT_002e1b11` is a
  .bss byte]. [S] A mission started straight from the command line never sees the boot front
  end, so its first front-end return flips 0xc one step behind the game's absolute sequence.
  Silent under NF_SILENT: without an output device `build_sound` returns None and the commands
  only bookkeep on the state machine — the tests exercise the state machine either way.
- **`crates/nf-audio/src/sfx.rs`** — `Driver::set_primary(slot)`: re-points the first lookup at
  the flipped bank (the port keeps banks resident; a no-op when the slot is not resident).
- **No drawing**: the state, flow and getters are all the draw code needs (`fe.music`).

## 10. Tests

`crates/nf-view/tests/menu_music_flow.rs` — numbers/ids only, each with a sister assertion that
can fail:

- Headless: the fresh state (70 vs 100, jingle 0x1d7), the five restart pages' exact command
  lists, the no-op walk over the list pages + MP setup, the bank alternation (0xc → 0x38 → 0xc,
  0x1d7 ↔ 0x470), the intro movie swap (stop/restart/stop/restart, then Main enters nothing),
  the credits swap (save 70 → track 0x27 → slider 13 = 65 → leave restores 70 and restarts the
  pair), the quit fade, the volume knobs (slider 0/20 → 0/100, cheat 0/100), the full
  boot → mission → results → front end handover walk, and the action-driven walk through
  `FrontEnd::action`.
- NF_ISO-gated sisters (real disc bytes): SBINFO.SBI slots 0/1 = hashes 0xc/0x38; SB_0 carries
  0x1d7 + the blips but not 0x470, SB_1 the mirror; a mission bank (SB_12) carries neither
  jingle but still 0x1da; the credits track 0x27 resolves through MFXINFO.MXI to a parseable
  SMF + non-empty SSD.

What ran before the push: see docs/reports/frontend-music.md.
