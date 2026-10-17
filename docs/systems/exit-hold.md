# The level-exit hold — `GS_PauseGame(1)` + the 300-frame fade before the load

*mayhew-end-transition (task 750o), 2026-10-17. The end of 07000005 (Mayhew): the end trigger
fires, and the game does NOT load the next level immediately — it freezes the world and fades
to black over 300 frames, and only then runs the loader. The port used to drain `level_exit`
into the loading screen the same frame.*

## The game's flow, with the evidence

When a `LOAD_LEVEL` (0xE8) trigger activates — the truck chain's auto-fire through
`Trigger_Update`'s case-10 arm, or the player's use key through `Player_Activate` —
`Trigger_Activate` 0x1b2318 runs its type-10 tail `LAB_001b24b8`:

1. the success/fallback channel writes (ext+0x28/0x2a/0x2c) [M 0x1b2318:64-88];
2. `Music_Event(3, 1)` — the end sting [M 0x1b2318:89];
3. **`GS_PauseGame(1)`** [M 0x1b2318:90] — `GameState[0x50] = 1` plus the `MPGame` slots
   [M `GS_PauseGame` 0x17d178:8-19];
4. `ResetMap_LevelToLoad(next, 1, 0)` [M 0x1b2318:91] — sets `IsWarmReset`, wraps the story
   code for the intro machinery (0x07000006 → 0x71000006; 06 has no briefing movie
   [M 0x1d3800:28-101 + `ResetMap_Load` phase 0/2 0x1d6360]), pushes **state 3**
   (`ResetMap_Load`) onto the GameFlow stack [M 0x1d3800 tail];
5. **`GameFlow_PushState(300.0f, 7, 0xff)`** [M 0x1b2318:92] — the push converts state 7 → 8
   and runs `Camera_SetFade(6, 300.0f, 1)` first [M 0x1c9f38:18-24; the ABI
   `PushState(float duration, uint state, uint colour)` is disasm-pinned in
   docs/reports/gameflow-state-stack.md];
6. the trigger is spent (type := 99) and `Player_RamSave()` runs [M 0x1b2318:93-96].

With state 8 on top of the stack, `GameFlow_Main` waits while the fade camera's busy byte
(+0x235) is 1, then pops → state 3 (`ResetMap_Load`) runs and the next level loads
[M 0x1c98c8:173-189]. The fade itself advances every frame on `Camera_UpdateAll`, outside the
pause gates [M `Game_Run` 0x1c9790 tail], at the rate `Camera_SetFade` stored: alpha runs
linearly 0 → 1 over the 300 frames (rate = 60/duration × 1/60, × the 0xff colour byte / 255
[M `Camera_SetFade` 0x1bbdd8:11-33]).

**What the pause freezes.** `GameFlow_Main` advances the game clock (GameState+0x34 and the
frame counters) only while `GS_IsPaused(-1) == 0` [M 0x1c98c8:44-47]; `psiInput_MapInputs`
drops the player input while `GameState[0x50]` is set [M 0x10cd80:36]; `Game_Run`'s
`SSys_Monitor`/`Light_Update`/`control_movement_object_handler` sections are behind the same
gate [M 0x1c9790:30-38]. But `Game_Run` itself still runs every frame — the switch in
`GameFlow_Main` falls through to `LAB_001c9e28: if (SkipCodeFrame == 0) Game_Run();` — so the
world keeps being DRAWN under the fade (`Game_Draw` runs while GameState[0x40] == 0, and
state 8 is not in the skip-draw set {1,3,4,5,9} [M 0x1c98c8 LAB_001c9e28 tail + PushState's
draw-skip switch]).

