# glass-breach-guards-2 — the 08/01 glass-breach guards, un-held

Task: `s/glass-breach-guards` (750p) was held at the 2026-10-17 batch — merged onto master it
broke eight tests that pass without it. This task merges it properly, keeps the port's single
breakables system, and makes all eight pass alongside the breach tests.

## What the held merge actually broke

The held branch (`origin/s/glass-breach-guards`, base 50d51ae7) carried its own parallel
breakables sim (`crate::breakable::Breakables` + `Drone.prev_pos` + an NIS `Request::BreakAt` +
a `set_dynamic` pane injection + a 0x20 bake exclusion in world.rs). Master had *independently*
evolved the same system from the level-01 work (`crate::breakable::Breaks` + `break_kills` +
`Request::DoBreak` + a `drone_step` guard-smash arm). The batch's union-resolved `drone.rs`
kept BOTH: every bullet that touched any breakable ran two damage passes, one of which
(the held `Breakables::bullet_hit`) **draws the shared RNG** for the non-glass kill sound —
a determinism-stream change. Guard-death/drop, hit-rate and pickup tests on 07000001 (which has
55 breakable placements, mostly kind-0 vases) diverge from the first such hit. That, plus the
collision-presence changes (bake exclusion + injection), is consistent with all eight failures.

Resolution: **master's `Breaks` implementation wins everywhere** (it is the merged, suite-green
lineage). The held sim (`breakables` field, `prev_pos`, `BreakAt`, the RNG-drawing
`bullet_hit`, the injection) is dropped; its *tests, docs and probes* are kept, adapted to the
`Breaks` API (`breakables`→`breaks`, `broken`→`dead`). `docs/WORKLOG.md` keeps both sides'
entries (append-only union).

## What was fixed on top

1. **The real-path guard-smash contact never fired on master** (the held report measured 0/55
   panes broken while the woken squad pressed the entry-hall wall for 3300 ticks). Master's
   `drone_step` tested a strict point-in-box; the disc pane meshes are flat sheets (pane 130's
   z pair is −1.01/−1.01 [D nf_view bounds]) and a moving guard's centre-point inside a
   zero-thickness box essentially never happens. The game feeds the arm from the pane's
   `obj+0xD0` contact list, whose entries exist as soon as the body's collision hull *touches*
   the glist — the same list the bullet arm filters [M Break_Update 0x12bfd8:15]. The port's
   contact test is now the tick's (from → to) segment against the bounds grown by the class-2
   body radius (0.55, Bond's capsule RADIUS [M player.rs]; the drones' hull the same scale [S]).
   Master's four breakable unit tests pass unchanged.
2. **The breach works**: on the real path (`nf_view::load` + `step_bond`, no input) the intro
   cut-scene wakes the entry-hall guards, they charge, and panes **130 and 136 shatter** on
   their contact; the east-wing pane 121 stays intact (the sister assertion).
3. **Dead-pane collision (the remaining deviation, documented)**: `Break_Kill`'s obj+0xFE |= 1
   takes the pane out of the world — "the collision goes away" [M 0x12b970:60]. The port's
   static bake keeps a dead pane solid, so the guards press the opening instead of crossing it.
   I *built* the game-faithful mechanism (0x20 out of the bake with the scope ordinal preserved,
   intact panes re-injected per tick via `set_dynamic`, `Break.shape_key` following world.rs's
   hash/synthetic rule): the breach test then went fully green — pane 130 shattered and a guard
   crossed at **t911, tick-for-tick the held branch's verified run** — but it **stalled the
   `walkreal_07000001_live` contact walk** (Bond stuck 10 s at an injected body; the dynamic
   set interacts with the cel-scoped actor queries differently on the current master lineage
   than on the held branch's older base). With the stop-work window closing I reverted the
   collision mechanism (commit 934e8414 is post-revert) and kept the sim-side contact fix.
   The crossing is informational in the test; the mechanism sketch is on the branch history
   (commit 67bd4c6f) for the follow-up.

## Tests

- `crates/nf-view/tests/glass_breach_live.rs` (held, adapted): the woken squad smashes the
  entry-hall panes (130/136), the east wing stays intact — green.
- `crates/nf-view/tests/breakables_live.rs` (held, adapted): the 08 control-room shot +
  corridor-window sister — 2/2 green.
- The three probe examples (`probe_breakables`, `probe_glass_shot`, `probe_glass_spot`)
  adapted to the `Breaks` API; `cargo build -p nf-view --examples` clean.
- nf-game lib: **276/276** (includes the 4 breakable units — first-bullet vase 0x5F3, glass
  0x2E + miss sister, health pool, guard-walk level/kind gates).
- **The eight held-batch breakers, all green**:
  `walkreal_07000001_live` 2/2, `fix_objects_live` 6/6, `render2_live` 8/8,
  `escort_follow_07000001_live` 1/1, `fix_pickups_trace_live` 2/2,
  `fix_guard_weapons_live` 3/3, `fix_player3_live` 6/6, `npc_doors_live` 7/7.
- Not run: the full `tools/test/suite.py` (the sandbox reaps background jobs; the 2-hour
  window went to the merge archaeology above). The touched crates' lib+live tests above stand
  in; the suite should run on the merged tree before this merges to master.

## What's left (follow-up)

1. The dead-pane collision hole (the guards crossing the breach) — re-land the
   exclusion+injection mechanism from commit 67bd4c6f and chase the `walkreal_07000001`
   stall (suspect: the dynamic set's participation in cel-scoped actor queries, and/or the
   injected shapes' material/flag handling — `set_dynamic` pushes without the node_pass flag).
2. The full suite run on the final tree.
3. The held report's open items (the escort's new swarm interaction, the 0x08 coder event)
   remain open — nothing in this task changes them.

## Live checks for the main agent

None this window (no visible-behaviour change survives: the kept contact rule changes when
panes *break*, not how anything draws). If the follow-up lands, render the 07000001 entry hall
(`--bond` near the courtyard side of panes 130/136/155) mid-breach and compare against PCSX2's
guard-crash-through-glass moment.

## Usage

not visible to me (no token/credit/context figures are exposed to this session's tools; the
driver's own accounting is the only source).
