# fix-gate-guard-truck — the 07000005 truck-ride exit's gate guard (channel 0x22)

Branch: **s/fix-gate-guard-truck** (from origin/master dc44c0e4). Task: GAPS G-037 / U-048 — find the
writer of the gate-guard's activation channel 0x22, port it, and add the full truck-ride integration
test (Bond rides the truck from the start to the LOAD_LEVEL of 07000006, through the real input path).

## What's done

**1. The writer of ch 0x22 is found — and it was already ported.** The unknown was knowledge, not code:

- **Writers** [D placements of 07000005, main map]: TouchOnce **#21/#22/#23** (flags 0xEA, out p0 =
  0x23/0x24/0x25, mask 0x8 = player, at (60.15, 28.42, 96.43) / (61.85, 26.53, 82.04) / (58.73, 27.13,
  110.70)) → mux **#25** (0xEF, out 0x22, inputs 0x23/0x24/0x25) — the OR mux writes
  `switch_channels[0x22] = OR(inputs)` [M `Trigger_Update` 0x1b1a78 case 0xc]; plus the valley
  TouchOnce **#305** (out 0x22 direct, mask 0x8). Nothing else on disc writes it: no type-7 script
  event (op 0xD / 0x11 else-branch) touches 0x22, 0x23, 0x24 or 0x25 [D the full script scan], and
  `Drone_InitComms` 0x139520 — the task's named suspect — writes only the alarm pair 0x97/0x98 [M].
- **Readers**: spawner **#268** (mode 2, group 2, max_alive 2, waves 1, start = ext+0x0e = 0x22, kill
  0x43 [D params]) gates its spawn wave on `switch_channels[ext+0x0e]` [M `DroneSpawner_Control`
  0x137e68] and refuses a spawn whose position is visible [M `DroneSpawner_SpawnDrone` 0x137cb0 →
  `Drone_PositionVisible`, gated on spawner +0x1f — the port's U-049b visible_to matches the game].
  Its templates are the gate guards pl 3 (skin 0x05000030) + pl 4 (0x0500002E) standing at their post
  (28.34 / 32.06, 37.12, 44.86); the spawned guards wait on their placement's switch channel (param 3
  = 0x22) in WaitSwitch [M `NDrone2_DSTATE_Global` 0x15b830 → WaitSwitch 0x15b960]. The guard at post
  touches plate **#318** (0xEB, mask 0x10 = drone) → **ch 0x72** [D].
- **The hand-off link** (needed to board at all): the ch-0x5C gatehouse plate is NOT a placement —
  SP #37 (placement 37, scripts [0x06000A22], mode 1 = auto-play from the first `SP_Update` [D
  placement params; M `SP_CreateScriptPlayer` 0x1db9a8:74-86 — the create-time load is for
  mode ∉ {0,1}; mode 1 primes at the first update]) creates it from its **stream 8** (entity
  02000480, type 0xEA, params [0x5C, 8, …]) at its script tick **1884** [L the port's live trigger
  is created and touched at world t1884; D the 05-truck trace's hand-off at t1884; the pre-existing
  nis-script.md/channels.md cite t1882 for the stream-8 window — the two-tick spread is the stream
  window vs the touch tick]. Streams 6/7 create the
  ch-0x70 park plate and the ch-0x71 bed-following Touch; stream 9 fires event 0xD [0x73,1]. SP #38
  (0x060008C8, out 0x5C) is the hand-off cut-scene; SP #320 (0x06000A23, out 0x65) the EndLevel.

**2. The full truck-ride integration test, from the level start** — `crates/nf-view/tests/
gate_guard_truck_live.rs` (new file), every tick through the real input path (`Sim::step_bond` +
`Input`; the walker drives run/turn/crouch/jump — no `bond.pos` writes after the spawn):

