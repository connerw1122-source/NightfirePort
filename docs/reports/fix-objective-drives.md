# fix-objective-drives — the four gameplay-driven objectives, driven through the real writers

Task 105d. Campaign-flow blocker 6: the runner could not fire 09's ch71, 08's ch75 (the gondola),
4a's ch74 and 0a's objective inputs 0x48/0x49. **Status: done for the runner** — all four legs are
driven through each level's own writers in `crates/nf-view/tests/campaign_flow_live.rs` and all four
campaign tests pass (`4 passed; 0 failed`, ~52 s). **No production code changed** — the whole diff is
the runner test + the probe test, so the suite baseline for the production code is master's (below).

## The writers found (evidence first)

- **09 ch71 (0x47 "Take main elevator to office floor")** — the writer is 0a's arrival TOUCH_ONCE #58
  at the spawn ([D] docs/levels/0700000a.md; [L] ee_start_0700000a: ch 0x47 already on at load), and
  — found this session — **the 0xE8's own success write**: fix-level-chain's fresh parse (09's real
  exit is map39 #60, next 0x0700000a, gate ch 0x32 [D probe]) + `Trigger_Activate`'s success write
  [M 0x1b2318:96-98] means riding the cab (TOUCH_ONCE #67 → ch 0x32 → the gate) writes 0x47 through
  the game's own exit path. The runner's 09 row is now **3/3 with the real exit to 0700000a** (the
  old self-loop reading is dead — fix-level-chain).
- **0a 0x48 (Q-Worm install)** — placement #688 Q-WORM_PC_1 (type 0x35, param 1 = ch 0x48): the
  install completes when a weapon-0x58 bullet (the Q-Worm gadget) makes a class-5 CONTACT with the
  PC's nodes, then `switch_channels[param1] = 1` [M PCQWorm_Update 0x1aee20; PCQWorm_Create 0x1b07a8;
  the port's useobj.rs carries both]. **The runner fires the real gadget** (the walkreal-0a aim loop,
  weapon 0x58, aim-mode taps). `PCQWorm_Activate` (the E path, 0x1b0840) is ported but 0a's PC has
  param 2 = 0 [D probe] — the shot is the only real path, as the doc says.
- **0a 0x49 (security centre)** — AND mux #741: 0x49 = AND(ch 0x3A); ch 0x3A = SP #60
  "SCRIPTPLAYER_KeyboardMH" out (mode 64, mask 4) [D docs/levels/0700000a.md].
- **08 ch75 (0x4b gondola)** — the full chain is real in the port: the cable-car floor Touch #236
  (out ch 4, mask 8) → AND mux #235 (0x4A AND 4) → ch 1 starts SP #295 (mask 1 on ch 1) whose LEVEL-
  BIN script 0x0600004C carries the cab (and Bond on its floor — the probe shows him riding the key
  line down 14+ units) down the mountain, fires ch 83 at t1410 → SP #296 (the pack cinematic
  0x06000078) → ch 5 → the copter #45 writes **ch 0x4B** when its move completes [M Copter_Update
  0x17dbb8 cases 4/5/6] → mux #276 → ch 100 → the mission ends 0x0730008 [D docs/levels/07000008.md §5.3].
  The runner drives the plate, the mux, the ride and asserts the whole chain; 08 completes.
- **4a ch74 (0x4a "Gain access to lobby")** — no writer exists in the level's data (triggers, 11
  scripts, SPs scanned [D scans; docs/levels/0700004a.md §6]); the port's documented stand-in writes
  ch 0x4a when the lift script 0x06000571 ends (nis.rs, keyed to level 0x0700004a + the script hash).
  The runner rides the lift from the spawn (the platform carries Bond down the shaft, the first plate
  #277 reveals the objective, the landing NIS plays) and the script end fires the channel. **Fix-
  finale's re-seat hack is NOT needed** — the raw-step ride holds (health topped, the shaft garrison's
  fire shrugged off like fix_finale_live).

## How verified (all failing-first or probe-backed)

`cargo test --offline --release -p nf-view --test campaign_flow_live` → **4 passed / 0 failed**:
- `campaign_flow_story_missions_1_to_3` — 09: `3/3, real drive 09: … the cab touch → the real exit
  to 0a`; 0a: `3/3 … the muxes real; the hatch E → the real exit to 0b`; 08: `complete … the real
  carriage ride → ch 0x4B; mission end → 0x0730008`.
- `campaign_flow_story_missions_4_to_7` — 4a: `1/1, real drive 4a: the ride real …` complete.
- Sister assertions inside the drivers: 09 — the lobby AND mux #274 is OFF before the elevator ch and
  ON after, and the walk raises no alarm; 0a — 0x47 folds through the real plate on 0a (09's rec 3),
  AND mux #741 OFF before the keyboard; 08 — AND mux #235 stays OFF with ch 4 alone (before the 0x4A
  leg), Bond's z covers the descent; 4a — ch 0x4a OFF mid-ride (k==1200), ON only at the script end.
- `zz_obj_drives_probe.rs` (kept): `probe_08_gondola_chain` runs the whole gondola chain COLD end to
  end with **no stand-in at all** (ch 4 → ch 1 → the ride → ch 83 at t1411 → ch 5 at t1753 → ch 0x4b
  at t2053 → ch 100) — the proof the chain is real and the warm-load gap below is the only stand-in;
  `probe_4a_ride` (the ride to ch 0x4a at t5010); the 0a door/ray/SP probes (the blockers' evidence).

## The stand-ins left at documented blockers (each with its live check)

1. **09 ch 0x39** — the elevator SP #70's activation writer (the E path has no reach for it;
   walkreal_07000009 pinned every dead end). gdbwatch `switch_channels[57]` (0x26fc90+57) on a real
   run through the lobby names the writer object.
