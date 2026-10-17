**Reproduce first, on the newest origin/master, through the REAL path.** Earlier fixes for some of these passed
their own headless tests and the user still hit the bug in the game. Before changing code, write a live test that
drives the same path the player takes (the level loaded through `nf_view::load`, Bond moved by `step_bond` inputs —
walking, the use key, the ladder input — the real scripts and triggers running, no teleports past the problem
spot unless the report says so) and FAILS on current master the way the user describes. Put the failing output in
the report. Only then fix it, and the same test must pass. If you cannot reproduce it, say exactly what you tried
and where the path diverges from the report; do not mark it done on a guess.

## Your task: `mayhew-end-transition`, the end of the Mayhew level

The user, 2026-10-16 evening: "end of mayhew still takes some work to transition to the next level with mahews
pathing." (750c mayhew-exit-walk, already in progress, covers his walk before the MID cut-scene — read its branch
`origin/s/mayhew-exit-walk` and its report if it has landed, and coordinate: do not redo it.) This task is the
END: Mayhew's route to the level's exit at the end of the mission and the hand-off to the next level (the end
trigger, the outro/level-exit, ResetMap_LevelToLoad). Reproduce the whole end sequence through the real path on
master, find where his route stalls or the transition fails to fire, port the game's behaviour [M], and pin: the
level ends and the next level loads without the player doing anything the game doesn't require.

**No drawing**: the main agent draws every page (it compares each against PCSX2). You deliver the data, the state,
the flow and getters the draw code can call, in `crates/nf-view/src/menu.rs` (`FrontEnd`, `Page`, `action()`) or
`nf_game` as fits, with docs first (`docs/systems/<name>.md`, addresses + evidence tags) and tests (numbers/ids only,
ISO-gated where they read the disc, each with a sister assertion that can fail).

