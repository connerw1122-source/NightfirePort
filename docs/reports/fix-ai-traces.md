# fix-ai-traces — report

Branch `s/fix-ai-traces`. The task: replay Bond's recorded path from `tests/traces/{05-truck,01-doors,01-raptor,02-civilian}.trace`
through the port's sim and compare every drone's state/position frame by frame; fix the divergences from the decompile.

## What's done

1. **The replay harness** — `crates/nf-view/tests/ai_traces_live.rs` (new file, no shared code touched):
   - parses a trace (data rows, per-level segments — the recorder keeps recording across a level load);
   - loads the level with `nf_view::load`, places Bond per frame (pos/yaw from the trace, velocity from the
     frame delta, `grounded` on big y drops, crouch from the ✕ rising edge), steps the real `Sim::step_bond`
     with the pad bits mapped through the game's own style-0 mapping (docs/systems/input.md §3: R1 0x0008 =
     fire/trigger, L1 0x0004 = aim mode, △ 0x0010 = jump, □ 0x0080 = reload, ○ 0x0020 = use), and applies the
     recorder's cheats (`--god`) the way fix-pickups-trace did (`Dev::after_tick`);
   - compares every drone's state id (`Drone+0x10C`) and position (`obj+0x30`) per frame in DRONE_LIST order
     (`Drones::llist`), printing the **first divergence per drone**;
   - handles the three alignment facts that make the comparison exact (all [L], see the atlas section):
     a warm-up K (the recorder starts mid-run), the hip conversion for `obj+0x30.y` (`DroneAnim_PreTransCallback`
     0x13b490 lifts an animating drone's object y to feet + `Drone+0xA0`; a disabled drone's anim never ticks),
     and the user's kills injected where the game's list tail aligns shifted (Bond's pitch is not in the trace,
     so the port's Bond cannot hit what the user hit).
   - pins each drone's matched-frames count per trace (can only go up).

2. **The fix: PlayScript (state 3)** — the earliest divergence found (07000001's first guard, line 0 of both
   01 traces; also the immediate cause of the guard "already aiming" at tick 1). The game's drones with a start
   script (`Drone+0x554` = placement param 1, `NDrone2_DefaultInit` 0x14b300) enter **PlayScript**
   (`NDrone2_DSTATE_PlayScript` 0x15ba68 = `NDrone2_StateFuncs[3]`, table read from the ELF) from
   `NDrone2_DSTATE_Global` 0x15b830 instead of their start state; the port skipped it entirely (its own comment
   said "start scripts (PlayScript 3) are not ported") and the guard went straight into combat (AIM_STAND 0x67
   at tick 1). Ported: `Drone::start_script` (param 1, with the 0x6000000/0x6000892/0x600089c gates from
   0x14b300), Global's branch, and the state's ENTER (the script variant table: default/0x60005e6/0x60008ae →
   resume ATTACK via `play_script(0x24, 0x11a, …)`; 0x6000168/0x6000169/0x60007ac → the start state; the
   AttackTalk/property side effects are noted in the code comment) and UPDATE (the opponent look runs while the
   script plays; the fully-alerted `SeenAndAttacking` arm is deliberately left out — see "unresolved").

   Files touched: `crates/nf-game/src/drone.rs` (new struct field on its own line, new state const, the Global
   branch, the two PLAY_SCRIPT arms — additive only).

3. **The divergence tables** (written into `docs/atlas/drone-port-check.md`, "Trace replay" section):
   - **02-civilian: replays perfectly** — 24 of 25 drones match all 1083 frames; the ALLY_LEAD escort's pos
     drifts at 295 (the hip converter's y, state 28 matching throughout).
   - **05-truck**: drone 2 matches the WHOLE level (7851 frames); the patrol/idle guards match their pre-kill
     windows (the first user kill is at line 885); two open state divergences (below).
   - **01-doors / 01-raptor**: the first guard's STATE now matches from line 0 (the fix); the remaining first
     divergences are Mayhew's gating (drone 1) and the first fight's cover choice (drones 15/16).

## How it was verified

- The pinned tests `ai_traces_live::*` fail-before for the PlayScript fix (the pre-fix tree's first 07000001
  guard was AIM_STAND (0x67) at line 0 where the game has PlayScript (3) — the run is quoted in the atlas
  section), and green after. Per-drone matched counts are pinned and can only go up.
- Full suite (`NF_ISO=… cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view --
  --test-threads=1`): **nf-game 174 passed / 0 failed, nf-view 383 / 0, nf-formats + nf-audio green** — only the
  3 allowed `disc_golden` failures on the slim ISO (run with `--no-fail-fast` to see all packages).

## What's left (in the order the traces expose them)

1. **Mayhew's start gating** (07000001, drone 1, line 0): the port starts the ally in ALLY_LEAD_INIT (0x1c);
   the game holds him in WAIT_SWITCH (1) until his channel (0xb) fires. `NDrone2_DSTATE_Global`'s ally path /
   `DroneTypeSettings[12].start` needs the [L] channel check for allies.
2. **The first fight's cover choice** (07000001, drones 15/16, lines 63–76): the port entered RUN_FOR_COVER
   (0x8c) where the game was in COMBAT (0x58 / 0x103). Both traces agree, so it is a deterministic difference
   in `ChooseCombatMove`/cover availability, not noise.
3. **07000005's two late state divergences**: the gate guard entered SURRENDER_ANIM (0x3f) at frame 29828 where
   the game stayed IDLE (what disarms a surrender in the game's context?), and a guard entered 0x6F at frame
   36372 (near the truck park) where the game stayed IDLE.
4. **The PlayScript root-motion step**: the first guard's pos drifts 0.597 within the script (the state
   matches; the root track's per-tick application needs the SetScript variant args' exact mapping — the port's
   `play_script(hash, 0x24, 0x11a, …)` uses the decomp's numbers in the port's arg order, unverified).

## What needs a new recording (exact steps)

1. **A guard surrendering** (07000005, the gatehouse guard, ~8.4 s in the existing trace): the port plays
   SURRENDER_ANIM where the real game stayed IDLE — no existing trace covers a surrender the game actually
   accepts. Record: `python tools/trace/trace_record.py tests/traces/05-surrender.trace --god --seconds 30`,
   stand in front of the first gate guard (the one at (99.7, 16.9, 101.6)), aim at him WITHOUT firing for ~10 s,
   then fire two shots past him, and note the time you expect the hands-up.
2. **The ally Mayhew's channel release** (07000001): a trace that starts AFTER the intro (so Mayhew's channel
   0xb fires while recording): `--god --seconds 45`, walk straight to Mayhew and wait for him to start leading.
3. **Bond's pitch**: any 10 s where you aim UP at a guard on a ledge (the 07000005 sniper tower) — this is the
   missing input that stops the port's Bond from reproducing the user's kills in replays (the `user-kill`
   injections). A recorded pitch column would remove the injections entirely (the recorder would need
   `player anim+0x8A8` added — one more `p.read32` in `trace_record.py`).

## Live checks for the main agent

1. `animobj+0x5c` (the hip source of `DroneAnim_PreTransCallback` 0x13b490) vs the anim's root track y: watch
   (read) `animobj+0x5c` for the first gate guard (07000005, drone at (99.7, 16.9, 101.6)) for 60 frames while
   he idles, and compare with the root y channel the port parses for the same anim. Expected: they differ by
   0.023–0.033 (the port's converter gap). If they are EQUAL, the port's root parse is off instead.
2. `Drone+0x50C` (alertness) of 07000001's first guard at (-4.9, 6.3, −0.09) during the first 6 s of play
   (the PlayScript window): expected < 1.0 the whole time (the game's msg-3 alert arm never fires). If it is
   1.0, the port's creation alert for this guard is wrong and the `alert >= 1.0 → ATTACK` arm I left out of
   PLAY_SCRIPT's UPDATE must go back in with `NDrone2_SeenAndAttacking`'s full body.
3. Poke `switch_channels[0xb] = 1` on 07000001 while Mayhew is in WAIT_SWITCH (state 1): expected — he enters
   his lead init and starts walking (the port already does this; this confirms the channel is his only gate).
4. The surrender: with the gate guard at IDLE, get exactly into the state the trace shows at frame 29828 (Bond
   at ~(111.6, 20.8, 97.5) walking along the wall) and screenshot whether the real game's guard ever raises his
   hands. If he does not, the port's surrender trigger has a false positive in that context.
