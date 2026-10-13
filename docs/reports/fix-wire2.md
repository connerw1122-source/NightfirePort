# fix-wire2 — the cable crossing (07000005) and the 07000014 fusebox dev scenario

Branch `s/fix-wire2`. The user's playtest of 2026-10-12: "pressing e didnt do anything so I lept at
it over the ledge and it kind of grabbed it. I then couldnt really move forward so I kept jumping 2
more times to the other side." Plus: the dev scenario for 07000014's fusebox "spawns above the vent
I'm supposed to be in. So likely need to spawn in there crouched."

## What the decompile says (the law)

- **E never grabs wires.** `Player_Activate` 0x1a5170's sphere-dispatch handles doors (0x1C),
  triggers (0x1D), SSs (0x25), cars, GTs, monitors, locks, gunimps, PCQ worms and SPs — **no
  class-0x2C arm** [M 0x1a5170:95–160]. So "E did nothing" at the cable is the GAME's behavior; the
  port's fix-wire claim that "the use key runs the same scan" was wrong (no code ever read the use
  key for wires — the wire_live test only passed because its Bond stood inside the sphere so the
  walking scan grabbed).
- **The two real grab paths**: (1) the walking collision — `Player_DealWithObjHit` 0x194458
  dispatches a class-0x2C hit to `Player_CollWire` 0x197130 [M]; (2) **the jump grab** —
  `Player_HandleJump` 0x1916b0:118–147: with the third-icon context `BLData+0x95F == 3`, the jump
  press scans ALL objects for the nearest class-0x2C within 1000 units (no capsule test) and grabs
  it, eating the jump [M].
- **The icon context** comes from the third icons: `ThirdIcon_Create` 0x197070 (placement kind
  0xFA; instance+0x00 = param 0 = the context type, +0x04 = param 1 = the on-channel, 0 = always
  enabled); `ThirdIcon_Update` 0x196e80 walks the icon's room list, finds the PLAYER
  (`Player_Init` 0x1910f8:76 sets the player's class to 3) and writes the type to `BLData+0x95F` on
  a 4-tick refresh; `Player_CollisionHandler` 0x1949a8:22 resets +0x95F to 0xFF every tick [M].
  **[D probe]** 07000005 map 50 carries three type-3 icons right under the cable — (−23.58, 32.24,
  2.65), (−23.56, 32.24, 0.21), (−23.80, 32.74, 12.33); the wire is at (−23.49, 34.47, 6.26). The
  game's hint at the cable is real and the port never had it.
- **The hand-over-hand move axis is the STRAFE axis (action 1)** [M `Input_Actionf(…, 1, 1)` at
  `Player_Wire` 0x197658:170; `psiInput_MapInputs` 0x10cd80 style 0: action 1 = field +0x18 = the
  left-stick X]. Forward (action 2) does nothing on the wire in the game too — the user pressed W
  and concluded the crossing was broken; on PS2 the stick's left/right moves hand-over-hand.
- **An idle (mode 6) with move input skips the `AnimScriptIsStopped` gate** [M 0x197658:186–219 —
  the `+0x94D == 6` branch only wraps the idle re-add; the move tree falls through ungated]. The
  port gated ALL modes, so a strafe press during the idle waited out the whole idle cycle.

## What was done

- **The jump grab ported** (`crates/nf-game/src/player.rs`, `Bond::physics_tick`): the grounded
  jump press with `icon_ctx == 3` grabs the nearest wire within 1000 units (`grab_wire`), exactly
  the decompile's scan; a refused grab still eats the jump (the game returns from HandleJump
  regardless). `Bond` carries the new `icon_ctx: u8` (`BLData+0x95F`).
- **The third icons parsed** (`crates/nf-game/src/wire.rs`): `Icon { pos, ctx, on_channel }` +
  `wire::icons(level)` over the 0xFA placements; the Sim (`crates/nf-view/src/scene.rs`) resets
  `bond.icon_ctx = 0xFF` per tick and writes each enabled icon's type — same-cel scoping with a
  4-unit sphere fallback standing in for the game's room-list walk [S]. The type-6 creep-wall
  context and the type-2 HUD context are recorded but not consumed (their systems are out of scope).
- **The mode-6 move gate fixed** (`player.rs::wire_tick`): the `AnimScriptIsStopped` gate now only
  applies when the mode byte is NOT 6, so the strafe press during the idle starts the move on the
  press — the user's "couldn't really move" start lag is gone.
- **The E record corrected** (`scene.rs` comment + `tests/wire_live.rs` renamed/rewritten): E does
  nothing at wires, game-faithful.