**Tests pin only proven behaviour.** Every assertion about what the game does cites its evidence: [M] a decompile
address that actually says it, [D] disc bytes, or [L] a live PCSX2 measurement from a FRESH boot (a save-state resume
skips the boot path). A guess, a port choice or an [S] item is never a hard assertion: mark it `// [S] unverified` and
keep it out of pass/fail. If an existing test contradicts the decompile, the TEST is wrong: fix it, with the address, and
say so in the report. Never weaken a decompile-backed test to make code pass.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/mayhew-end-transition`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox7 at 2026-10-17T03:54Z

---
## Report (done, 2026-10-17T04:59Z)


---
## Report from the worker (done, 2026-10-17T04:59Z)

# mayhew-end-transition — the end of Mayhew (07000005): the exit hold and the hand-off to 07000006

Branch `s/mayhew-end-transition` (from origin/master 31dd6161; merged origin/master 50d51ae7
into 52456c6a). User, 2026-10-16 evening: "end of mayhew still takes some work to transition to
the next level with mahews pathing." 750c mayhew-exit-walk (merged in) covers the walk before
the MID cut-scene; this task is the END: the end trigger, the outro/level-exit,
`ResetMap_LevelToLoad`.

## What the end actually is (the evidence)

"Mayhew" is the level's own name (Mayhew A = 07000005). There is NO Mayhew drone on it — all 16
drones are estate guards [D the level's drone census; docs/systems/actors.md §4] — and no
escort route (the kind-8 "Mayhew Route" belongs to 07000001 [D golden/ai_paths.tsv:1]). Nobody
walks with Bond at the end: Bond rides the truck bed **alone (crouched)** [D the truck script
0x06000A22's stream 7 parents him; D fix-truck's trace]. The only pathing in the end chain is
the gate guard reaching his post (ch 0x72): spawner #268 (start ch 0x22) places templates
#3/#4 at their post (28.34/32.06, 37.12, 44.86) and the guard there touches Touch #318 (out
0x72, mask 0x10 = drone) [D the disc placements].

The end chain [D docs/levels/07000005.md §3; D the disc placements/scripts; M the decompile]:

1. the park (ch 0x70, TouchOnce stream 6 at t 5384) + the bed touch (ch 0x71, stream 7,
   parented to the truck) + the guard at his post (ch 0x72) → mux #316 (out 0x65, AND);
2. SP #320 "EndLevel" (`0x06000A23`, mode 0x40 = self-trigger on its out channel) waits 300
   script ticks, then event 0x11 writes ch 0x1D [D the script's stream 1: `op 05 wait 300; op
   12 event 0x11 [0x98, 0x98, 0x1D, 1]`; M `Script_EventHandler` 0x1d94c0 case 0x11];
3. #118 "Door_Gate8" (0xE8, gate 0x1D, next 0x07000006, fallback 0x1F) auto-fires — no
   proximity check for kind 10 [M `Trigger_Update` 0x1b1a78:162-166];
4. **`Trigger_Activate` 0x1b2318's type-10 tail** (read directly, LAB_001b24b8): the
   success/fallback channel writes [M :64-88], `Music_Event(3,1)` [M :89],
   **`GS_PauseGame(1)`** [M :90 — `GameState[0x50]=1`, M 0x17d178:8-19],
   `ResetMap_LevelToLoad(next, 1, 0)` [M :91 — pushes loader state 3, M 0x1d3800],
   **`GameFlow_PushState(300.0f, 7, 0xff)`** [M :92 — converts to state 8 with
   `Camera_SetFade(6, 300, 1)`, M 0x1c9f38:18-24, the ABI disasm-pinned in
   docs/reports/gameflow-state-stack.md], spent (type 99) [M :93], `Player_RamSave` [M :95-96];
5. state 8 holds while the fade runs, then pops onto state 3 = `ResetMap_Load` [M
   0x1c98c8:173-189]. The world FREEZES under the fade — the game clock stops while paused
   [M 0x1c98c8:44-47], the player input dies [M `psiInput_MapInputs` 0x10cd80:36],
   `Game_Run`'s world sections are behind the pause gate [M 0x1c9790:30-38] — but the fade
   advances every frame [M 0x1c9790 tail] and the scene keeps drawing [M 0x1c98c8
   LAB_001c9e28: `Game_Run` runs every frame; state 8 is not in the skip-draw set {1,3,4,5,9}].

**The script's 300-tick wait is BEFORE the hold, not part of it** — the world stays live
through those 300 ticks (the real capture's guards idle-animate, frames 36874..37173 [L
tests/traces/05-truck.trace]). **The script event-0x12 loadmap does NOT hold** [M
0x1d94c0:194-199 — `Player_RamSave` + `ResetMap_LevelToLoad` only, no pause, no push].

The real capture agrees with the hold: the last live 07000005 tick (line 37173) has drones
mid-animation and the `ch 37173 29 0 1` marker (ch 0x1D going on); the next captured line is
already 07000006 — the capture hole between them is the paused hold (the trace captures
unpaused sim ticks), then the load's four pre-place frames (bond 0.0, lines 37174-37177) and
the first in-game tick at the truck-ride start #496 [L 05-truck.trace].

## What was missing in the port

`Trigger_Activate`'s port posted `level_exit` + `Music_Event(3,1)` and even *cited*
`GS_PauseGame(1)` in a comment — but never modelled it. The viewer drained `level_exit` into
`pending_level` the same frame and started the async load: no pause, no 300-frame fade, no
hold. That is the user-visible "transition still takes some work": the game holds the frozen
final scene under a 5-second fade-to-black before the loading screen; the port snapped.

### Reproduced on master, through the real path (the failing output)

A master-API probe (throwaway, uncommitted — the committed pin needs the new field) drove the
750c walk → the hand-over → the crouched ride → the park on UNPATCHED master code and asserted
the freeze by behaviour:

```
thread 'the_world_freezes_at_the_end_trigger_fire' panicked at crates/nf-view/tests/zz_master_repro_live.rs:47:5:
assertion `left == right` failed: the game clock stops while paused [M 0x1c98c8:44-47]
  left: 5787
 right: 5687
