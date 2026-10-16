# frontend-music — report

Task: port which music the front end plays (title, main menu, the list pages, the results page,
multiplayer setup) and over the pause page — the track ids, the start/stop/crossfade moments,
the movie backdrops' own audio vs the music, and the mission-end → front-end handover. Data +
state + flow + getters only, no drawing. Branch `s/frontend-music` (tip 49f9f460, merged with
origin/master 24b1f4dd).

## What's done

- **The system doc first**: `docs/systems/frontend-music.md` — §1 the front end has NO MapMusic
  (`UpdateMusicalEvents` 0x114e70:17 skips EventList 0x7000048; `Sound_Ready` 0x1dceb8's
  0x7000048 case only loads a bank), §2 the jingle pair + the bank alternation, §3 the
  page → music table, §4 the credits swap, §5 the movie backdrops, §6 the pause page, §7 the
  mission-end handover, §8 the volume knobs, §9 the port, §10 the tests. Every fact tagged [M]
  (function + address, re-verified against the decomp this session), [D] (disc bytes), [L]
  (ELF .data), or [S].
- **The state machine** `crates/nf-view/src/menu_music.rs` — `MenuMusic` (`jingles_up` =
  `menu_movie` 0x2e1c48, `bank_flip` = `DAT_002e1b11`, `loaded_bank` = the resident bank hash,
  `credits_saved_vol` = `music_vol_524`, `credits_music`, `music_master` = `MusicMasterVolume`
  @ 0x30c958, init 70 [L]; `SfxMasterVolume` 0x30c95c = 100 [L]) and `MusicCmd`
  (StopJingles / StartJingle / StartMusic / StopMusic / MusicVolume / FadeDownMusic) — the
  IOP-side effects, queued for the host.
- **The transitions**, all decomp-cited: the five restart pages `if menu_movie == 0 {
  SFXRemove(0,0x1d7); SFXRemove(0,0x470); SFXStart both; menu_movie = 1 }` (P_START 0x20a408
  :17-22, P_MAIN 0x20a7b0:42-47, P_NFMAP 0x20b608:36-41, P_NFRESULTS 0x208e78:60-65,
  P_MPDEBRIEFING 0x212198:85-90); the no-op list pages + the whole MP setup chain (the pair
  continues — that is the multiplayer-setup answer); the credits swap (enter 0x2152b0:89-93:
  save the volume, remove the pair, `SFXStartMusic(0x27,0)`; leave :56-63: `SFXStopMusic`,
  restore the ENTRY volume, the pair back); the movie backdrops (`Menu_PlayMovie` 0x215990
  :14-15 removes the pair, `psiStartBackgroundMovie(…, SFXGetVolume())` :18-19 replaces it
  with the movie's own audio — no mix; the finish arms 0x209e88:23-28 / 0x20a1f0:34-39 /
  0x215100:29-34 restart it); the pause page (`MenuManager_Create` 0x21b908:121-124
  `SFXPause`+`SFXPauseAllStreams`, Delete 0x21c2b0:11-15 undoes it — the pair is held, not
  removed); the quit fade (`SFXFadeDown(1)` at 0x204408:150, 0x216618:28, 0x20ec00:155); the
  AV music slider (×5, 0..20 steps, cancel restores the entry value — 0x214818:72/80); the
  cheats MUSIC toggle (0/100 — 0x215bf0:13-16); the page-change msg 0x44 re-sends 0x4c to the
  new page (0x21c858:345-346).
- **The bank alternation**, settled against the disc: `Sound_Ready` case 0x7000048 loads bank
  hash 0xc / 0x38 by `DAT_002e1b11` then flips it [M 0x1dceb8:94-101]; `DAT_002e1b11`
  persists through the PTP data [M 0x1c8198:59, 0x1c8560:61]; `P_START`'s once arm pins it to
  1 after the boot page's enter [M 0x20a408:31 — same 0x4c, after the pair restart, so on a
  fresh boot it is a no-op]. Disc probe: SBINFO.SBI slot 0 = 0xc = SB_0 carries jingle 0x1d7
  (not 0x470), slot 1 = 0x38 = SB_1 carries 0x470 (not 0x1d7), both carry the menu blips
  0x1d8-0x1da, a mission bank (SB_12) carries neither jingle. Both jingles are polyphonic
  2-subtrack loops [D]. The game requests BOTH ids and the bank gates the audible result; the
  port issues the resident bank's id (`loaded_bank`, init 0xc = the boot's own load).