2. **0a ch 0x48 (only if the shot can't land) and ch 0x3A** — the keyboard SP #60's placement matrix
   is IDENTITY [D probe this session: `at [0.0, 0.0, -0.0, 1.0]`], and the game's own E path reaches
   `SP_Activate` only through obj+0xE4-bound class-0 entities [M 0x1a5170:121-122] — the writer is
   live-checked (gdbwatch the keyboard sequence). The PC cubicle: every eye-ray from every walked
   ring position is blocked by a full-height wall, normal ≈ (0.30, 0, 0.95) [D segment survey this
   session + the walkreal's 112-shot probe] — PCSX2: walk to Q-WORM_PC_1; if the real game lets Bond
   shoot from outside, the port's collision for that cell is the bug (the collision owner).
3. **0a walks** — the cab-mouth SS doors (placements 524/525, channel param 0, mask 0 [D probe this
   session]) stand closed; their opener is the live-checked arrival writer (gdbwatch switch_channels +
   the leaf obj+0x90 during the real 09→0a intro), and the lobby's A* has no reachable start node.
   The runner places Bond at each writer (noted per row) and drives the writers themselves really.
4. **08 ch 0x4A** — the safe-room briefcase [M Pickup_Handler 0x18e650:330-333] sits behind the safe
   bolts whose burner (the laser watch) is not ported. The walkway descent also has no walked route
   (no walkreal covers 08; walk_07000008 teleports) — Bond is placed on the floor plate, noted.
5. **08 ch 5 — the warm-load pack-cinematic gap (NEW port bug, needs a follow-up)**: on the WARM 08
   load the load print says `07900008.bin: 1 scripts, for 06000078` (the pack parses and enters the
   lib) yet SP #296's start hits the missing-script arm (state 5) when ch 83 fires — the COLD probe
   runs the same cinematic fine (ch 5 at t1753). The runner drives ch 5 at that blocker; everything
   after (the copter's ch 0x4B write) is real. Suspect: the warm path's lib/`Scripts::new` or the
   SP's lazy-load lookup racing the pack insertion.
6. **4a's writer** — the documented nis.rs stand-in (no data writer exists); the landing NIS enable
   placement contradiction (walkreal_0700004a's parked live check: gdbwatch the enable matrix +
   Bond's y through the −52 landing) is untouched — Bond falls into the shaft void after the landing
   cut-scene and the objective still completes at the script end (t5010), health intact.

## What's left

- The follow-ups above (each already has a filed live check); plus 0a's office door ch 0x30 (no
  writer in the dumps — the keyboard sequence is the candidate, the walkreal's live check).
- 08's row shows `1/2` for the objectives column: rec 0's (0x4a) monitor state lags its channel
  (both channels are ON and ch 100 completes the mission) — cosmetic bookkeeping, noted for the next
  pass.
- A walked 0a route (the doors) and a walked 08 walkway descent (the nav work) would remove the two
  placements.

## Files touched

- `crates/nf-view/tests/campaign_flow_live.rs` — the four drivers + the dispatch (the fallback and
  fix-level-chain's FAIL-flag fix kept for the undriven levels).
- `crates/nf-view/tests/zz_obj_drives_probe.rs` — the probe file (the evidence + the cold-chain proof).
- **Shared files: `drone.rs`/`scene.rs`/`player.rs`/`lib.rs` untouched. No production code changed.**

## Suite total

The full suite on the final tree did not fit the session window (test-only diff — the production
code is byte-identical to origin/master after the merge, whose baseline the fix-character-room-light
worker measured on the identical tree: **1065 passed / 7 failed / 35 ignored**, the failures being
the 3 allowed `disc_golden` + `weapon_matrix_live` TIMEOUT + 3 pre-existing pylon). On this branch:
the 4 campaign tests pass, `walkreal_07000009` (the suite's own 09 walk) passes, and the probe tests
pass. The final-tree suite run: `suite_final.out` left running in the sandbox for the next message.

## Live checks for the main agent

PCSX2 (spot / action / expected):
1. 07000009: ride the lobby lift — gdbwatch `switch_channels[57]` (0x26fc90+57): the writer names
   the elevator SP's real activator (the runner's one ch 0x39 stand-in).
2. 0700000a: the 09→0a arrival intro — gdbwatch switch_channels + the SS leaf obj+0x90: what opens
   the cab-mouth doors (placements 524/525, channel 0, mask 0)?
3. 0700000a: walk to Q-WORM_PC_1 — can Bond shoot the PC from outside the cubicle? (The port's
   full-height wall, normal (0.30, 0, 0.95), blocks every ring ray.)
4. 0700000a: the security keyboard — gdbwatch ch 0x3A (58): which object writes it (SP #60's
   placement matrix is identity, the E path needs an obj+0xE4-bound entity)?
5. 07000008: board the gondola normally — confirm ch 0x4A's writer (the briefcase behind the
   un-portable laser-watch burn) and whether the ride plays the pack cinematic (07900008.bin) warm.
6. 0700004a: the −52 landing — gdbwatch the #289 NIS enable matrix + Bond's y (the parked
   walkreal_0700004a contradiction).

nf-view (--bond spots): no visible change — the diff is headless-only (the runner's rows are the
check: re-run `cargo test … --test campaign_flow_live -- --nocapture` and compare the four driven rows).