**The script's 300-tick wait is BEFORE the hold, not part of it.** The park (ch 0x70) raises
mux 0x65; SP #320 (`0x06000A23`, mode 0x40 = self-trigger on its out channel) waits 300
script ticks, then event 0x11 writes ch 0x1D [D the script's stream 1: `op 05 wait 300; op 12
event 0x11 [0x98, 0x98, 0x1D, 1]`; M `Script_EventHandler` 0x1d94c0 case 0x11]. The world
stays live through those 300 ticks (guards idle-animate in the real capture
[D tests/traces/05-truck.trace frames 36874..37173]). `Trigger_Update`'s case-10 arm then
fires #118 the same tick ch 0x1D turns on — no proximity check for kind 10
[M 0x1b1a78:162-166] — and the hold begins.

**The real-capture signature** [L tests/traces/05-truck.trace]: the last live 07000005 tick
(line 37173) has drones mid-animation (guard 7 at 55.964→55.944 across 37172→37173), the ch
0x1D marker sits on it (`ch 37173 29 0 1`), and the next captured line is already 07000006 —
the capture hole between them is the paused hold (the trace captures the unpaused sim ticks;
a paused world produces none), then the load's four pre-place frames (bond 0.0, lines
37174-37177) and the first in-game tick at the truck-ride start #496.

**The script-loadmap path does NOT hold.** Script event 0x12 (`Script_EventHandler`
0x1d94c0:194-199) runs `Player_RamSave()` + `ResetMap_LevelToLoad(p0, 1, 0)` — no
`GS_PauseGame`, no `PushState(300.0f, 7)`. The port's `Request::LoadLevel`
(crates/nf-game/src/nis.rs event-0x12 arm) must stay hold-free; only the two
`Trigger_Activate` arms get the hold.

## The port

- **`nf_game::drone::ExitHold`** — `{ next, fade_frames, total }`. `Drones::exit_hold`
  (own field line) is set by BOTH `Trigger_Activate` post sites — the auto-fire arm
  (`Trigger_Update` case 10 → gone) and the use-key arm (`player_use_triggers` →
  `activate_request`) — alongside `level_exit` + `Music_Event(3,1)` [M 0x1b2318:89-92].
  `Drones::exit_fade()` returns `Option<(frames_left, total)>` for the draw code: the black
  overlay's alpha is `1 − left/total` (linear [M 0x1bbdd8]).
- **The freeze** — `Sim::step` and `Sim::step_bond` early-return while the hold stands, after
  ticking `fade_frames` down [M 0x1c98c8:44-47 (the clock stops), 0x1c9790:30-38 (the world
  sections gate), 0x10cd80:36 (the input dies)]. Deviations, both headless-visible: the port
  counts the fade on the fixed 60 Hz tick instead of the render-side `Camera_UpdateAll`
  (same 300-frame length), and the fire tick itself completes before the freeze begins (the
  game's `Trigger_Activate` sits mid-walk too; the gated sections after it in the same frame
  are the difference — at most one tick).
- **The drain defers** — the viewer's `level_exit` drain puts the request back while the hold
  stands, and resolves + starts the async load on the first frame after it clears
  [M 0x1c98c8 case 8 → pop → state 3]. `Player_RamSave` content is identical whenever it is
  taken: the world is frozen between the fire (the game's save point [M 0x1b2318:96]) and the
  drain.
- **Not held**: the script event-0x12 load path (`Request::LoadLevel`) [M 0x1d94c0:194-199];
  the mission-end path (its own 60-frame fade already exists on the front-end flow
  [M 0x182c60:73-74]).

## Tests

`crates/nf-view/tests/mayhew_end_transition_live.rs` (ISO-gated, real path, no teleports):

1. `the_end_hold_freezes_the_world_for_300_frames_then_the_next_level_loads` — the walked
   hand-over (the 750c route), the ride, the park; at the fire: `exit_hold` = 300,
   `level_exit` = 0x07000006 already posted; over 100 hold ticks NOTHING in the world moves
   (Bond, drones, channels) while `fade_frames` counts down; at exactly 300 the hold clears
   with `level_exit` intact; `RamSave::capture` + `load_with_carry("07000006.bin")` spawns
   Bond at the gate-0x1D start #496 (57.18, 39.73, 22.61 [D placement; M `Player_Start`
   0x190fd0's first gate-on start]).
2. Sister (the freeze can fail): the pre-fire world at the park is live — positions advance
   while SP #320 waits — so the frozen-window assertion has a live contrast in the same test.
3. Sister (the 0x12 distinction): 07000006's dressing exit (event 0x12 → 07000007) posts
   `level_exit` with NO hold [M 0x1d94c0:194-199].