- **The mission-end handover**: the results page's enter still issues the restart pair over a
  mission (the game's call is unconditionally made) but the mission bank resolves neither id —
  nothing sounds until the front end reloads, whose `Sound_Ready` flips the bank and whose
  `MenuManager_Create` resets `menu_movie`, so the first page enter restarts the pair with the
  OTHER jingle. The port mirrors it: the state machine queues, the host's `front_end_loaded`
  gate mutes the jingles off the front end.
- **Host wiring** (`crates/nf-view/src/lib.rs`): `menu_music_tick()` per redraw (page-change
  transitions, the command drain onto the driver, the per-frame pump); the flip consumed at
  boot (`run()`, the front-end level) and at every front-end reload (`poll_async_load`'s Ok
  arm — re-homed after master's async-level-load merge, keyed on `load.next ==
  FRONTEND_LEVEL`, right after `swap_in_scene`); `demo_results` inherits
  `bank_flip`/`loaded_bank`/`music_master` (the statics outlive a manager); the menu sound
  hold narrowed to the pause page only (the front end's own pages never pause sound); the
  intro movie glue calls `movie_started`/`movie_finished`. Silent under NF_SILENT (no output →
  the commands only bookkeep). `FrontEnd` gains the `music` field (own lines, lesson 4).
- **`crates/nf-audio/src/sfx.rs`**: `Driver::set_primary(slot)` — re-points the first lookup
  at the flipped bank (banks stay resident as spares; no-op when not resident).
- **No drawing**: the state, flow and getters are all the draw code needs (`fe.music`); the
  main agent's render pass is unaffected.

## How it was verified

- `crates/nf-view/tests/menu_music_flow.rs` — 14 tests, numbers/ids only: 10 headless (the
  fresh state incl. the 70-vs-100 discriminator, the five restart pages' exact command lists,
  the no-op walk over the list pages + MP setup, the bank alternation 0xc→0x38→0xc with
  0x1d7↔0x470, the intro movie swap sequence, the credits swap incl. the entry-volume restore,
  the quit fade, the volume knobs, the full boot→mission→results→front-end handover walk, an
  action-driven walk through `FrontEnd::action`) + 4 NF_ISO sisters that read real disc bytes
  (SBINFO slots 0/1 = 0xc/0x38; SB_0's 0x1d7 + blips and 0x470's absence; SB_1 the mirror; a
  mission bank lacking both jingles but keeping 0x1da; the credits track 0x27 resolving
  through MFXINFO.MXI to a parseable SMF + non-empty SSD). Each has a sister assertion that
  can fail. The first run caught two real bugs, fixed against the decomp: `jingle_id` must
  read the RESIDENT bank (added `loaded_bank`), and the boot's own flip must be consumed.
- **What I ran** (per-crate, foreground — the detached fast runner dies between tool calls in
  this sandbox, as in the three previous tasks; `suite.out` empty, 0 processes):
  - Crates the tests touch: `cargo test --offline --release -j2 -p nf-view -p nf-audio` →
    **145 passed / 0 failed**.
  - Post-merge flows: `menu_music_flow` 14/14, `menus_flow` 37/37.
  - Full suite on the FINAL merged tree (80cfabfb + 49f9f460): `cargo test --offline
    --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view --
    --test-threads=1` — **every binary green except the 3 slim-ISO `disc_golden` (the allowed
    trio) and `fix_0c_pylon_live` ×2 (the documented no-GPU environmental set, wgpu "adapter:
    NotFound")**. A capture of the same run reached **766 passed / 5 failed** (exactly those 5)
    before a tool timeout killed it mid `fix_guard_weapons_live`; that binary re-ran clean
    (3/3, 166 s).
- Merged origin/master (24b1f4dd): master's async-level-load moved the load onto a worker —
  the hook re-homed to `poll_async_load`; WORKLOG both kept; no shared file reordered.

## What's left

- The main agent's audible pass: the menus must SOUND in the live host (the draw is theirs) —
  the jingle pair over title/main/mission list/results/MP debriefing, silence of the pair over
  the movie pages, the credits' track 0x27, the pause hold.
- [S] The PTP-persisted `DAT_002e1b11` restore is documented but not modeled (the port boots
  fresh; the once-arm pin keeps the alternation anchored regardless).
- [S] A mission started straight from the command line never sees the boot front end, so its
  first front-end return flips 0xc one step behind the game's absolute sequence (doc §9).
- `Menu_RunMiniMission` 0x1ffb90 fades the same way as the quit fade (cited in
  menu_music.rs); mini missions are not ported — noted, no action here.

## Live checks for the main agent

If a debugger pass over the menus is convenient (PCSX2 + `tools/trace/gdbwatch.py`):

1. `MusicMasterVolume` @ 0x30c958 (ACTION.ELF load), read, boot → title: expect **70**.
2. `menu_movie` @ 0x2e1c48, read, on the title page after the intro movies: expect **1** (the
   pair up); on the P_INTRO page mid-movie: expect **0**.
3. `DAT_002e1b11` @ 0x2e1b11, read, at each front-end load (title → mission → results →
   front end → mission list): expect the flip 0 → 1 → 0 (bank 0xc, 0x38, 0xc) — i.e. the
   audible jingle alternates 0x1d7 / 0x470 / 0x1d7.
4. `music_vol_524`, read/write around the credits page: expect `SFXMusicGetVolume()`'s value
   on enter (70 fresh), restored on leave even if the AV slider moved mid-visit.
5. IOP RPC log on the pause page open: expect `SFXPause` (batch op 9) + `SFXPauseAllStreams`
   (op 0x16), and NO SFXRemove/SFXStart pair change.

## ntfy

(not run yet — sent right after this report; if it failed, the exact output goes here.)
