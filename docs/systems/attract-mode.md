# Attract mode — the title's no-input demo (`P_ATTRACT_Handler`)

What the front end does when nobody touches it: after 2700 input-less frames (45 s at the fixed
60 Hz) the title page — and the main menu — open the game's attract page, which plays a
background FMV (the attract movie) and returns to the page it came from when the movie ends or
the player presses the skip action. Everything here is read off ACTION.ELF's decompile; the
decompiler's known blind spots (dropped stores) were cross-checked against the disassembly where
it mattered (the `bSkipAttract` scan, below).

## The no-input counter

- `frame_counter` 0x30d164 increments once per menu tick [M `Menu_AlphaUpdate` 0x1f3170 —
  `frame_counter = frame_counter + 1`, called from `MenuManager_Update` 0x21c3d8:34 every menu
  frame].
- `last_action` 0x30d168 is set to `frame_counter` on every handled menu input [M
  `Menu_ResetNoInputCount` 0x1f30f0; `Menu_InputAction` 0x1f1f40 writes it at 12 sites — one per
  input-action branch of the dispatcher; the disassembly confirms each is a `sw gp, -29960(gp)`
  store, not a decompiler ghost].
- `Menu_GetNoInputCount` 0x1f30e0 returns `frame_counter - last_action`. The port's
  `FrontEnd::no_input_frames` counts the same thing; `FrontEnd::action` zeroes it on entry (the
  port collapses get/reset into one counter).

## The timeout

Two pages check the count, and only these two [M the only `Menu_GetNoInputCount` callers in
ACTION.ELF]:

- the title page — `P_START_Handler` 0x20a408, on its msg-0x50 tick:
  `if (Menu_GetNoInputCount() == 0xa8c) Manager_SendMessage(own page, 0x44, 0x40000035, 0)`;
- the main menu — `P_MAIN_Handler` 0x20a7b0, the same line.

`0xa8c` = 2700 frames = 45.0 s at the fixed 60 Hz. The compare is exact equality [M]; the count
can't skip the value because every tick adds 1 and any input zeroes it. Message `0x44` is the
manager's open-page message [M `Manager_SendMessage` 0x21c858 case 0x44]: it clears all queued
input actions, pushes the current page on the manager's page stack, and finally sends the new
page its msg-0x4c "open". `0x40000035` is the attract page's id (the menu-def PAGE record whose
aux-movie dword is 0 [L the menu-def parse pinned in `menu.rs::aux_movie_for`]).

Both openers reset the count when they (re)open: `P_START` calls `Menu_ResetNoInputCount` at the
end of its 0x4c [M 0x20a408 tail], and `P_MAIN` does the same [M 0x20a7b0:37] — so after an
attract run the 45 s clock starts over.

## What plays: the attract movies

`P_ATTRACT_Handler` 0x20a1f0, on its 0x4c open [M; the handler's C is complete here, the stores
it shows were spot-checked in the disassembly at 0x20a220-0x20a384]:

1. `movie.291` 0x30d368 (a 1-byte global) toggles.
2. The background movie code is picked:
   - opener id != 0x40000002 (the main menu), or the toggled `movie.291` now 1 → `0x073a0048`;
   - opener == 0x40000002 and the toggle now 0 → `0x073b0048`.

   The opener id arrives as the handler's sixth argument: `Manager_SendMessage` case 0x44 sends
   the page's 0x4c with d2 = the id of the page that was up before the switch [M 0x21c858:345,
   `uVar6 = *(old page + 0x18)`]. So from the title the code is always `0x073a0048`; from the
   main menu the two codes alternate A, B, A, … across successive timeouts.
