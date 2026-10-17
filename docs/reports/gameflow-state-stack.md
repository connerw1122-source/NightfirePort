# gameflow-state-stack — the real GameFlow stack vs the port's stand-in

Branch `s/gameflow-state-stack` (from origin/master, master unchanged since the cut — no merge
needed). Task 719.

## What the game does (all [M] ACTION.ELF unless noted)

- **GameFlow_GetState 0x1c9e80 / PopState 0x1c9eb0 / PushState 0x1c9f38** over `GameStateStack`
  0x2c7398 + `StackIndex` 0x30cece (64 entries, wrap to 0 past 0x3f). PushState's real ABI is
  `PushState(float duration, uint state, uint colour)` — the decomp's arg mapping is scrambled;
  the disassembly at the Mission_Update site (0x183054-60) shows f12 = 60.0f, a0 = 7,
  a1 = the fade colour.
- **PushState converts**: 7 → 8 and 0xd → 0xe via `Camera_SetFade(6, duration, 1)` (0x1bbdd8) —
  the push is **aborted** when SetFade returns 0 (its +0x234/+0x235 busy pair is up;
  `beqz $v0` at 0x1c9f88/0x1c9fa8); 0xb → 0xc with the duration stored straight into
  fade+0x22c (0x1c9fbc `swc1 $f12, 0x22c`), no fade and no abort. After every push/pop the
  draw-skip byte GameState+0x40 is recomputed from the new top: {1,3,4,5,9} → skip, else draw.
- **GameFlow_Main 0x1c98c8** ticks only the top state: 1 boot (pushes 2; Reset_MapLoadSettings
  0x1d6958 then pushes 3 in the same tick), 2 waits on +0x24 then pushes 3, 3 = ResetMap_Load
  0x1d6360 (the 0/1/2/3-phase loader; phase 0 pushes the briefing movie 6 over itself; the tail
  pops 3 and pushes 4), 4 = Locks_Init + pop, 6 = movie (pops on psiMovieFinished or skip
  action 0x19), 8 waits for the fade to finish, 9 = psiLaunchDriving + pop, 10 = Input_Update +
  Game_Draw only, 0xc counts the held fade down, 0xe sets **SkipCodeFrame** while the fade-in
  runs (the Main tail skips Game_Run — the sim freezes under a fade-in) and pops when it ends.
  States 5 and 10 are pushed by **nothing** on NTSC-U — all 16 PushState call sites enumerated.
