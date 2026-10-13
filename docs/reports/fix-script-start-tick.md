# fix-script-start-tick

Branch `s/fix-script-start-tick`. The playtest bug: on 07000005, stepping on the wall-hug plate
made the two gate guards vanish ("why the hell would they disappear?") — the port removed them on
the touch tick and the gate stood empty for the whole ~1,400-tick wall-shimmy scene; the real game
keeps them present throughout and swaps in the two placed guards behind the gate when the scene
ends [L docs/fields/hitdata-spawner-ally.md "Live result"].

## What was wrong

The previous attribution ("the script player runs its first commands one tick after `start`",
documented as a deviation in `spawner_kill_live.rs`) was **half right**: the first commands do run
one tick late in the port, but the script player's clock and load path were already faithful —
`SP_LoadScript`'s trailing `Script_Update` [M 0x1dbc08:56] runs the first commands inside the load,
same tick, exactly like the game. The real cause was **where the players ran**: `drone.rs` updated
all script players together in `Scripts::tick` BEFORE the control walk (the old "all of them here"
deviation). The game runs every object's control func in ONE newest-first walk
(`control_movement_object_handler` 0x1336f8:54-68, `control_funcs[obj+0xFF]`; the list is prepend
order [M 0x134698]), so:

- the TOUCH_ONCE plate #301 (placement 301) writes ch 0x43 **in the walk** (`triggers::calc`
  handles only the continuous TOUCH family);
- SP #298 (placement 298) then triggers on it and its first commands run in the same walk — event 5
  → `Drone_EnableAll(0)` → `NPCGlobals = 0` [M 0x1d94c0 case 5, 0x1383e8];
- the gate spawner (placement 268) returns at its first line [M 0x137e68:21-23] with the kill latch
  still off — for the whole scene, until the script's end re-enables the NPCs (stream 8's event 7 at
  clock 1398 → `Script_Free`'s 0x80 flag [M 0x1da880:41-43]).

In the port the SP saw ch 0x43 one tick late (the plate hadn't fired yet during the pre-walk
`Scripts::tick`), and the spawner — inside the walk, AFTER the plate — latched and removed the pair
on the touch tick.

## The fix

- `nis.rs`: `Scripts::tick` split into `begin_tick` (requests reset, the skip take, the create-time
  prime beat — unchanged semantics) and `update_player(i, …)` (one `SP_Update` from the walk).
- `drone.rs` (shared file, hooks listed below): the players joined the control list as kind 5 at
  their placement (the same newest-first sort); the request drain runs once before the walk (the
  priming/copter requests) and after every player's update (`drain_script_requests` — the freeze
  must gate the control funcs behind the SP in the same frame, which in the game is synchronous
  inside `Script_EventHandler`); `set_dynamic` moved after the walk (the game's `Collide_Update`
  0x1336f8:238 is post-walk — Bond's next movement reads this walk's script-object and door poses,
  so the 07000008 ride keeps its one-tick-behind-Bond lag instead of gaining a second one; the
  door pose in the dynamic set is same-walk now, matching Collide_Update's position).

## Verification

- **Fail-first NF_ISO tests** (`nf-view/tests/spawner_kill_live.rs`, rewritten per the task):
  - `the_wall_hug_scene_holds_the_gate_pair_and_swaps_the_pairs_at_its_end` — the full 07000005
    sequence through the real input path, asserting at the touch tick (ch 0x43 on AND
    `npc_enabled` false AND the gate pair present AND the spawner alive with the latch off),
    mid-scene (+300 ticks: the same, the placed pair hidden), and after (the re-enable, then the
    pair removed, the spawner gone, the placed pair enabled — the 0x8b0 → 0x880 flip [L]).
    Pre-fix this fails at the touch tick (freeze at touch+1, the pair already removed) — probed on
    master's code before the fix.
  - `the_wait_first_stream_starts_its_clock_on_the_start_tick` — the sister for a script started
    on a tick whose first command is a wait: the scene's stream 12 is `Wait(369)` → `Sound(368)`
    [D 0x06000994 decoded]; the sound must land at touch+369. Probed pre-fix: touch+370.
  - The file's other three tests (the no-kill-channel sister, the NPCGlobals-gate latch hold, the
    07000011 stairwell keep) pass unchanged.
- **Re-checked the sequences the shift could touch**: the full suite is the arbiter — nf-game
  23/23, nf-view 85/85 (the cut-scene intros incl. 07000001's, `fix-mission`'s channels, the
  07000008 ride chain, the truck chain, the 07000011 chains), nf-audio + nf-formats green except
  **the 3 allowed `disc_golden` failures** (slim ISO).
  `NF_ISO=<abs> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`.
- **One re-baseline** (shared test file): `fix_player3_live.rs`'s grenade damage pick used
  `guards.first()` and assumed his 3-units-back spot sits on his own floor. With the players in
  the game's walk order, a channel is seen only by the objects updated after it — the same rule
  the triggers already follow — and 07000001's first enabled guard's patrol phase shifted, putting
  the spot by a ledge (Bond fell 3.5 units; the grenade exploded out of reach). The pick now
  requires the spot's floor to match the guard's (test-side only; the sim asserts unchanged).
  Verified fix-specific: the test passes on origin/master in a worktree.

## Shared files touched beyond the task's list (all minimal)

- `crates/nf-game/src/drone.rs`: the kind-5 walk arm; `drain_script_requests`/`script_obj_pos`
  extracted (master's fix-party-mission re-enable/hud_frozen/0x172 arms live in the shared drain —
  merged mid-task); `script_sounds` made `pub` (the sister test's observable); the `set_dynamic`
  move; `begin_tick` at the old `Scripts::tick` call site.
- `crates/nf-view/tests/fix_player3_live.rs`: the grenade pick (above).
- `docs/port-map/core-loop.md`: divergence 9's fix note (append-style, the entry's own tail).
- Merged `origin/master` (22 commits: fix-party-mission, fix-death-restart) and re-based the
  re-enable arm onto theirs.

## What's left / live checks for the main agent

- The wall-hug scene's visible order is pinned by the tests; a render check would confirm it
  visually: `--bond 24.9,37.2,71.5,0,0` at the plate (the gate pair present, the placed pair
  hidden), again ~700 ticks into the scene (unchanged), and after the scene (the gate empty, the
  placed pair visible). The port has no GPU here.
- `Scripts::cutscene_playing`'s skip window and the viewer's skip path were not re-verified against
  the game (no behavioural change expected — the skip rides the same tick).
- The script *entities'* own list position (core-loop divergence 9's remainder) is still
  approximated (their triggers get `usize::MAX - o`); the players' fix does not cover entities.