- `cold_start_walks_the_gatehouse_and_the_truck_ride_ends_the_level`: cold start #26 (104.95, 25.12,
  108.83) → walk to the gatehouse porch (the courtyard 23.8–23.9 steps ~0.9 up to the porch 24.8
  [D floor probes]; the walk uses the player's own run+hop stair move) → stand on the ch-0x5C plate →
  the hand-off cut-scene → the crouched ride [D fix-truck] → the park → the chain → the exit. Asserts
  the trace's chain order, the guards, the park and the exit:
  0x5C @1884 → 0x22 @3671 → 0x72 @3674 → 0x70+0x65 @5386 (same tick) → 0x73 @5413 → 0x1D @5687 →
  `level_exit 0x07000006`; both group-2 guards present; the park (29.856, 37.929, 52.137) against the
  trace's (29.885, 37.887, 51.656) — y within 0.05, xz within 0.5.
- Sister `without_the_plate_stand_the_handoff_never_fires`: standing east of the plate's footprint
  (x ≤ 101.0 [L live bounds]) for 2600 ticks — the plate is still created at t1884 but never touched:
  no 0x5C, no hand-off, no exit.

**3. A wrong claim corrected by evidence**: the core-loop port-map's item 10(i) says "riding straight
to the truck never spawns the guard" — disproved: the truck's ride path crosses courtyard plate #21's
volume (lo [59.2, 26.0, 94.0]–hi [61.1, 30.8, 98.9]) at ≈(61, 28.3, 96.6) with Bond riding inside it,
so a from-the-start ride raises ch 0x22 mid-ride (the test's 0x22 @3671) and the guard reaches his
post long before the park (the port's guards spawn directly at post — 0x72 three ticks after 0x22 —
while the trace spreads them 34827→36856; the ORDER is what the game pins, and the test asserts it). The walkreal-a ignored test's diagnosis ("the walked Bond stops 1.1 below
the gatehouse floor") is resolved by the same geometry: the porch is ~0.9 over the courtyard and the
y-27.05 quad needs the toucher within 0.55 [M the touch capsule pos+0.15, r 0.55, `Player_Collision`
0x194df0] — the teleport stood at pos 25.88 (gap 0.47, fires) while the edge-sloped walk stopped at
pos 25.64 (gap 0.71, misses). The route in the new test stands Bond at the plate where the porch floor
is full height.

**No code change in `drone.rs`/`triggers.rs` was needed** — every link of the chain is already ported
and cited (the OR mux arm, the spawner's start gate, the WaitSwitch path, the script-entity trigger
creation). Per the rules no change was invented; the task's named files see no diff.

## How it was verified

- The new test (above) on the real input path, deterministic (fixed 60 Hz, the shared RNG, no
  wall-clock). Fail-first evidence: the sister asserts the load-bearing plate touch; the walked-ride
  route itself failed before this session's route (the walkreal-a session's ignored test stood 1.1
  low and never fired 0x5C — its own diagnosis text).
- Neighbours green: `fix_truck_live` (2), `level_end_live` (5), `level_reload_live` — run before push.
- Full suite: see the total line below (the fast runner, final tree).

## What's left

- Nothing for this task. The walkreal-a file keeps its own ignored tests (its file is outside this
  task's list); the new test supersedes its truck route.
- The ch-0x45 spawn reveal plate (pl 36, quad y 24.41) still does not fire under the spawn's feet
  (the documented toucher stand-in reach, drone.rs's toucher comment) — the objective reveal rides on
  it; left to the mission work (and pinned from the other side by fix_truck's standing sister).

## Shared files touched

Only docs: `docs/UNKNOWNS.md` (U-048's ch-0x22 resolution, in-cell), `docs/GAPS.md` (G-037 resolved),
`docs/WORKLOG.md` (appended). No shared source file touched.

## Live checks for the main agent

`switch_channels` u8[256] @0x26FC90 [M triggers.md]; ch 0x22 = 0x26FCB2, 0x5C = 0x26FCEC, 0x72 =
0x26FD02, 0x1D = 0x26FCAD.

1. **gdbwatch on 0x26FCB2 (ch 0x22), 07000005**: walk Bond from the cold start west through the
   gatehouse courtyard over (60.1, 28.4, 96.4) — expect the byte to rise 0→1 on the crossing (plate
   #21, mask 0x8), and the two gate guards to appear at (28.34/32.06, 37.12, 44.86) shortly after.
2. **gdbwatch on 0x26FCEC (ch 0x5C)**: stand Bond on the gatehouse porch at (99, 26, 106.6) from
   level start without moving — expect 0→1 at ≈script tick 1884 (the truck script's stream-8 entity
   creates the plate then), followed by the hand-off cut-scene putting Bond on the truck bed.
3. **fieldprobe screenshot** at the same porch spot before t1884: the truck should be pulling up to
   the gatehouse; after the hand-off, Bond rides crouched (the trace's stance).
4. **nf-view `--bond` spots** (visual, if wanted): `--bond 99,26,106.6` (the porch stand — the
   hand-off fires ~31 s in), `--bond 62,28,96.4` (mid-ride view over courtyard plate #21), and the
   park `--bond 29.9,37.9,52.1` (Bond crouched on the truck bed at the castle gate).

## Full-suite total (final tree)

_(pending — filled in below when the runner lands)_
