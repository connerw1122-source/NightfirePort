# fix-walkreal-b2 — walkreal-b's work landed on master, green

Branch `s/fix-walkreal-b2`, cut from `origin/master` (323bf00). Task: bring over walkreal-b's
walker harness, the door lock-edge fix and the per-level walk tests — without walkreal-a's
`drone.rs` nearest-door stand-in and without `agent-ctx/` — and leave no test red.

## What landed

1. **The door lock-edge fix, with its test.** `crates/nf-game/src/door.rs` `Doors::end_tick`:
   the game snapshots the switch-channel latches with TWO amemcpys at the head of
   `control_movement_object_handler` [M 0x1336f8: `amemcpy(0x26fd90, 0x26fe90, 0x100);
   amemcpy(0x26fe90, 0x26fc90, 0x100)` — `prev ← hold; hold ← channels`], so `prev` lags the
   live channels by two snapshots and a channel pulsed since the last snapshot still shows a
   rising edge in `Door_Update`'s `prev == 0 && now != 0` test [M 0x136550-0x136608]. The port's
   single copy made the Q-Decryptor's burn invisible to the edge (door[19] stayed locked). The
   fix's test is `walkreal_07000012_live::walk_spawn_to_the_shaft_exit` — the burn → ch 0xa →
   the edge → door[19] `state & 0xc` → the shaft exit, all through the real input path.
2. **The walker harness**: `crates/nf-view/tests/common/walker.rs` (walkreal-a's, + one new
   method) and `crates/nf-view/tests/common/tower2.rs` (the shared 07000012 route). Per-level
   walk tests: `walkreal_07000001, 09, 0a, 0b, 0c, 0d, 11, 12, 13, 14 _live.rs` (the 10 that
   exist at walkreal-b's tip; b itself deleted a's 02-05). Two probes kept as evidence:
   `walkreal_probe0a.rs` (the cab-mouth leaves, their keys and the warm pin) and
   `walkreal_probe13.rs` (the entrance pin, warm chain).
