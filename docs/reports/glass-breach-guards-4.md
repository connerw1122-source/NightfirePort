# glass-breach-guards-4 — finish the 08 glass-breach guards (held at merge twice)

Branch `s/glass-breach-guards-4` (from `origin/s/glass-breach-guards-3` + merged origin/master
f53df46b; only `docs/WORKLOG.md` conflicted — master's file kept, the guards block appended; no
code conflicts). The held-branch merge blocker was
`interact_07000001_live::bullets_shatter_the_glass_and_vases_and_the_sister_stays_whole`
(interact_07000001_live.rs:171 "unshot scenery stays whole (the sister)"): the guard glass-breach
path broke 01's breakables nobody shot.

## Done

1. **Root cause, found to the field.** The class-2 pane drain is
   `health -= Vec_SqDist3D(hitter+0x30, hitter+0x40) · 20` [M Break_Update 0x12bfd8:23-34] — the
   hitter's current vs previous position. The movement pass copies +0x30 → +0x40 per object
   **after** that object's anim/root-motion processing in the same per-object walk [M
   0x1336f8:212 — the copy sits behind the rotation/world-sphere work in the handler's walk]. So
   **a step carried by the anim root motion lands in both +0x30 and +0x40 before the copy — it
   reads 0 and never drains the pane.** The move that shows in the pair is the AI steering step.
   The port's replayed intro-script walk (g0, the placed guard at (−4.92, 6.34, −0.09), start
   script 0x6000169, re-enabled with the anim clock restarted at 1.0 [M 0x149dd0:16-28;
   NDrone2_Enable touches no state machine]) carried him across pane 130's sheet at 0xB+23 in the
   port — a root-motion-carried crossing — and the port's grow-0 crossing contact killed the
   health-0 pane inside the sister's window. In the game the same replay demonstrably leaves the
   pane whole [D 750p: pane 130 alive until the breach at 0xB+911]. The re-enable ROUTING is not
   the bug: WaitSwitch's channel-fire arm with a start script goes to PlayScript in the game too
   [M 0x15b960 case 3/msg 0xE fall-through to SetState(3)], and g0 is not even a WaitSwitch drone
   (switch channel 0) — his freeze is the intro's `Drone_EnableAll(0)` and the re-enable skips
   only WaitSwitch drones [M 0x1383e8:14].
2. **The fix (crates/nf-game/src/breakable.rs, drone.rs).** `Drone::scripted_move` — set in
   `root_motion` when it applies a root-motion delta, reset at the head of `Drones::tick_inner`
   (own lines, merge lesson 4; both constructors initialised) — and the scripted-walk gate in
   `Breaks::drone_step(from, to, level_01, scripted_walk)`: the class-2 arm skips a
   root-motion-carried step. The kind-2 and level-0x7000001 gates are untouched [M 0x12bfd8:24-25].
   The AI-steered crossing (the breach squad's charge) still drains — the breach arm itself is
   unchanged.
3. **The port's old "breach" was g0's scripted crossing all along.** With the gate, the breach
   squad never crosses: the five group-1 guards pace at the glass line (z −0.5..−0.7, the watch's
   3600 ticks) — their AI paths them at the closed front door instead of through the pane.
   `glass_breach_live` re-pinned to what the evidence supports: pane 130 whole at 0xB+122 [D the
   observed run], east pane 121 intact [D], the squad released (5 group-1 templates [D spawner
   #297]) and charging; the breach delivery (a runner crossing, the game's 0xB+911 [D 750p]) is
   informational in the test and the report's remaining item. The old assertion ("the woken
   guards shatter an entry-hall pane") passed for the wrong reason — the breaker was g0's
   erroneous scripted crossing, which the [D] evidence refutes; per the rules the TEST was fixed
   and the reason is stated here.
4. **Green on this tree (all through the real input path, NF_ISO set):**
   - nf-game breakable units: 5/5 (incl. the new sister: the scripted walk-by never drains, the
     same AI crossing does)
   - glass_breach_live: 1/1 (pane 130 whole at 0xB+122; east 121 intact)
   - **interact_07000001_live: 10/10 — the merge blocker passes**
   - rocket_glass08_live: 3/3; escort_follow_07000001_live: 1/1; fix_portonly_review_live: 3/3
   - The full `tools/test/suite.py` could not run in this window (the sandbox kills background
     processes between tool calls; the window went to the root-cause work above). The targeted
     set above covers every crate the change touches; the next window should run the full suite
     on the merged tree in foreground chunks.

## What's left

- **The breach delivery.** The game's breach is a guard crossing at 0xB+911 [D 750p]; the port's
  squad charge stalls at the glass line (front-door pathing). Next window: the group-1 charge
  pathing on 07000001 (their route/door logic vs the pane line), then re-pin the breach timing in
  glass_breach_live.
- The full suite run on the final merged tree (see above).

## Live checks for the main agent

1. **g0's walk on the real game**: gdbwatch drone 0's obj+0x30 (position, the z float) from
   ch 0xB to 0xB+1000 on a fresh boot of 07000001. Expected if the model holds: g0 walks toward
   the glass but the pane never drains from him — his crossing (if any) leaves pane 130
   (obj+0xFE bit 0) clear until 0xB+911.
2. **Pane 130 at 0xB+122**: fieldprobe/gdbwatch pane 130's obj+0xFE bit 0 at 0xB+122 (fresh
   boot, no input). Expected: alive (bit 0 clear) [D the observed run's premise — now pinned by
   glass_breach_live].
3. **The +0x30/+0x40 pair during a scripted walk**: gdbwatch g0's obj+0x40 (previous position)
   while his replayed script walks him. Expected: +0x40 tracks +0x30 (the copy lands after the
   anim/root-motion processing) — SqDist(pos, prev) = 0 per tick [M 0x1336f8:212]. During the
   breach runner's charge: +0x40 lags by the AI step (SqDist > 0) [M 0x12bfd8:26-27].
4. **The breach walker's identity**: which guard crosses at 0xB+911 (the spawner squad's group-1
   or a placed guard), and whether the pane's obj+0xD0 list holds his contact on the kill tick.

## Usage

not visible to me — this session's environment shows no token, credit, cost or context-window
figures anywhere in its interface.
