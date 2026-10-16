# npc-doors — the drone side of door use: the route-follow step-back, and the door tests

Branch `s/npc-doors`, from origin/master (6bab6409), merged with origin/master (f42de6ff, the menu
repair batch) at 06489006. Playtests 1012-1723 and 1013-1825 (07000001 / 06 / 07): "npcs cant open
doors", "NPC still can't open doors" — the escort stalls at doors, guards and guests don't path
through them.

## The one port bug found and fixed: the route follow never steps a node back

`LinkCreep_Handler` (0x15a2c8) runs `LinkCreep_Decrement` (0x15a148) when the straight feet→target
line fails `AINetwork_BoundsNodeTest(0.2, …)` and the drone is 1.0+ from the target. The decomp:

- with creep step ≥ 1: test the step-back point at r = 0.1; passable → step − 1, retarget (the port
  had this);
- **at creep step 0: `AINetwork_SetupNextNode(-1)` — the route steps BACK one node and
  `LinkCreep_Calc` re-creeps it.** Only an index already past the start (`route+0x80 < 0`) fails,
  and that is what surfaces as the follow's 9/0xc through the route+2 latch [M 0x15b110].

The port's `Route::follow` (nav.rs) returned `BLOCKED` (0xc) instead — a permanent stand. Every
followed route (mission, patrol, combat chase, 0x63) hard-blocked the first time its straight line
to the current creep point grazed an AI wall end, with no recovery. Reproduced on 07000001: the
0x63-driven Mayhew wedged at (2.4, 4.3) — the segment to his next creep point (4.8, 5.8) passes
0.096 from the boundary-wall corner at (3.88, 5.75, 5.11), inside the r = 0.2 test — and stood in
0x63 for 7,200 ticks; the follow log showed `code 12` every tick at creep step 0.

The fix (nav.rs `follow`): at creep step 0 with `index > 0`, run `next_node(net, -1)` and keep
MOVING, exactly the game's Decrement arm. Not modelled: the game's `LinkCreep_Calc(.., back = dir <
1)` backward-creep half (`*suggested*` — the port re-creeps the stepped-back node forward).

### Fail-first proof (the escort test, `npc_doors_live::the_escort_opens_his_door_without_bonds_help`)

- **Without the fix** (fix stashed): Mayhew stalls at (3.2, 6.3, 7.4) — ~4 units short of the
  courtyard shoji's door node pair — in ALLY_LEAD (0x1d) for the full 60,000-tick window; he never
  reaches the pair, never enters `DSTATE_OpenDoor` (0x81), the shoji never leaves CLOSED. That is
  the playtest's "the escort stalls at doors".
- **With the fix**: +3730 the escort enters 0x81 at (7.02, 9.82) with `hit_door Some(1)`; +3760 the
  shoji reaches OPEN with Bond 9.5 away and no use key anywhere in the test; +3818 he has crossed
  to (6.97, 12.36). Stable across the merge (re-run on the merged tree: green).

The same mechanics cover the guards: the combat chase dispatch (`move_to_object` →
`route_reached_special`, fix-guard-misc2) and the 0x63 goal walk both run through `follow`, so the
step-back un-wedges them identically.

## The tests (`crates/nf-view/tests/npc_doors_live.rs`, opt-in `NF_ISO`)

1. `the_escort_opens_his_door_without_bonds_help` (07000001) — the intro's ch 0xB releases Mayhew;
   Bond trails 3.9 behind (outside PlayerInWay's 3.0 band [M 0x15ea28]) and never presses E; the
   escort's own `AllyLeadHide` pauses (an armed started guard within 14 [M 0x15ec08]) are cleared
   through the real fire path. Asserts: 0x81 entered, the shoji reaches state 4, the escort crosses.
2. `a_locked_door_stays_shut_for_drones` (07000005) — the four lock-channelled sliders (ch_lock
   200) refuse the drone-side `Door_Activate` (0x135be0's early gate: channel off + closed → hit
   sound only) and never leave CLOSED; the unlocked swing door #1 (the nav pair's binding) takes
   the same call to OPENING/OPEN as the control. Sister: a 0x63-driven guard walking at the locked
   pair's doorway moves (> 2.0 from his post) but never crosses the door plane, and the door stays
   CLOSED — a closed door that no drone opens still blocks it.
3. `census_of_doors_and_routes`, `watch_07_guests_path_through_doors`,
   `probe_01_wall_at_the_stall` — the diagnostic probes backing the report numbers (05/06/07 door
   and route censuses; the 07 watch shows no guest route crosses a door pair in 24,000 idle ticks
   — the party girls never receive kind-8 mission routes in the port, see Remaining).

## What ran

- `cargo test --offline --release -p nf-game --lib`: 234/234 (branch tree and merged tree).
- `npc_doors_live`: escort + locked tests green on the branch tree and on the merged tree (the
  fail-first run above was the pre-fix check).
- The fast-runner suite on the branch-point tree (master 6bab6409 + worklog): **1700 passed, 6
  failed, 66 ignored (60 min)** — the failures are exactly the allowed set (3 disc_golden on the
  slim ISO + the documented pylon Vulkan-ICD trio). A re-run on the final merged tree
  (f42de6ff) was started in the sandbox's last minutes (`suite2.out`); the total line could not be
  read before the session ended — the next session should read `tail -1 suite2.out` and its
  FAILED lines.

## Remaining

- **07000006 chase variant**: the guard-opens-door chain on 06 is covered by master's
  `fix_guard_misc_live::the_06_guard_driven_through_a_door_pair_opens_it_and_crosses` (0x63 drive
  through the path 7 pair → door #1) and now by the step-back fix for the chase geometry; the
  combat-triggered chase (Bond fired at beyond a closed door) is not its own test yet.
- **07000007 guests**: the watch shows party girls (kind 13, PARTY_GIRL 0x2f) never receive their
  `AssignAIPath(8)` routes in the port (prop 0x1d's route assignment — PartyGirlInit 0x1728c0), so
  no guest walks the kind-8 door pairs (paths 3/6/7 → doors #3/#1). The game's party wanders
  through those doorways. That is a mission-route assignment gap, not a door-gap — it needs its own
  task.
- The `LinkCreep_Calc` backward-creep half of the step-back (`*suggested*` above).

## Live checks for the main agent

1. 07000001, Bond following Mayhew through the courtyard shoji without pressing E: the shoji
   should start opening while the escort stands at its node pair (state 0x81) — `fieldprobe` a
   screenshot at the moment his state reads 0x81, expected the shoji leaf mid-slide.
2. 07000007, the party: a guest crossing the doorway at (80.0, 43.0, -49.9) (door #3) — does the
   real game's party girl open it herself, or does the doorway sit open via its ch_open 130
   channel? `gdbwatch` on the door's obj+0xF4 while a guest approaches.
