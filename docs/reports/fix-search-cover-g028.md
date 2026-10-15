# fix-search-cover-g028 — guard cover use (G-028, part of G-026)

Branch `s/fix-search-cover-g028`, cut from `origin/master` cf4f33cd (unchanged during the session; the branch
is the final tree). Commits: f99edf1d (worklog), d34a6810 (the work below).

## What the task found vs what it asked

The task brief said the port "runs the cover pass after vision … and has no real cover-node selection". The
first half was true; the second was outdated. Reading the chain against the decompile showed the selection was
already ported and live-verified (WORKLOG 0023 against `work/chase_trace.tsv`, sniper/ambush in 0079, scary
objects by fix-cover-remainder):

- `NDrone2_FindCover` 0x152548 — ported as `drone.rs::find_cover`: the unoccupied nodes on the drone's nav
  network, smallest emitter distance byte (strictly less, first wins ties), byte² ≤ (opponent distance)²,
  `CoverNodeOK(node, 0, needLos)`, then the claim (owner, flag 0x10, side, approach/in-cover points,
  `+0x4F8 |= 0x800|0x1000`). One decompile subtlety checked and NOT ported because it is dead code: the
  early return when `needLos == 0 && (+0x4F8 & 0x1000)` [M 0x152548:100-102] — every decompiled caller
  (`CombatState` 0x148338 ×4, `CombatNoRoute` 0x164c08, `UnderCoverIdle` 0x169e58 ×2, `BotAttackRun`,
  `BotAttackCrouch`, `BotCoverIdle` ×2, `BotCoverTypeChange`, `UnderCoverTypeChange`) passes needLos = 1.
- `NDrone2_CoverAvailable` 0x155a38 — ported as `cover_available` (prop 0x30, claimed → yes, opponent visible
  and < 4 → no).
- `NDrone2_CoverNodeOK` 0x151d68 — ported as `cover_node_ok` (side bits & 0xE, LOS bits & 0x70 with needLos,
  ≥ 2.0 from the opponent, ≤ 1.5× combat range with prop 0x44, the anim predicates, the 1.5 crowd check).
- The enter/leave states 0x8C → 0x93/0x94 ⇄ 0x95 → 0x96, leave 0x99/0x9A/0x9B → 0x9C — ported; the RNG inside
  the episode matches the game's order, re-verified in the decompile this session: `UnderCoverIdle`'s timer C
  is `Rand_Rand(5)` FIRST, then `CoverAvailable(1)` → 0x8C, else `CoverNodeOK(…, 1)` → 0x95, else
  `CoverAvailable(1)` → 0x8C, else the tries gate (+0xCD8, old value < 3) re-arms `Rand_FRand(150)` [M
  0x169e58:132-162] — exactly the port's arm (`drone.rs`, UNDER_COVER_IDLE / msg::TIMER_C).

## What was changed

1. **The tick order (the real deviation).** `Drone_InitComms` [M 0x139520:124-284] runs
   `Drone_ProcessCoverNodes` (0x138e78; cadence counter −2, < 1 → 10 — port matches) FIRST, then
   `Drone_ProcessOpponents` (0x1390c0) at :151 and `DroneVision_ProcessDroneSight` (0x176a28) at :162. The
   port ran the cover-node walk after the vision pass AND after the nav-node cache. Moved to the game's
   position in `Drones::tick` (drone.rs), before the opponents pass. **Observability**: nothing between the
   two positions in the port's tick mutates the walk's inputs (the switch channels — use-objects/WorldObjects
   run after both; Bond's position/eye; the collision world; the node owners — claims happen in the per-drone
   control after both), and the game's own pass inputs are the InitComms-entry snapshot either way, so the
   move is a conformance fix with no behavioural delta; the whole cover/AI suite is unchanged by it. No
   failing test was written for the move because none can observe it — stated per the reproduce-first rule
   rather than guessing at a synthetic one.
2. **docs/systems/drone-ai.md** — new "Cover [M, L]" section with the addresses above (tick order, selection,
   stance, enter/leave + RNG order, port map), and the stale port-status line ("Not yet (G-026/27/28): cover,
   grenades, …") corrected to what is actually open (hearing beyond the sound-alert chain, per-target cover
   slots, the 0x86 flee route update, U-030).

## Tests (NF_ISO, 07000005)

New `crates/nf-game/tests/cover_selection_live.rs`, through the real level data:

