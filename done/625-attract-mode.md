## Your task: `attract-mode`, what the title does when left alone

`P_ATTRACT_Handler` 0x20a1f0 and the title's no-input counter (`Menu_ResetNoInputCount`, `FrontEnd::no_input_frames`
already counts): after the timeout the game plays an attract sequence (a demo movie or a recorded play?). Port the
timeout value, what plays (movie codes / demo data), how input interrupts it, and the return to the title. Flow +
state on `FrontEnd`, the host starts the movie through the existing movie player. Tests: the timeout fires at the
game's frame count; any input returns to the title.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/attract-mode`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox4 at 2026-10-15T22:19Z

---
## Report (done, 2026-10-16T00:02Z)

Closed by the driver: the report is on `s/attract-mode` (docs/reports/attract-mode.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T00:23Z)


---
## Report from the worker (done, 2026-10-16T00:24Z)

# attract-mode — report (task 625, branch s/attract-mode)

## What's done

The title screen's no-input attract sequence is ported end to end, docs first, no drawing:

- **The timeout** [M `P_START_Handler` 0x20a408 / `P_MAIN_Handler` 0x20a7b0, msg-0x50 branch]:
  `Menu_GetNoInputCount() == 0xa8c` (2700 frames = 45 s at the fixed 60 Hz) sends manager
  message 0x44 with wparam 0x40000035 — open the attract page. Exactly two pages check (the
  only `Menu_GetNoInputCount` callers in ACTION.ELF — machine-verified via the jal sites
  0x20a5e8/0x20a910). The count is `frame_counter` 0x30d164 − `last_action` 0x30d168; the
  counter increments per menu tick [M `Menu_AlphaUpdate` 0x1f3170] and `Menu_InputAction`
  0x1f1f40 writes `last_action` at 12 sites (machine-verified `sw gp,-29960(gp)` stores).
- **What plays** [M `P_ATTRACT_Handler` 0x20a1f0's 0x4c]: a non-looping background FMV —
  `psiStartBackgroundMovie(code, 0, SFXGetVolume())` on the one BGFMV slot. The code is
  `0x073a0048`, or `0x073b0048` when the opener is the main menu (its id 0x40000002 arrives as
  the 0x4c's sixth argument [M `Manager_SendMessage` 0x21c858:345 passes the previous page's
  id]) and the toggled `movie.291` 0x30d368 is 0 — so the title always plays 0x073a0048 and the
  main menu alternates A, B, A. Both codes are `UsingMovie`-whitelisted [M 0x107eb8] →
  `30_FPS/073A0048.PSS` / `073B0048.PSS`. The open also removes the menu-music pair
  `SFXRemove(0,0x1d7)` / `SFXRemove(0,0x470)` and blocks page input (`Manager_SendMessage(…,
  0x68, 1, 0)` sets the manager block byte +0x1d4 and clears all queued actions [M 0x21c858
  case 0x68]).
- **How input interrupts it** [M `MenuManager_Update` 0x21c3d8:37-43]: while a movie plays the
  manager polls `Menu_InputAction(-1, 0x19, 4)` — the newly-pressed skip action — and runs
  `Menu_StopMovie(0)` 0x215a50 (guarded by the `menu_movie` 0x2e1c48 block flags: +1 set, hold
  counter +4 == 0). That is the ONLY input that reaches anything while the attract plays; the
  0x22 cursor/confirm/back dispatcher is gated off by the block byte [M 0x21c3d8:70].
  `bSkipAttract` 0x30d2a6 is read in the handler but **never written anywhere in ACTION.ELF**
  (whole-program operand scan, machine-verified: one load, zero stores) — a dead developer
  flag; not modelled.
- **The return to the title** [M `P_ATTRACT_Handler` 0x20a1f0:39-65]: on `psiMovieFinished`
  0x1082e8 the menu music restarts (`SFXStart(0x1d7,0)`/`SFXStart(0x470,0)` once), the input
  block lifts (0x68/0), then `psiStopBackgroundMovie` and `Manager_SendMessage(own page, 0x5f)`
  — the manager pops the stacked opener [M 0x21c858 case 0x5f re-opens it with lparam 5] whose
  0x4c calls `Menu_ResetNoInputCount`, restarting the 45 s clock.

**In the port** (state, flow and getters on `FrontEnd` in `crates/nf-view/src/menu.rs`; the
host starts the movie through the existing `movie::Player`):

