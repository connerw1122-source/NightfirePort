The user (2026-10-17): "09 level elevator buttons dont work. make sure we're prioritizing level work so we can actually complete each level completely and get all models and functions per level working" and "I want EVERYTHING covered. ALL ON FOOT LEVELS". Level work now runs ahead of everything else in the queue.

**Reproduce first, through the REAL path.** Load the level with `nf_view::load` (or `load_weapons` when a
level's own weapon is needed), move Bond with `Sim::step_bond` inputs (walking, turning, the use key, the
watch gadgets, the weapons), let the level's own scripts and triggers run, and observe. No teleporting past
a problem unless the task says so; a teleport to reach a spot is fine when the test then operates the
thing through the real input. Cite the decompile [M], the disc [D] or a fresh-boot PCSX2 value [L] for every
game claim; tests pin only proven behaviour.

## Your task: `level-04-complete`, play 07000004 (Enemies Vanish (Castle D)) from start to finish

1. A live walk test (extend `crates/nf-view/tests/walkreal_07000004_live.rs`, un-ignore it) that plays the
   level from its start through the real input path and completes EVERY objective — the optional ones too —
   and exits the way the game does (the load into the next level, or the mission results). Every cut-scene
   on the route plays (skipped only with the game's own skip action), every scripted event fires.
2. Fix every blocker the walk hits: collision, doors, triggers, scripts, AI that stops progress, missing
   channel writers. Each fix gets its own focused test with a sister that fails on the old code.
3. In the report: the objective list with how each completes [M]/[D], the route, and anything the walk
   still cannot do with the exact reason.

Known open items on this level (start here): the key-gated route; the cable car / gondola pieces if present.

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

**Branch:** `s/level-04-complete`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox3 at 2026-10-17T11:47Z

---
## Report (done, 2026-10-17T13:27Z)


---
## Report from the worker (done, 2026-10-17T13:27Z)

# level-04-complete — 07000004 "Enemies Vanish" (Castle D) played start to finish

Branch `s/level-04-complete` (built on `origin/master`, with the `s/level-04-interactables` census
stacked on top — this task continues that level's work). Everything below was driven through the
real input path: `nf_view::load` + `Sim::step_bond` (walking, turning, the fire key), the level's
own scripts, triggers, pickups and mission layer running untouched. No teleports.

## What's done

**`crates/nf-view/tests/walkreal_07000004_live.rs` — 3 tests, all green (`--test-threads=1`):**

1. `walk_the_full_mission_start_to_finish` — the full play, 1951 ticks, no falls, no stuck events:
   spawn (1.87, 5.84, 41.07) [D start #0] → the intro SP #3 + story scene SP #2 settle through the
   game's own skip action (AI re-enabled, ch 0x53 written [D script 0x06000082 stream 9]) → obj0
   announced through the real monitor (enable arm [M 0x1827d8:55-67]) → the armament walk: the
   StormUzi #7 take (weapon 18 owned [M Pickup_Handler 0x18e650 type-0 arm]) and the
   BodyArmourHeavy #9 take (armour 0 → 50 [M :220-225]; the settled assassin's bullet-type pain
   chipped it to 48.5 by the assert — the raise is the [M] claim) → the assassin (drone #1, skin
   0x0500001e [D]) killed by the walk's own shots → ch 0x54 [M DroneFunc_SetDeathChannel 0x13e660]
   → obj0 folds COMPLETE through the real Mission_MonitorObjectives (record state 4 [M
   0x1827d8:78-85]) → the OUT trigger #46 "MultiplexOut" mirrors ch 0x54 into ch 100 (the
   mission-complete channel) [D list [0x64]; M Trigger_Update case 5] → Mission_Update case 6
   posts the "Mission Complete" banner with LevelToEndTo **0x0730004** [M 0x182c60:85-122] → case
   7's 240-tick hold → case 4's ResetMap_LevelToLoad → the viewer's `mission_end =
   Some(0x0730004)` (the port's end sequence: outro → results; the code is the next arc's DRIVING
   interlude). **Every objective completes; the level exits the way the game does.**
2. `walk_the_assassination` — the focused channel-level pin (kill → ch 0x54 → the mux copy), kept
   from walkreal-a.
3. `without_the_assassination_the_mission_stays_open` — the sister: the same route (both pickups)
   with the target ALIVE → ch 0x54 off, ch 100 off, MissionState 1, obj0 sits announced (state 3),
   Mission_Update internal never leaves 1. Fails on any code that completes the mission without
   the player's own kill.

`crates/nf-view/tests/interact_07000004_live.rs` — 10/10 green (the one-test-per-KIND suite from
the interactables task; its OUT test's body/comment corrected to the mux truth below).

## The objective list and how it completes

The mission table [L ee_start_05.bin @ 0x2a3f60 via mission::c07000004] has ONE objective — there
are no optional ones on this level:

| obj | text | watch | enable | how it completes |
|---|---|---|---|---|
| 0 | "Defeat the assassin" | 0x54 | 0x53 | announced when the story hand-off ch 0x53 fires [D 0x06000082 stream 9]; completes when the target's death channel 0x54 rises [D param 12; M 0x13e660] — through the real monitor [M 0x1827d8:78-85], pinned by the walk. |

The level's own completion writer is the OUT mux #46 (out 0x54 → list [0x64] = ch 100): the kill
feeds it, `Mission_Update` internal 1 sees ch 100 and runs case 6 → 7 → 4 [M 0x182c60].
Mission_MonitorObjectives' own all-objectives AND [M 0x1827d8:119-123] is the redundant second
writer: it needs the earlier castle parts' channels (0x47/0x4e/0x51/0x53), which a carried
campaign has ON and a cold load does not — the level's designed writer is the #46 mux, and the
walk completes through it with no hand-offs.

## Corrections to the docs (with the addresses)

- **The OUT kind is a pure channel mux, not a touch trigger.** `docs/levels/07000004.md` and the
  interact test said the #46 copies "while Bond stands inside". Wrong: `Trigger_Update` case 5
  writes `switch_channels[list[i]] = switch_channels[out]` for every non-zero list word EVERY
  tick, and `Trigger_Hit`'s result is not consulted [M 0x1b1a78 case 5:147-160]; the placement has
  no collision mesh (unbounded lo/hi [D]) and needs none. The old interact test "walked" to the
  box centre — a NaN target — and passed only because the copy is unconditional. Both corrected.
- **The exit route's step 3 is ported.** The census said the mission layer was un-ported; master
  merged 031-fix-mission since. `docs/levels/07000004.md` now records the ported chain and the
  walk asserts it end to end.
- **The "key-gated route" and "cable car / gondola" known items are settled** (census, unchanged):
  the key gate is 07000003's own exit AND-gate; the gondola pieces do not exist on this level.

## Blockers the walk hit, and their fixes

- **The interactables branch's `attach_level(level, hash)` signature left ~26 one-arg callers** —
  every nf-game test binary failed to build (`error[E0061]`), so the whole nf-game suite was dead
  on the merged tree. Fixed mechanically: each caller passes its fixture's own level hash
  (synthetic-Data tests their `Data.level`, disc fixtures their `level_id`/load code), and
  `nf-extract`'s tracer passes its parsed `level_id`. nf-game: **510 passed / 0 failed**.
- The walk itself hit no port blocker: the route's only failures were test-side (a too-strict
  armour assert against the live assassin's chip; the NaN walk target above). No gameplay fix was
  needed, so no focused fix tests beyond the two walk tests + the corrected OUT test.

## The route

spawn (1.87, 5.84, 41.07) → the cut-scenes settle at (−3.87, 5.44, 30.30) (Bond placed beside the
target) → the west lawn: the StormUzi #7 (−10.66, 4.39, 42.66) → the BodyArmourHeavy #9 (−1.85,
4.37, 46.07) → back to the target (−2.64, 5.50, 30.08): the shoot loop (turn/look/fire through
`step_bond`, reloads, repositions) kills him → the mux copy → the banner → mission end 0x0730004.
The assassin is a live enemy the whole walk (he closed 10+ units to engage during the pickup legs
and chipped Bond's armour with bullet pain — the game's own AI, top-up only keeps Bond's health
at 100).

## Suite (what ran, and the sandbox's limits)

This sandbox kills background jobs between tool calls (the background `suite.py` died three
times with an empty log), so the suite ran in foreground windows, crate by crate / set by set,
`--test-threads=1` throughout:

- **nf-formats + nf-audio (full crates):** green except the 3 `disc_golden` tests — the slim
  ISO's known failures (bigf golden manifests), exactly the allowed set.
- **nf-game (full crate):** 510 passed, 0 failed, 0 ignored.
- **nf-view, the level-flow family** (`suite.py --only` over walk/walkreal/interact/campaign/
  save/mission suites — 49 binaries): **211 passed, 0 failed, 8 ignored (6 min)**.
- **nf-view, the fix_*/menu_* bulk:** 206 binaries — **809 passed, 2 failed, 17 ignored**. The 2
  are `fix_0c_pylon_live`'s wgpu surface creation (`no_adapter_backends` — this sandbox has no
  GPU adapter; the file is untouched on this branch and the test is a render test the main
  agent's machine runs).
- **Not run** (the window budget ran out): ~141 nf-view test binaries, the alphabetical tail from
  `menus_mouse_parity` onward (menus/mp/save-view suites). No branch change touches their code
  paths (the branch's only src changes are the merged interactables census fixes: `drone.rs`
  attach_level's main-map rule + hash param, `world_fx.rs` leafgens/trees, one `scene.rs`
  call-site line).

## What the walk still cannot do (with the exact reason)

- Nothing on the route. Every objective completes and the level exits through the game's own
  mission-complete chain. The two rigid-body white lamps (0xD8) remain unported (a whole physics
  system, levels.md §7.1) — they are decorative, off the route, and gate nothing.
- The end sequence after `mission_end` (the outro movie → results pages) is the viewer's existing
  flow (fixed 2026-10-15), not re-pinned here; the walk's assertion ends at the latch the task
  names (the load into the next level / the mission results).

## Live checks for the main agent

- Fresh-boot PCSX2, 07000004: after the assassin dies, the "Mission Complete" banner should post
  WITHOUT Bond needing to be anywhere specific (the mux is position-independent) ~4 s after the
  death anim, then the level ends into the driving interlude. A gdbwatch on
  `switch_channels[0x64]` (ACTION.ELF BSS) during the death should show the write coming from the
  Trigger_Update case-5 block (0x1b1a78…0x1b1e6x), not from Trigger_Activate.
- The assassin's chip: the same watch on Bond's BLData+0x8B0 (armour) during the pickup legs
  should show 50.0 right after the #9 take and a small bullet-type decrement as he engages —
  confirming the armour raise + the live-AI chip the walk measured (48.5).

## Usage

not visible to me