- `cover_choice_follows_the_game_scoring` — the chase-trace scenario (gate guard, Bond at 79.86, 22.92, 96.0);
  asserts the guard claims node 1 (the live trace's node) and that the scoring rule replicated in the test
  over the level's real nodes picks the same. **The replication was shown RED mid-session when the
  `CoverNodeOK` side/LOS gates were omitted: it then picked node 3 (emitter byte 1) over the trace's node 1 —
  the gates are load-bearing, and the test discriminates a scoring deviation.** Sisters: the winner satisfies
  byte² ≤ opp-dist², and the network carries > 1 candidate (the ordering is not vacuous).
- `cover_stance_follows_the_node_flags` — node 1 re-flagged crouch-only (`+0xC`: keep 4, clear 8): the guard
  still picks node 1 and takes a crouch cover type (0 crouch-lean / 1 crouch-step / 4 as-low) with no stand
  anim (0x17/0x18/0x1A/0x1B/0x1C forbidden); sister with stand-only flags: type 2/3, no crouch anim. This is
  the "crouches/stands per the node flags" pin. Both directions pass on the shipped selection — pinning tests,
  not fail-first (the flag logic was already correct; the red scoring run above is the demonstrated failure).

Existing cover coverage stays green: `ai_live.rs` 35/35 (incl. `guard_takes_cover_like_the_game` — the live
trace match), `drone_portcheck_live.rs` 5/5 (incl. `under_cover_idle_retries_three_times`). The slim sandbox's
`work/` carries no `ai_trace_*.tsv` / `chase_trace.tsv` (only `action-symtab.tsv`), so the trace matching rides
on that pre-existing test, whose measured columns (node 1, UnderCoverInit 31 / Aim 37 ticks, the in-cover
position 58.83/99.32) come from the 0023 session's live capture.

## Full suite

The fast runner (`tools/test/suite.py`) was restarted after an early startup failure (missing `cargo` on the
background PATH, then `alsa-sys` needing `PKG_CONFIG_PATH=$HOME/alsausr/...` — both fixed in the relaunch) and
ran on the final tree. Result: see "Suite total" below.

## What's left (for the next session, if any)

- Per-target cover slots: the game walks all 8 target slots; the port models slot 0 (Bond) only — the known
  deviation, documented at the moved pass. Ally-vs-drone cover needs it.
- U-030 (RunForCover route shape near the gate, live 198 vs port 114 ticks) — still open, untouched here.
- The `RecoverFromScaryObject` (0x86) flee route update (G-028 remainder) — untouched here.

## Shared files touched

- `crates/nf-game/src/drone.rs` — the cover-node walk block MOVED (verbatim) from after the nav-node cache to
  before the opponents pass; three comment lines updated. No other lines changed.

## Live checks for the main agent

PCSX2 (07000005, tools/trace):
1. Spot: Bond teleported to (79.86, 22.92, 96.0) — the chase-trace spot. Action: let the gate guard engage.
   Expected: `Drone+0x108+4` walks 0x58 → 0x5A → 0x8C → 0x93 → 0x94 → 0x95 → 0x96; `+0xBB4` = `CoverNodes +
   1×0xB0` (node 1 at (59.11, 98.69), yaw −90°); `+0xBB8 & 3` = the side (2 here); he STANDS behind the pillar
   (idle anim 0x1A, lean-out 0x18 at aim — both-stance node, the seed picks lean).
2. Spot: same. Action: `gdbwatch` on `iTimeToCalc.255` (0x0030caa0) and on node 1's usability word
   (`CoverNodes + 0xB0 + 0x90`, 0x283E10 base). Expected: the counter decrements by 2 per frame and resets to
   10 every 5th frame; on reset frames the node words refresh BEFORE the drone-sight round-robin writes its
   seen latches the same frame (the order this task moved).
3. Spot: same. Action: poke node 1's flags word (`+0xC`) to 0x1E4 (crouch-only). Expected: the same episode
   ends with the guard CROUCHING behind the pillar (`+0xBBB` ∈ {0,1,4}, idle anim 0x10). Poke 0x1E8
   (stand-only): he stands (`+0xBBB` ∈ {2,3}).

nf-view (--bond):
- `--bond 79.86,22.92,96.0,-2.55,0.05` — looking at the gate from the trace spot: the gate guard runs behind
  the gate pillar (59.11, 98.69), stands in its lee, leans out to fire in bursts.
- `--bond 67.31,26.5,99.47,-2.8,0` — Bond 6 units away, in the open: the same guard fights in the open
  (Combat/AimStandFire, steps) and never runs for cover (the sister).

## Suite total

(pending — filled in before the finish step)
