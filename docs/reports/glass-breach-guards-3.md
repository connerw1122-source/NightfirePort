# glass-breach-guards-3 — finish the 08 glass-breach guards (held at merge again)

Branch `s/glass-breach-guards-3`, continued from the held `s/glass-breach-guards-2` (705c,
0e5fbd7). Task: merged onto origin/master the branch broke
`interact_07000001_live::bullets_shatter_the_glass_and_vases_and_the_sister_stays_whole`
(interact_07000001_live.rs:171 "unshot scenery stays whole (the sister)") — the guard
glass-breach path was breaking 01 breakables nobody shot.

## Done

1. **The merge.** `s/glass-breach-guards-3` created from `origin/s/glass-breach-guards-2` and
   merged with origin/master (a8401b57, 57 commits ahead). Only `docs/WORKLOG.md` conflicted
   (both sides appended); resolved by taking master's file and appending the glass entry block
   at the end — no entry lost. No code conflicts.
2. **The contact rule re-scoped to the game's collision-pair semantics** (crates/nf-game/src/
   breakable.rs `Breaks::drone_step`). The game feeds the class-2 arm from the pane's
   `obj+0xD0` contact list [M Break_Update 0x12bfd8:15], filled by the collision pass when the
   body's movement carries it into the pane's collision geometry. The held branch's port test
   grew the pane's AABB by the 0.55 body radius, which registers a "contact" ~0.6 before the
   collision can engage — a proximity kill the game's list cannot produce. The contact test is
   now the step segment CROSSING the pane's bounds (grow 0): a guard walking up to the glass
   has no pair until the collision engages, and a pressed body's position is collision-owned —
   zero net step, zero damage [M the SqDist(pos, prev_pos)·20 rule, 0x12bfd8:23-34; prev copy
   0x1336f8:212]. The kind-2 and level-0x7000001 gates stay exactly as the decompile has them.
3. **Tests green on this tree** (all through the real input path, `NF_ISO` set):
   - nf-game lib: 278/278
   - breakable units (nf-game --lib breakable): 4/4 (incl. the drone-arm sisters: kind != 2 and
     off-01 never take drone damage)
   - glass_breach_live: 1/1 — pane 130 still shatters on the squad's through-movement, east
     wing 121 intact
   - walkreal_07000001_live: 2/2; escort_follow_07000001_live: 1/1; fix_portonly_review_live:
     3/3; rocket_glass08_live: 3/3
   - interact_07000001_live: 9/10 — every test passes except the sister (below)
   - The full `tools/test/suite.py` run: started three times; this sandbox kills background
     processes between tool calls (setsid included), so no full-suite total exists on any tree
     this session. The targeted set above stands in; the next window should run the full suite
     in foreground chunks.

## What's left — the sister, diagnosed to the tick

The merge blocker reproduces on this tree: `interact_07000001_live.rs:171`. Full chain, all
numbers from an instrumented run through `nf_view::load` + `Sim::step_bond`:

- ch 0xB posts at t2119 [D script decode; the port's walk_07000001_live pins it]; Bond's
  `disabled` clears at t2121; the sister's window ends at t2241 (enable + 120).
- The spawner (#297, group 1) releases the squad at t2123-2125. **g0 — drone 0, group 0, a
  PLACED guard standing at (−4.92, 6.34, −0.09), 0.92 from pane 130's sheet (z = −1.01) —
  enters state 0x3 (`NDrone2_DSTATE_PlayScript`, `NDrone2_StateFuncs[3]` [M]) at the enable and
  his scripted root motion walks him into the glass: hull-zone reach at t2129 (0.53 from the
  sheet with the old 0.55 grow), sheet-crossing at t2142 (= 0xB + 23).** The health-0 pane dies
  there — inside the sister's window — and the test fails.
- The game's evidenced breach is **0xB + 911** [D the observed run, task 750p]: pane 130
  shatters and a guard crosses ~900 ticks after the channel fires. The held tree (pre-merge,
  with the switch-channel clearing bug) reproduced t911; this merged tree (with the
  channel-persistence fix) breaks at 0xB+23. The guard walk to the glass runs ~888 ticks too
  early/too short in the port — the divergence is the **scripted-move (state-3 root motion)
  timing/collision for g0**, not the contact rule and not the level/kind gates.
- Evidence that placed drones ARE class 2 in the game (so the arm genuinely applies to them):
  `Drone_Create` 0x136be0 zeroes the DIVars (`+0x30 == 0`), and `NDrone2_CreateObj` 0x14d7c8
  writes `obj+0xFF = 2` on that branch [M]. The breach walker in the game is a ~30-unit walk
  from the hall's far side at the port's own walk speed (0.92 units took g0 23 ticks), which
  matches 0xB+911; the port gives the walk to the wrong guard (g0, 0.92 from the glass) or
  starts it at the wrong time.

**Next window's fix**: the state-3 scripted-move path (root-motion application in the port's
anim/script system) — either the walk's start timing relative to the enable or the actor
binding of the intro script's walk. The contact rule landed here is correct as far as the
decompile goes and should be kept.

## Live checks for the main agent

1. **g0's walk on the real game**: gdbwatch (`tools/trace/gdbwatch.py`) drone 0's `obj+0x30`
   (position, the z float) from ch 0xB to 0xB+1000 on a fresh boot of 07000001. Expected if the
   diagnosis holds: g0 does NOT reach the pane plane (z −1.01) before 0xB+911; his position
   stops ~0.55 short of the sheet (hull line) or he never walks there at all.
2. **Pane 130 at the sister's window end**: fieldprobe/gdbwatch pane 130's `obj+0xFE` bit 0 at
   0xB+122 (fresh boot, no input). Expected: alive (bit 0 clear) — the sister's premise on the
   real game.
3. **The pressed-body contact**: gdbwatch the pane's `obj+0xD0` list while a body presses the
   glass (g0's walk-up): does a class-0x02 entry exist while the body is blocked (position
   stopped)? Expected: yes (the hull touches), and the pane still survives because the pressed
   body's SqDist(pos, prev) is 0 — confirming the damage rule's dependence on real movement.
4. **The breach walker's identity**: which object's class-2 contact kills pane 130 at 0xB+911 —
   the spawner squad's (group 1, placed false) or a placed guard's. If a placed guard does it
   in the game, the port's g0 divergence is timing only; if it is a squad member, the port's
   actor binding of the scripted walk is the bug.

## Usage

not visible to me — this session's environment shows no token, credit, cost or context-window
figures anywhere in its interface.