- **The push callers and their durations** (each verified in disassembly):
  Mission_Update 0x183054 = 60.0f/7; MP_Update 0x184ea8 = 60.0f/7; C_SBNFMAP 0x20ba50 =
  84.0f/7 (mission pick); P_NFMAP 0x20b864 = 84.0f/**0xb**; C_GCPAUSE 0x205cec = 84.0f/7 — one
  call in a tail shared by the RESTART and QUIT arms; P_MPCONFIRM, P_ENDMISSION, C_LBPMMAP,
  C_MPDBG = 84.0f/7; Menu_RunMiniMission = 84.0f/0xb; Trigger_Update 0x1b1a78 = movie 6 +
  30.0f/7; Trigger_Activate = movie 6 + 300.0f/7 and a 300.0f/7 warm restart; SP_Update
  0x1dbfd8 = 20.0f/**0xd**; ResetMap_LevelToLoad/ResetMap_Load push 3/9/6/4 with duration 0.
- The fade camera's lifecycle: UpdateGlbVars 0x1bada0 runs +0x235 through 1 running → 2
  finished → 0 idle; states 8/0xe pop when it leaves 1.

## What the port did and what changed

The stand-in already had the ids (1,2,3,4,6,8,9,0xc,0xe), the {2,7,8,0xd,0xe} load guard, the
0x71/0x73 wrap switch, and the phase-0 movie/phase-2 resolve. It differed in:

1. **The push duration was not modelled** — every fade/wait was a hardcoded 8-frame countdown
   and `push_state` took no duration. Now `push_state(state, frames)` mirrors the game:
   callers push the **raw** id (7/0xb/0xd) with the game's frame count, and 7→8 / 0xb→0xc /
   0xd→0xe convert inside. All 7 port sites updated to the game's values: mission end 60
   [M 0x183054], mission pick 84 [M 0x20ba50], MP confirm paths 84 [M 0x20ec00], MP debug
   restart 84 [M 0x217860], the MP-match-end demo runner 84, the mp_match_end_live pin 60
   [M 0x184bd0].
2. **Two pushes were missing entirely**: C_SBNFMAP's 84-frame fade on the mission pick and
   C_GCPAUSE's shared RESTART/QUIT fade (the game pushes it on both arms [M 0x204408:150-152]).
   Both added; the driving-row pick correctly pushes nothing (C_SBNFMAP's driving arm has no
   push).
3. **The fade-busy abort**: a 7/0xd push while a fade is in flight now pushes nothing
   (`fade_active`, set on convert, freed when the hold pops) [M 0x1bbdd8 + 0x1c9f88].
4. **SkipCodeFrame**: `GameFlow::skip_code` is set while state 0xe's fade-in runs and cleared
   per tick, matching the Main tail's Game_Run skip [M case 0xe + LAB_001c9e28].
5. Docs: the previously wrong comment "0x42a00000 = 60.0f" corrected (it is 84.0f); the
   state-5 dead arm of the draw-skip set noted next to the existing state-10 note.

Not changed on purpose: the title/intro movies stay the port's state-6 countdown model (the
game's P_INTRO 0x209e88 polls its own FMVs without touching the stack — the port's page flow
depends on the model; behaviour equal, noted as [S port choice]); the 64-entry/0x3f stack wrap
(the port's Vec never overflows in practice); the one-tick phase-1→2 difference in load_tick;
the host's async load overlaps the fade instead of waiting under it (headless-invisible).

## Tests

New `crates/nf-view/tests/gameflow_stack_sp_run.rs` (4 tests):
- `the_boot_stack_sequence` — [1] → [1,2,3] in one tick, the load's pop-3/push-4 tail → [1,2].
- `the_sp_run_stack_matches_the_game` — the full SP run: menu walk rides state 2; the mission
  pick → [1,2,3,8] with fade_frames 84; the fade pops onto the loader; phase 0 pushes the
  briefing 6 (holds while BGFMVPlaying, pops at the end); phase 2 resolves; phase 3 → [1,2];
  the mission end → 60-frame fade over the next load; the results page; the next on-foot row
  repeats the sequence into 0x07000001. Every step's stack contents asserted.
- `push_state_contracts` — the 0xb wait (84 frames, no fade), the abort while a fade is live,
  the slot freed after the pop.
- `the_fade_in_freezes_the_sim_like_skipcodeframe` — skip_code rides the 20-frame fade-in and
  clears at the pop; the draw-skip set {1,3,4,5,9} pinned.

Guard-rail updates (assertions only, now citing the push sites): menus_flow (the mission pick
drains the 84-frame fade before the load ticks), mission_unlock_flow (same), movie_playback_flow
(same).

## What I ran

- `cargo build --offline --release -p nf-view` — clean.
- Full `--release` test runs for nf-view's flow surface: menus_flow 42, mission_unlock_flow 4,
  movie_playback_flow 9, menus_e2e 9, menu_sp_e2e 1, mp_menu_flow 2, mp_match_end_live 3,
  menu_movies 5, fail_replay_level 3, menu_graph_walk 7, mission_stats_flow 2, menu_audio_flows
  9, menu_sfx_flow 5, memcard_save_flow 2, menus_mouse_parity 2, dev_menu_flow 9, credits_data
  14, front_end_chain_live 17, `nf-view --lib` 16 — **all passed, 0 failed**.
- nf-game/nf-formats/nf-audio are untouched by this branch (nf-game compiles as a dependency of
  every run above).
- The background fast runner (tools/test/suite.py) was started twice (19:40 and 21:05 UTC);
  the sandbox killed the first between tool calls (suite.out 0 bytes). The restarted run's
  total line: see below.

**Suite total (final tree): not completed in this session.** The fast runner was started three
times (19:40 plain nohup, 20:06 setsid without the cargo PATH — died on
`FileNotFoundError: 'cargo'`, 21:05 setsid with `source nf-env.sh`); the sandbox kills
background jobs between tool calls in this environment, and every attempt left a 0-byte
suite.out. The final-tree evidence is the targeted per-file run above (the only crate this
branch changes is nf-view; nf-game compiles clean as its dependency in every run). The next
session should start `NF_ISO=... nohup python3 tools/test/suite.py > suite.out 2>&1 &` on this
branch and compare against master's baseline (3 allowed disc_golden failures).

## What's left

- The suite total above if its run outlives the session; the targeted runs are the evidence.
- Live checks (optional, for the main agent):
  - PCSX2 0x30cece (`StackIndex`) watch during a mission pick and a mission end: expect
    [1,2,3,8] and [1,2,3,8] with the 84/60 holds.
  - 0x2a37a8 (GameState+0x40) read at the draw decision right after a fade push: expect 1 for
    one frame, then 0 while state 8 ticks (the one-frame skip the port does not model).
- The trigger-movie pushes (Trigger_Update/Trigger_Activate 6 + 30/300 fades) are not wired to
  the port's trigger system yet — the flow machine now supports them (`push_state(6, 0)` +
  `push_state(7, 300)`), the caller glue is a separate task.

## Usage

not visible to me (no token/context-window figures are shown in this environment).