3. The movie-state block `menu_movie` 0x2e1c48 (12 bytes) is set: +2 = 1 ("SFX parked for the
   movie"), +1 = 1 ("a movie is up"), +4 = 0 (the hold counter), +0 = 0 ("the menu music wants a
   restart").
4. The menu music pair is removed: `SFXRemove(0, 0x1d7)` and `SFXRemove(0, 0x470)` [M; the ids
   are the front end's two looping menu sounds — the same pair `P_START`/`P_MAIN` restart on
   open].
5. `psiStartBackgroundMovie(code, 0, SFXGetVolume())` 0x1081a8 — non-looping (loop arg 0), the
   single BGFMV slot replaces whatever page backdrop was looping [M `psiStartBackgroundMovie`
   restarts the one slot].
6. `Manager_SendMessage(menu manager 0x2e3cd8, 0x68, 1, 0)` [M 0x20a1f0:37]. Message 0x68 sets
   the manager's input-block byte (+0x1d4) to the wparam and calls `Input_ClearAllActions(-1)`
   [M 0x21c858 case 0x68] — page input is dead while the movie plays.

Both codes are in the `UsingMovie` whitelist [M 0x107eb8; the port's copy is
`movie::USING_MOVIE`], so they map to real files: `cdrom0:\movies\30_FPS\073A0048.PSS` and
`073B0048.PSS` [M the sprintf in 0x1081a8]. The on-foot slim ISO carries them as zero-byte stubs
[L]; their picture counts (the real attract lengths) are not measured in this sandbox — see
"Live checks" in the report.

## While it plays

`MenuManager_Update` 0x21c3d8 polls `psiMovieFinished` 0x1082e8 (`BGFMVPlaying == 0`) every
tick, before anything else touches input [M :37-43]:

- while the movie is unfinished, it polls `Menu_InputAction(-1, 0x19, 4)` — the newly-pressed
  skip action — and calls `Menu_StopMovie(0)` 0x215a50 on it. `Menu_StopMovie` checks
  `menu_movie+1` (set) and the hold counter `menu_movie+4` (0 — P_ATTRACT set it at open, and
  `Menu_UpdateMovieCounter` 0x215970 only decrements a non-zero counter), then un-pauses SFX,
  calls `psiStopBackgroundMovie` 0x108328 and sends 0x68/0 (input back on). The game's other
  page input — the 0x22 dispatcher that moves cursors and confirms — is gated on the block byte
  [M 0x21c3d8:70], so the skip action 0x19 is the ONLY input that reaches anything while the
  attract movie plays. In the port `Action::Confirm` is the 0x19 stand-in (the same mapping the
  intro skip uses [M GameFlow_Main 0x1c98c8 case 6]).
- `bSkipAttract` 0x30d2a6 is read in `P_ATTRACT_Handler`'s 0x50 branch [M the `lb gp, -29642(gp)`
  at 0x20a378] as an instant-skip flag, but a whole-program scan of ACTION.ELF (lui/gp-relative
  operand scan over every instruction word) finds no store to it — no code in the program writes
  it. It is a dead developer flag in this build [M the scan; the symbol is `bSkipAttract` in
  action-symtab.tsv]. The port does not model it.

## The return to the title

`P_ATTRACT_Handler`'s 0x50 tick [M 0x20a1f0:39-65]:

1. `psiMovieFinished()` true → the menu music comes back: `SFXStart(0x1d7, 0)`,
   `SFXStart(0x470, 0)` (guarded by `menu_movie+2` set and `menu_movie+0` clear, so it happens
   once), `menu_movie+0` = 1; then `Manager_SendMessage(0x2e3cd8, 0x68, 0, 0)` — the input block
   lifts.
2. The close block (guard `menu_movie+1`): `SFXUnPause` + `SFXUnPauseAllStreams`,
   `psiStopBackgroundMovie()`, another 0x68/0, then
   `Manager_SendMessage(own page, 0x5f, 0, 0)`.
3. Message 0x5f [M 0x21c858 case 0x5f] pops the page stack: the page the opener pushed (the
   title, or the main menu) is re-opened with lparam 5 (bit0 set — do not push again), and the
   saved cursor control is restored. The opener's 0x4c then runs `Menu_ResetNoInputCount`
   [M 0x20a408 tail / 0x20a7b0:37], and the 45 s idle clock restarts.