```

On master the sim clock ran **100 ticks past the end trigger's fire** (5687 → 5787) — the
world kept ticking through the level end; the game freezes it [M 0x1b2318:90-92].

## The fix (all [M], docs/systems/exit-hold.md)

- `nf_game::drone::ExitHold { next, fade_frames, total }` + `Drones::exit_hold` (own field
  line). Set by BOTH `Trigger_Activate` arms — the auto-fire site (`Trigger_Update` case 10 →
  gone) and the use-key site (`player_use_triggers` → `activate_request`) — alongside
  `level_exit` + the end sting. NOT by the script event-0x12 path (`Request::LoadLevel`)
  [M 0x1d94c0:194-199].
- `Sim::step`/`Sim::step_bond` tick the hold and skip the world while it stands [M 0x1c98c8:44-47,
  0x1c9790:30-38, 0x10cd80:36]. The expiry tick stays frozen too: state 8 pops straight onto
  state 3 and the load replaces the scene before the next tick [M 0x1c98c8:173-189].
  *Deviations* (headless-visible, documented): the fade counts on the fixed 60 Hz tick instead
  of the render-side `Camera_UpdateAll` (same 300-frame length); the fire tick itself
  completes before the freeze begins (the game's `Trigger_Activate` sits mid-walk too).
- The viewer defers the `level_exit` drain while the hold runs; the load starts the frame
  after it clears [M 0x1c98c8 case 8 → state 3]. `Player_RamSave` content is identical
  whenever it is taken — the world is frozen between the fire (the game's save point) and the
  drain.
- `Drones::exit_fade() -> Option<(frames_left, total)>` — the draw code's read: the black
  overlay's alpha is `1 − left/total` (linear, the rate `Camera_SetFade` stored [M
  0x1bbdd8:11-33]). **The overlay itself is NOT drawn yet — per the no-drawing split, the
  main agent draws it from this getter.**

## Tests (ISO-gated, real path, sisters included)

`crates/nf-view/tests/mayhew_end_transition_live.rs` 3/3:

1. `the_end_hold_freezes_the_world_for_300_frames_then_the_next_level_loads` — the walked
   hand-over (the 750c route, real inputs only + the one crouch press the game's ride stance
   needs [D fix-truck]), the ride, the park; the pre-fire contrast (mux 0x65 on, the world
   still live, no hold); at the fire the hold = 300 and `level_exit` = 0x07000006; over 100
   hold ticks Bond, the player anchor and the game clock stand still while `fade_frames` runs
   300 → 200; the hold clears at 300 with the request intact; the carried load spawns Bond at
   the gate-0x1D start #496 (57.18, 39.73, 22.61 [D; M `Player_Start` 0x190fd0]) and the save
   rides ch 0x1D. The end fires with no player action beyond the walk and the ride stance.
2. `the_use_key_exit_starts_the_same_hold` — E at the castle-gate door #118 (gate off → the
   fallback arm [M 0x1b2318:64-88]) starts the same hold; the clock freezes; 300 frames clear
   it. Fails if the use-key site loses the glue.
3. `the_script_loadmap_exit_does_not_hold` — 07000006's dressing exit (event 0x12 →
   07000007) posts with NO hold and the world keeps ticking [M 0x1d94c0:194-199]. Fails if
   anyone wires the hold into the script-load path.

## What ran (on the final merged tree)

- `nf-game --lib` 261/261; `nf-view --lib` 18/18; `nf-formats --lib` 79/79; `nf-audio --lib`
  11/11.
- The touched live binaries, `--test-threads=1`, release, NF_ISO:
  `mayhew_end_transition_live` 3/3 (new), `mayhew_handover_live` 2/2, `fix_truck_live` 2/2,
  `gate_guard_truck_live` 2/2, `level_carry_live` 8/8, `level_end_live` 6/6,
  `walk_07000005_live` 2/2, `walk_07000006_live` 2/2, `walkreal_07000005_live` 1 passed +
  1 `#[ignore]`d (the pre-existing cable sister, its own comment).
- The fast suite (`tools/test/suite.py`) **could not run in this sandbox**: background jobs
  are reaped between tool calls (started twice; the empty `suite.out` stands as the record) —
  the same failure pause-music-1b and 750c reported here. The foreground slices above cover
  the touched surface.

## What's left

1. **The fade overlay's draw** (the main agent's page): from `sim.drones.exit_fade()` —
   alpha `1 − left/total`, drawn over the live (frozen) scene; the loading screen follows.
2. The 0x71 briefing wrap on mid-game exits (the level-chain research's gap #2):
   `resolve_exit_code` passes plain 0x0700xxxx as-is; net-equivalent for 0x07000006 (no
   briefing movie — `ResetMap_Load` phase 0's hash table has no 0x7100006 [M 0x1d6360]) —
   left as-is, documented.
3. A fade-IN on the 06 arrival is NOT modelled: `ResetMap_Load` phase 3 ends
   `PopState` + `PushState(0, 4)` with no fade push on the plain-load path [M 0x1d6360:117-171].
   If the PCSX2 comparison shows one, it is a follow-up.
4. The driving-interlude exits (0x0730) stay logged-and-skipped (the known gap).

## Live checks for the main agent

1. 07000005, fresh boot, the truck route: at the park expect ~5 s of fade-to-black over the
   FROZEN final scene (the truck parked, the guards frozen mid-idle) BEFORE the loading
   screen; then 07000006 at the courtyard. Watch that nothing visibly ticks during the fade.
2. The fade draw against the getter: render points during the hold — bond at the park
   `--bond 29.9,37.9,51.7,0.0,0.0` at fire+0 (alpha 0), fire+150 (alpha ≈ 0.5), fire+299
   (alpha ≈ 1.0, black). The getter is `sim.drones.exit_fade()`.
3. `fieldprobe`: after the park, `GameState[0x50]` (0x2a3768 + 0x50 = **0x2a37b8**) should
   read 1 during the fade and 0 after the load; the fade camera's busy byte
   (`*(u32*)0x2d88c8 + 0x235`) should run 1 (running) → 2/0 (finished/idle) [M 0x1bada0's
   lifecycle]; and `switch_channels_time[0x1D]` should stop advancing while 0x2a37b8 == 1
   [M 0x1c98c8:44-47].

## Usage

- Token/credit/context figures: **not visible to me** — this session's interface exposes no
  token, cost or context-window numbers I can read. The sandbox clock read 03:57 UTC at
  session start (expiry ~05:57 UTC per the task message).