- **The `fusebox-07000014` dev scenario** (`crates/nf-view/src/dev.rs`): Bond starts CROUCHED
  INSIDE the level's crawl vent at (41.0, 2.45, 62.0), yaw π (facing the drop), with the Laser
  (0x4E — the disc's cutting weapon [L weapon_data]) owned, loaded and selected. Evidence: the
  level's only crawl-height tunnels ([D collision probe]: headroom 1.44–1.46 over floor 2.15–2.40
  at x 31–41, z 36–65; the cold start (40.81, 2.40, 65.08) is the tunnel's north end; the vent
  graphics dress those runs). A whole-level crawl scan found NO vent near the west generator room
  (the fuseboxes #383/#385 at y −2.2) — the user's "the vent I'm supposed to be in" is the level's
  vent system, and the scenario now starts inside it crouched instead of above it.

## How it was verified

- Reproduce-first: `crates/nf-view/tests/fix_wire2_live.rs` (3 tests, all green) —
  1. `the_jump_at_the_cable_grabs_it_and_e_does_nothing`: Bond standing at the icon spot on the
     south ledge, outside the world sphere (pre-checked), `icon_ctx == 3`; E → still WALK (the
     game-faithful sister); the JUMP → substate 6, cam mode 0xB, hang at centre.y − 1.0207, snapped
     onto the wire. Pre-fix this test cannot compile (no `icon_ctx`) and the behavior was: the jump
     leaps off the ledge (the pre-fix code is in HEAD~3).
  2. `the_hand_over_hand_move_crosses_on_the_strafe_axis`: the strafe press DURING the idle moves
     Bond within 25 ticks (pre-fix it waited the idle track out); W (forward) never moves him
     (sister, band 0.08 above the ±0.0542 sway); the full crossing reaches the far end alive.
  3. `the_fusebox_scenario_starts_crouched_inside_the_vent`: the scenario places Bond crouched
     (cc = CROUCH_ROOT_HEIGHT) inside the tunnel (crawl headroom < 1.7 verified), the Laser
     selected, still crouched and inside after 60 ticks.
- `tests/wire_live.rs` on the merged state: 2 passed, 2 ignored — the driver's fix-wire2 × walkreal-b2
  merge removed the walk-in/use-key wire grab entirely (their comment: unproven against
  `Collide_PickObj`'s query versions until measured), so the walk-in legs are `#[ignore]`d there;
  the jump-grab path this task ports is the grab that remains, and it is the one fix_wire2_live covers.
- Full suite (origin/master merged in — walkreal-b2 / render-artifacts / npc-doors2 / wallhug-freeze):
  605 + 374 passed, 3 failed — the 3 `disc_golden` failures the slim ISO allows (`cargo test
  --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view --
  --test-threads=1`). All fix-wire2 tests green on the merge.
- **The merge carried a parallel worker's wire change** (npc-doors2/walkreal-b2): the walk-in
  auto-grab was REMOVED on master (their evidence: `Collide_PickObj`'s world-sphere branch runs
  only for HITTEST versions 0x200/0x201, and a wire has no glist triangles for the other versions —
  so the walking pick cannot hit a wire at all), replaced by a cel-gated use-key grab ([S] stand-in
  for the jump-grab, fixing 07000002's bunker swallow). The merge keeps BOTH: their cel-gated E
  affordance (needs the capsule inside the sphere) and this task's decompile-faithful jump grab
  (icon ctx 3 + nearest wire within 1000 [M 0x1916b0:118-147]). At the user's ledge spot the
  capsule is outside the sphere, so E still does nothing there — the jump grabs.
- Merge fixup: nf-extract's `Player` literal gained the new `capsule` field (standing stance).

## Shared-file hooks (small, additive)

- `crates/nf-game/src/player.rs`: the `icon_ctx` field (own line in the constructor), the jump-grab
  arm in `physics_tick`'s jump branch, `physics_tick` grew the `ctx` parameter (all five callers are
  in the same file), the mode-6 gate condition in `wire_tick`.
- `crates/nf-view/src/scene.rs`: the `icons` field + the load-time `wire::icons` call, the per-tick
  icon-context write, the grab-scan comment correction; the merge kept master's `wire_cels` +
  use-key grab alongside.
- `docs/WORKLOG.md`: appended only.

## What's left

- The wire's third-person camera (`cam_mode = 0xB` is recorded but the viewer still renders
  first-person; carried over from fix-wire). With the camera side-on, the strafe crossing reads
  naturally; in first person the user looks along the cable and reaches for W. Renderer work.
- The type-6 icon → `Player_Activate`'s creep-wall grab, and the type-2 icon's HUD pane context:
  the icon plumbing now exists, those consumers are separate tasks.
- The icon's room-list scoping is approximated (same cel / 4-unit sphere) — a live check below can
  pin the exact room radius.
- The user's "jumped 2 more times to the other side" artifact: jumping on the wire drops Bond with
  a 30-frame regrab lockout and the walking scan can re-catch him mid-fall — game-faithful on both
  ends (the drop [M 0x197758] and the collision grab [M 0x194458]), left as-is.

## Live checks for the main agent

1. Icon scope radius: in PCSX2 with `gdbwatch.py`, watch the write to the player's `BLData+0x95F`
   (the player object at EE ≈ 0x16cb8a0's control data) while walking the castle-gate ledge toward
   and away from the cable — expect 3 within the icons' room and 0xFF beyond it; note the distance
   where it flips (the port uses same-cel OR 4.0 units).
2. The jump grab in the real game: stand at the icon1 spot (−23.58, 32.24, 2.65 area), press jump —
   expect the grab to fire from the ground (Bond never leaves the ledge); the port now does this.
3. Render check (`--bond` spots): `--bond -23.5,31.5,2.6,3.1,0` should show the cable overhead and
   the floating grab icon; on the wire (`--bond -23.5,33.4,4.3,0,0`) the hand-over-hand pose.

## ntfy

Sent to https://ntfy.sh/nightfire-yoda-8k3x (see the finish step's output below if it failed).