So the loop is: title/main —45 s idle→ attract movie —movie end or ✕/START→ back —0x4c reset→
45 s —… From the main menu the movie alternates 0x073a0048 / 0x073b0048; from the title it is
always 0x073a0048.

If the movie never starts (a missing file — `psiStartBackgroundMovie`'s `strFileOpen` failure
path hangs [M 0x108cc0; the port reports and falls back]), `psiMovieFinished` is true from tick
one (`BGFMVPlaying` is 0) and the page closes immediately [M the same 0x50 path]. The port's
headless player reproduces exactly that: a declined start reports finished at once.

## The port

State lives on `FrontEnd` (`crates/nf-view/src/menu.rs`); no drawing — the attract page has no
controls [L the menu-def's 0x40000035 record: aux 0, no control rows], so the draw side only
needs `attract_playing` + `attract_movie` to put the movie plane fullscreen.

- `ATTRACT_FRAMES = 0xa8c` (2700), `ATTRACT_MOVIE_A = 0x073a_0048`, `ATTRACT_MOVIE_B =
  0x073b_0048`, `PAGE_ATTRACT = 0x4000_0035`.
- `FrontEnd::attract_playing`, `attract_movie` (the code this run plays), `attract_toggle` (the
  game's `movie.291` bit), `attract_opener` (the page 0x5f returns to).
- `FrontEnd::tick` opens the attract when the count hits `ATTRACT_FRAMES` on the title or main
  page (the game's 0x44/0x40000035). The fire is also guarded on `!intro_playing`: the port
  flips the page to Main before the intro movies play, but the game's up-page during the intro
  is `P_INTRO`, which never checks the count [M 0x209e88 has no `Menu_GetNoInputCount` call] —
  the 45 s clock may only start at the real page's 0x4c. The same 0x4c reset is applied where
  the port re-opens a page without its own handler run: the intro's end (P_MAIN's 0x4c stand-in
  [M 0x20a7b0:37]) and the credits' two exits (ToMain rides msg 0x44 → the new page's 0x4c;
  PopBack rides 0x5f → 0x44 lparam 5 → the opener's 0x4c [M 0x21c858 case 0x5f:592-597]).
- `FrontEnd::action` gates input while `attract_playing`: Confirm = the skip (the game's
  StopMovie + 0x5f, collapsed into one call), everything else is swallowed (the game's block
  byte + cleared actions).
- `FrontEnd::attract_movie_finished` is the host's hook: the player finished (or the skip) →
  the opener comes back with the count reset.
- The host (`lib.rs::movie_tick`) starts `movie::Player::start` on `attract_movie` while
  `attract_playing` (the game's `psiStartBackgroundMovie` on the one BGFMV slot), polls
  `finished()` (the game's `psiMovieFinished` poll), and calls `attract_movie_finished`. A
  declined start (a missing/corrupt PSS with the movies feature on) closes the attract at once
  — the game's un-started movie leaves `BGFMVPlaying == 0`, so `psiMovieFinished` is true from
  tick one [M 0x1082e8]. The music pair's stop/restart [M `SFXRemove`/`SFXStart` 0x1d7/0x470]
  is data for the menu-sfx work — the port's front end has no menu-music voice yet.

## Tests

`crates/nf-view/tests/menus_flow.rs` (numbers only — no disc, no draw):

- the timeout fires at exactly 2700 ticks on the title (sister: 2699 does not);
- the code pick: from the title always A; from the main menu A, B, A across three timeouts
  (sisters: wrong codes at wrong counts);
- the skip: Confirm during attract returns to the opener with the count reset (sister: the
  non-skip actions do not end it and do not reset the count);
- the movie-end path: `attract_movie_finished` does the same (sister: it is a no-op when
  attract is not up);
- the intro guard: no fire while the intro movies are up, even past 2700 idle ticks, and the
  clock restarts at the main menu's open (sisters: one tick short / the fire one tick later);
- the existing `the_attract_timer_counts_2700_inputless_frames` keeps holding (the count still
  reads 2700 at the fire tick, and the skip's Confirm zeroes it).