3. **One real route fix — the burn is the held fire's.** walkreal-b predates fix-gadgets2: its
   burn loop drove no trigger, and master's scene.rs gates the beam on the held fire
   (`trigger && runs_beam(...)` [M the held-fire refire loop 0x1a5460:601-694 →
   Player_MuzzleFlash 0x1a4298 → Draw_LaserBeam 0x1a4560; Lock_Update 0x1ae3d8:194 burns on the
   beam's contact]). The walker gained `tick_fire(input)` (additive; `tick` unchanged) and
   tower2's burn loop uses it. Shown failing first: both 12 and 13 died at
   `tower2.rs:47 "the beam never burned the keypad lock"` before the change; 12 passes end to
   end after it. This is the same model the 0c tests already use (0fcf394 "holds fire while
   aiming").

## Shared-file hooks (all additive, listed per the task)

- `crates/nf-game/src/nis.rs`: `Player.live_matrix: Mat4` (own line, ctor own lines) + the
  stream-0 keying block in `Player::update` [M Script_Interp 0x1d9a08:106-133 — obj+0x90 keys
  from stream 0 every frame; `SP_Activate` 0x1dc338 reads the live object]. Needed by
  `walkreal_07000009_live`'s SP-position assertions (the elevator SP's live matrix).
- `crates/nf-game/src/drone.rs`: the fusebox SP ctor initializes `live_matrix` (same fix as
  walkreal-b's own merge commit aba1fdb). No other drone.rs change — **walkreal-a's
  nearest-door stand-in is NOT carried**; `fix_mission_live` passes on master unchanged.
- `crates/nf-game/src/useobj.rs`: test-ctor `live_matrix` initializer only. The game-side
  `sp.matrix` → `sp.live_matrix` read switch is NOT applied — no brought test needs it; it
  stays with the useobj owner (b carried it for the 09 elevator E; the 09 tests pass on
  master's read with today's routes).
- NOT touched: `scene.rs`, `player.rs` (walkreal-a's floor-ray head-start not needed by any
  brought test on today's master), `lib.rs`.

## Verification

- Full suite, all four crates: `NF_ISO=… cargo test --offline --release -j2 -p nf-game
  -p nf-formats -p nf-audio -p nf-view --no-fail-fast -- --test-threads=1` →
  **589 passed / 3 failed / 13 ignored**; the 3 failures are the allowed slim-ISO
  `disc_golden` tests (bigf header/tables + directory tree manifests).
- Every walkreal test: 12 **passes** (the burn fix + the two-latch fix, end to end); 09, 0b,
  0c, 0d, 11 pass (2+1+1+1+1 tests); the four documented gaps below are `#[ignore]`ed with the
  gap, the evidence and the owner named — none red.
- The 01 sister (`the_plate_refuses_bond_alone`) passes; only the escort walk is ignored.

## The four gap-ignores (each with its evidence)

1. **walkreal_07000001** — the contact escort's interior doors need the NPC door-open path
   (`NDrone2_OpenDoor`): the route was walked against walkreal-a's nearest-door-within-2.0
   stand-in, which master rightly dropped (it breaks Mayhew's walk in `fix_mission_live`).
   Symptom: the lead stalls with "no guard to clear" (round 10, state 0x1d, armed_peer 21.8).
   Owner: **045g-fix-npc-doors2** (branch `s/fix-npc-doors2` in flight). When it lands, drop
   the ignore.
2. **walkreal_0700000a** — the cab mouth's SS lift doors (placements 524/525, channel param
   0x00 [D]) never open: `SsList::tick` targets key 0 when the channel is 0, and the leaves
   are solid since fix-party-mission item 7 (Ss.hash → set_dynamic). Bond pins at z 12.4
   leaving the cab. The game's own start snapshot holds them closed too [L
   ee_start_0700000a.bin: (−2.973,1.303,13.031) @0xE2E2A4, (−4.893,1.275,13.031) @0xE2E314] —
   the port's init is faithful; the game opens them during the arrival via a writer the port
   lacks (a script/trigger, not the SS channel — the param is 0).
3. **walkreal_07000013** — the front entrance's mouth is glazed shut for the capsule: every
   walked lane (x −6.2, and the slid west x −8.09) pins at z 64.47 against a south-facing wall
   at z ≈ 65.0 — the security gate; its raise has no writer the port runs. Cold segment rays at
   y −368.4 pass under the sill (a probe artifact — only the capsule-height band blocks).
   Same regression walkreal-b's addendum documents.
4. **walkreal_07000014** — the vent cut-scene (SP #276, 0x06000833) never writes ch 0x47 in
   the window: walkreal-b's parked item ("the next session debugs the vent cut-scene"). The
   route past it is written and continues from the branch once the scene fires.

## Live checks for the main agent

1. **0a's lift doors**: run the real 09→0a intro in PCSX2; `gdbwatch.py` on
   `switch_channels` (0x26fc90, write access) and on a leaf object's `obj+0x90` (the matrix)
   during the arrival. Expected: a write turns a channel on and/or the leaf matrix moves to
   key 4 (x −2.102 / −5.741) before Bond steps out — the writing instruction names the opener
   the port is missing.
2. **13's entrance gate**: in PCSX2 walk the centre lane at x ≈ −6.2 after the objectives;
   `gdbwatch.py` the player z (obj+0x30..0x38) while crossing z 65.0. Expected: the game
   passes — record which channel/SP/scene lifts the gate (the success touches' ch 0x64? a
   cut-scene?). That decides the owner (fix-cutscenes2 candidate).
3. **14's vent scene**: confirm touch #301's firing in the game (the y band 2.07…4.14 vs
   Bond's feet on the store-room floor) and that SP #276 starts on it (walkreal-b's existing
   check, still open).

## --bond spots for the render (the door fix is visible)

- `--bond 9.3,1.1,-0.6,3.5,0` (07000012): the keypad wall with the two lock boxes; after the
  burn the beam should touch the nearer box.
- `--bond 6.4,1.03,-1.5,-1.57,0` (07000012): door[19]'s OPEN leaf lying along the wall west of
  the doorway (the two-latch fix opens it on the burn's edge).

## What's left

- The four gaps above (each with an owner and a live check); the 12 doorway crossing and the
  12 shaft lobby remain the in-test documented stand-ins tower2.rs carries (unchanged from b).
- When 045g-fix-npc-doors2 lands: drop 01's ignore and re-run.
- The probes (`walkreal_probe0a.rs`, `walkreal_probe13.rs`) are committed as the blockers'
  evidence trail; they pass (they only dump).
