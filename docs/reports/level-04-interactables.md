# level-04-interactables — 07000004 "Enemies Vanish" (Castle D)

Branch `s/level-04-interactables`, task 704a. Every claim below carries its evidence tag.

## What's done

1. **The census** (`docs/levels/07000004.md`): every placement of the main map (70, 0 unresolved
   [D]) by kind, with the game's behaviour [M] and the port status. The decisive structural finding:
   the game creates dynamic objects from ONE map per archive — `parsemap_parsemap` 0x1d1cf0's
   state-2 epilogue processes `DynamicPtr` (the LAST block-0x1a; `parsemap_handle_block_id` 0x1d1020
   case 0x1a), and the type-1 main map is the last map-type entry in all 35 level archives [D
   `nf-extract bin-list`; 07000999.bin is the one archive without a main map]. The other 49 maps are
   weapon/effect templates plus a shared castle "interactables prototype" map (map 1's
   Switch/Lock-Laser/Lock-Key/Pickup-Key/KeypadLocked/alarmswitchplayer/camera-lens) — dead data.
   That also settles the task brief's open items: the "key-gated route" lives on 07000003 (its exit
   gate ch 0x52 → the Load Point into this level); the "cable car / gondola pieces" are absent from
   this level's disc data. No doors, lifts, switches, locks, keypads, safes, monitors, lasers,
   searchlights, cameras, turrets, fuseboxes, vents, hatches, ladders, grapple points or vehicles
   exist on this level's main map [D + the 0x1d04f0 dispatcher table [M]].
2. **Two real port bugs fixed with [M] evidence**:
   - **Phantom switches + AI points on every castle level.** `Drones::attach_level` collected 0x29
     switches and 0xE9 AI points across ALL maps (`level.maps.iter().flat_map`) — the template
     maps' "Placement - Switch"/"alarmswitchplayer"/"AI_Point" at the origin became phantoms in the
     port's AI state (guards could be sent to a switch that does not exist in the game). Fixed to
     the main map only [M parsemap epilogue]; pinned by
     `interact_07000004_live::no_phantom_switches_or_ai_points_from_the_template_maps`.
   - **LeafGens 0x2B and Trees 0xF3 never created.** `LeafGen_Create` 0x1acbd0 / `LeafGen_Update`
     0x1acc48 (one falling leaf per Rand_Rand(30) gate, 0.02-band jitter [M]) and `Env_Tree_Create`
     0x17c200 (object + AnimObjectNew + AnimScriptAdd; the 0x600008c→0x600008d Rand_Random swap [M
     :29-35]; the pause arm for every level but 0x7000043 [M :44-52]) now create — world_fx
     `LeafGenFx`/`TreeFx`, the gate on the counter-hash (the RNG discipline: the game draws these
     from the shared stream, the port does not). The leaf's motion integrator and the world-object
     sway-anim runtime are NOT ported [S unverified] — the state is there for the draw code.
3. **One live test per KIND through the real input path** (`crates/nf-view/tests/
   interact_07000004_live.rs`, **10/10 green**), each with a sister that fails without the
   operation: weapon pickup (walk over #7 → weapon 18 owned + the respawn-2 hidden/state-2 park;
   loads through `nf_view::load_weapons(..., &[18])` because `Player_EquipWeapon` refuses a weapon
   whose FP model is not loaded [M 0x1a8a58]), armour pickup (walk over #9 → armour 50 [M
   0x18e650:220-225]), the target's real-fire kill → ch 0x54 [M 0x13e660], the OUT trigger's
   0x54→0x64 copy (no copy pre-kill), the pond ripples (4 gens, the gas pool alive), the two
   ScriptPlayers (hashes 0x06000082/83 + the game-skip settle → AI on, ch 0x53), the rain env (drop
   type 1 + the rainboxes), the leafgens (2, spawns tick up), the trees (model 0x05000003, anim
   0x0600008c, paused), the phantom rule. The walkreal assassination route
   (`walkreal_07000004_live`) still passes.
4. **Models resolve**: the level parse resolves 70/70 placements (0 unresolved [D `nf-extract
   level-summary`]); the census lists the model hashes for every kind. `NF_TEX_TRACE` was not run
   (no GPU render in this sandbox — the main agent draws); no missing-hash fallback can come from
   the parse itself, but the texture fallback sweep is left to the render pass.

## What's left (with the exact reason)

- **The rigid bodies 0xD8** (#39/#40 `rb_whitelamp`): unported — `RB_Create` 0x1d6bf0 pulls in the
  full rigid-body physics system (`RB_Update` 0x1d6e38, `RB_ApplyImpulses` 0x1d6ff8,
  `RB_RestingContact` 0x1d7168/0x1d73a0, `RB_CalcTorque` 0x1d7658), a whole system, not a
  level-sized fix. The two carpark lamps create nothing and don't draw in the port. Everything else
  interactive on this level works.
- **The leaf draw + tree sway**: the `LeafGenFx`/`TreeFx` state exists (positions, hashes, spawn
  counters) but no renderer consumes it yet — the main agent's draw pass owns that.
- **The mission layer**: ch 0x64 (mission-complete) has no consumer in the port (Mission_Update
  0x182c60 un-ported) — the level's completion loads the next arc's DRIVING level (0x7300004),
  outside the on-foot scope. The walkreal test asserts the raise and the copy.
- **Tree anim swap 0x600008c→0x600008d**: the game picks per-tree via `Rand_Random` [M 0x17c200:29-35];
  the port records the disc hash 0x0600008c only (no shared-stream draws) — the deviation is
  documented in world_fx.rs.

## Live checks for the main agent

- `--bond -8.13,4.8,26.7,3.1,0` — under the west Leaf_emmit: falling leaves by the carpark lamp
  (the port creates the spawns; the draw needs the leaf particle renderer).
- `--bond 5.0,4.8,30.0,-1.5,0` — the two trees at (4.64, 3.84, 26.49)/(−7.79, 4.39, 27.39) draw
  their 0x05000003 model; PCSX2 shows a gentle sway (the port holds bind pose — accepted deviation).
- `--bond -10.66,4.4,42.66,2.3,0` — pickup #7 before/after a walk-over: the uzi model disappears
  (respawn 2 → hidden ~20 s, then returns).
- The two whitelamps at (−12.05, 7.86, 23.03)/(8.01, 7.86, 23.03): PCSX2 has them resting on the
  carpark wall; the port does not draw them (the 0xD8 gap above).

## How it was verified

- `cargo test --offline --release -p nf-view --test interact_07000004_live` → 10 passed.
- `cargo test --offline --release -p nf-view --test walkreal_07000004_live --test worldobj_live`
  → 6 passed (no regressions from the main-map fix).
- `cargo test --offline --release -p nf-game` unit tests incl. the new world_fx leafgen test.
- The full-suite fast runner (`tools/test/suite.py`) was started twice in the background; the first
  launch died at once (`cargo` not on PATH in that shell), the relaunch at 12:56 UTC cannot complete
  inside this sandbox's ~13:03 UTC expiry. The per-crate runs above stand in; the final-tree suite
  run is left to the next session (the branch is merged-clean with origin/master as of the branch
  point d628f01f + these commits).

## Usage

not visible to me — this sandbox exposes no token/context-window/cost figures to the agent.