- Constants `ATTRACT_FRAMES = 0xa8c`, `ATTRACT_MOVIE_A/B`, `PAGE_ATTRACT = 0x4000_0035`.
- `FrontEnd` fields (own lines at the struct tail): `attract_playing`, `attract_movie`,
  `attract_toggle` (the game's `movie.291`), `attract_opener`.
- `FrontEnd::tick` fires the timeout on the title/main pages at exactly 0xa8c (guarded on
  `!attract_playing` and `!intro_playing` — the game's up-page during the intro movies is
  P_INTRO, which never checks). Count resets land at every re-open stand-in: the intro's end
  (P_MAIN's 0x4c [M 0x20a7b0:37]), the credits' two exits (ToMain = msg 0x44 → the page's 0x4c;
  PopBack = 0x5f → 0x44 lparam 5 → the opener's 0x4c), and the opener's return.
- `FrontEnd::action` gates input while the attract is up: Confirm (the port's action-0x19
  stand-in, the intro skip's mapping) collapses the game's StopMovie → 0x50-finish → 0x5f pair
  into `attract_movie_finished()`; every other action is swallowed and does not reset the count
  (the game's `Menu_InputAction` never runs for them).
- `FrontEnd::attract_movie_finished` (pub): the host's `psiMovieFinished` hook — back to the
  opener, count reset, no-op when the attract is down.
- Host glue in `lib.rs::movie_tick`: `Player::start(attract_movie)` replaces the looping page
  backdrop on the one slot (the game's `psiStartBackgroundMovie` semantics), the `finished()`
  poll ends the run, and a declined start closes the attract at once (the game's un-started
  movie leaves `BGFMVPlaying == 0` → finished from tick one). The redraw treats
  `attract_up` like the intro: the movie plane fullscreen, the overlay suppressed (the attract
  page has no controls [L the menu-def's 0x40000035 record: aux 0, no control rows]).

**No drawing** was done: the draw side only reads `attract_playing` + `attract_movie` (plus the
existing movie plane), per the task.

## Verification

- **Independent audit** (subagent): re-read all ten decompiled handlers and re-verified the
  doc's claims at machine level (raw gp-relative operand scan over the whole text segment +
  manual decode; symtab cross-check). Verdict: all six core claims exact; it found three real
  edge defects, **all fixed and re-tested**:
  1. no `!intro_playing` fire guard — a long intro (real FMVs) could fire the attract
     mid-movies;
  2. a declined `Player::start` (missing PSS with the movies feature on) left the attract up
     forever with input blocked — now closes at once;
  3. the credits' two exits did not reset the count — the `==` fire would be dead on the
     returned page.
- **Tests** (`crates/nf-view/tests/menus_flow.rs`, numbers/ids only, no disc, each with a
  sister assertion that can fail — 5 new, 42/42 green on the final tree):
  the timeout fires at exactly 0xa8c (sister: 0xa8c−1 does not) and never re-fires mid-attract;
  the title always picks A and the main menu alternates A, B, A (with the intro-end reset
  pinned at count 1); Confirm ends the attract back at the opener with the count reset while
  Up/Back are swallowed without touching the count; `attract_movie_finished` mirrors the movie
  end (sister: a true no-op when down); the intro guard holds past 2700 idle ticks and the
  clock restarts at the main menu's open. The pre-existing
  `the_attract_timer_counts_2700_inputless_frames` still holds (its Confirm now exercises the
  skip path).
- **What I ran** (final tree, `cargo test --offline --release`): `nf-view` — the affected
  binaries `menus_flow` (42), `movie_playback_flow`, `menu_movies`, `menu_live`,
  `menu_backdrop_still`, `mp_menu_flow`, `dev_menu_flow` and the lib tests: **93 passed,
  0 failed** re-run after the audit fixes landed. `nf-game` + `nf-formats` + `nf-audio`:
  105 passed, 3 failed — the 3 `disc_golden` tests
  (`bigf_header_fields_behave_as_documented`, `bigf_tables_match_the_golden_manifest`,
  `directory_tree_matches_the_golden_manifest`), which the task allows to fail on the slim ISO.
- **Full suite**: `tools/test/suite.py` was started in the background at 23:31 UTC with
  `NF_ISO` + `NF_SILENT` set (an earlier 22:57 attempt died at the `alsa-sys` build for lack of
  `PKG_CONFIG_PATH` — this sandbox needs `PKG_CONFIG_PATH=/home/z/alsausr/usr/lib/x86_64-
  linux-gnu/pkgconfig`). Background jobs are killed between tool calls in this sandbox, so the
  350-binary pass cannot complete here (the same limitation the mp-match-rules task recorded);
  the foreground crate runs above are the verification of record (93/93 on the final tree for
  every binary the task touches). The suite's per-binary log is
  `/home/z/my-project/suite.log`: 177 binary sections in when this report was written, with
  the only failures the 2 no-GPU `fix_0c_pylon_live` render tests (wgpu `request_adapter`
  panic — the same no-GPU environment failure the mp-match-rules task recorded, unrelated to
  this task).

## What's left

- The attract movies' real lengths are unknown here: the slim on-foot ISO carries
  `073A0048.PSS`/`073B0048.PSS` as zero-byte stubs [L], and this sandbox has no full retail
  image. The live loop with the `movies` feature + a full image plays the real FMVs; headless,
  the attract closes at once (the game's un-started-movie behaviour).
- The menu-music pair events (SFXRemove/SFXStart 0x1d7/0x470 around the attract) are documented
  data for the menu-sfx work — the port's front end has no menu-music voice yet.
- The `fe.tick()`-per-redraw caveat (pre-existing, shared with every menu timer): the game
  counts 60 Hz vblanks, the port's menu ticks ride the redraw rate.

## Live checks for the main agent

1. **Attract movie lengths**: on a full retail image, measure `/MOVIES/30_FPS/073A0048.PSS`
   and `073B0048.PSS` picture counts (`nf-view --movie 073a0048 --frame 12 --shot out.png`
   prints them, or the `pss_movies_live` harness). Expected: two real FMVs (the whitelisted
   codes); their counts × 2 ticks = the real attract duration. The slim ISO's stubs measure 0.
2. **Render check** (no GPU here): with the front end up, idle 45 s (or poke
   `fe.no_input_frames`) — the attract should show the FMV fullscreen over black, no menu
   controls, no dark plate; on ✕/START it returns to the title/main menu with the page's own
   backdrop loop resuming. Suggested spot: the front-end boot (`--page start` equivalent) with
   the count forced to 0xa8c.
3. **PCSX2 cross-check** (optional): idle on the title 45 s → the attract FMV starts; the
   movie is always 073A0048 from the title; idle on the main menu twice → 073A0048 then
   073B0048 (the movie.291 alternation).

## ntfy

(none yet — see the final line if delivery failed)
