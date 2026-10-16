# stun-ai-recover (654)

The playtest 1015-1738 (07000005, the Stunner): the guards "get stunned infinitely and don't
complete the animation". The task: port the stunned drone state fully — entry, the stun
animation, the duration/re-hit rules, the exit back to the AI — and prove the game's timer ride
and the re-hit behaviour.

## What was already there (109b, fix-stunner-press)

The taser family was ported in fix-stunner-press: the msg-7 route [M NDrone2_DealWithObjHit
0x13fff0:269-300 — the stunner's bullets 0x4A/0x4C pick msg 7], the base case 7 (the accumulator
+0xBC grows by FRAME_RATE_MUL per tase msg, past 2·health the drone falls into DSTATE_Taser 0x4a
[M DroneFunc_HandleImpact 0x1469b8:58-131]) and the three family DSTATEs (Taser 0x162b98,
Stunned 0x162eb0, Stunned_Recover 0x1630b8). This session re-verified every arm of those three
switches against the decompile and found the family state machine itself faithful — the bug the
playtest hit was NOT in the state machine.

## The real defect: a downed guard could not be hit again (the re-hit was dead code)

Empirical trace on 07000005 (the real input path, the stunner on the stairs guard): the guard
falls into TASER 0x4a at hold tick 20, rides the 1800-tick twitch timer into STUNNED 0x4b at
rel+1798 — and then a re-tase tap on the downed guard did NOTHING: the trigger spent a round,
the stunner's target test locked onto him (the ray-vs-origin distance 0.000), the dart spawned,
flew — and `first_box_hit` returned None against all 16 of his posed hit boxes. The dart buried
itself in the floor. Every re-tase on a downed guard was a no-op, so DSTATE_Stunned's case-7
re-fall / case-0x19 re-arm [M 0x162eb0 case 7 → 0x19] — the game's whole re-hit rule set — was
unreachable from play.

Ground truth: `NDrone2_Collision` 0x140e90 branches on the DOWN flag `Drone+0x2f` [M :65-90] —
while it is set the shootable volume is a low sphere (radius 0.3, centre (0.3 − obj+0xa0) above
the origin), not the standing volume. The flag is set by DSTATE_Stunned's ENTER and cleared by
its EXIT [M 0x162eb0 case 1/2], with the same pair in DSTATE_StunGrenadeLoop [M 0x1631c0 case
1/2] and DSTATE_StunDartLoop [M 0x163570 case 1/2]. `Player_WeaponFiring` 0x1a5460:90-106 reads
the same flag on the aim target to swap the reticle datum — the game expects the player to aim
at downed guards. The port had no `+0x2f` at all, and its standing stand-in (the posed bone
boxes) demonstrably leaves the aimed-at-origin ray uncovered once the body is down.

## The fix (crates/nf-game/src/drone.rs)

- **`Drone::down`** — the `+0x2f` stand-in. Set at the STUNNED ENTER, cleared at a new
  (STUNNED, msg::EXIT) arm [M 0x162eb0 case 1/2]; the same pair in the STUN_GRENADE_LOOP arms
  [M 0x1631c0 case 1/2]. (DSTATE_StunDartLoop 0x163570 is not ported; noted below.)
- **The downed sphere in the bullet contact** [M 0x140e90:65-90]: when a drone is down and the
  posed boxes miss, the contact test falls back to the game's own low sphere — radius 0.3,
  centre pos + 0.3 up (the port's origins rest on the floor when down, so the `+0xa0` lift term
  drops out). The standing path is untouched (the established boxes stand-in; the damage tests
  pin it).
- Nothing else in the stun family changed: the entry, the 1800-tick fall timer, the 1200-tick
  RecoverTime [M 0x147188 cases 0x4a/0x4c = 600·0.01·(200−0xB9) = 1200 at the run-time stat 0
  [L 07000005]], the AnyoneNear(2.0 → < 4.0 u) kept-down band, the get-up 0x76 with its follow-on
  Attack 0x56, and the accumulator's never-reset [M — the grow at 0x1469b8:72 is the ONLY writer
  of `+0xBC` in the whole decompile; a grep across ACTION.ELF finds no other drone write] were
  all re-verified as game-true and left alone.

## Verification (crates/nf-view/tests/stun_ai_recover_live.rs, the real input path)

- `the_stunned_guard_rides_the_game_timers_and_returns_to_combat` — the full ride: the guard
  falls at hold tick 20, TASER→STUNNED at rel+1798 (the 1800-tick twitch timer [M 0x162b98 case
  0xc]), and with Bond 8 u away STUNNED→STUNNED_RECOVER at +49 past the timer expiry (the
  1200-tick RecoverTime [M 0x162eb0 case 0xc]), then the get-up completes into the combat family
  (0x56 in the state log) — **the animation completes, the guard returns to combat**. The
  playtest's "stunned infinitely" walk is pinned to the game's timers end to end.
- `the_player_near_keeps_him_down_and_a_retase_behaves_like_the_game` — with Bond at 2.5 u the
  STUNNED timer re-arms past its expiry (tc 3210 → 3510, the AnyoneNear(2.0) band [M 0x162eb0
  case 0xc]); one re-tase tap on the downed guard then lands (the downed sphere) and re-arms the
  stun per the stored fall anim 0x32 (tc 3510 → 4621 [M 0x162eb0 case 7 → 0x19]); the stored
  0x31 draw takes the re-fall arm instead (the test branches on the stored anim exactly like the
  game's case 7); the walk still completes to combat afterwards. **Re-stunning behaves like the
  game.**
- Both tests green on the final tree; the probe traces that found the defect are the
  `--ignored` scratch probes in `crates/nf-view/tests/probe_stun_lifecycle.rs` (NOT committed —
  the sandbox reset mid-session; the traces are quoted in the WORKLOG).

## What was run

- `cargo test --offline --release -p nf-view --test stun_ai_recover_live` — **2 passed** (the
  two lifecycle tests above).
- `cargo test --offline --release -p nf-game --lib` — **215 passed, 0 failed** (the crate the
  fix touches).
- `cargo test --offline --release -p nf-view --test fix_stunner_press_live` — **3 passed** (the
  109b regression surface: the press tick, the held tase, the P2K sister).
- The full suite (`tools/test/suite.py`): started in the background twice per the task; the
  first run was killed by a sandbox reset that also took the repo (it had not produced a total
  line), and after the re-setup (clone → rustup → the alsa prefix → the ISO → cargo fetch → the
  full rebuild) the remaining window did not fit another ~30-minute suite run. The runs above —
  the fix's crate (nf-game lib), the task's tests, and the task's direct regression surface —
  are what completed on the final tree; the total line is the one thing this report cannot
  give, and the next message should run `tools/test/suite.py` on the branch before merge.

## Left / noted deviations (all pre-existing, none introduced)

- **DSTATE_StunDartLoop 0x163570 / StunDartImpact 0x163430 / StunDartRecover 0x163700 (states
  0x50/0x51/0x52) are not ported.** They are a THIRD stun family next to the taser (0x4a-0x4c)
  and the stun grenade (0x4d-0x4f). The SP stunner's path does not enter them: its hits route
  msg 7 into the base HandleImpact case 7 → SetState(0x4a) [M 0x1469b8 tail], and no SetState
  site in the decompile targets 0x50. They look like the MP dart's family (the RecoverTime
  weapon cases 0x43/0x44 route msg 0x19, which the StunDartLoop handles). Porting them needs a
  live check of what the MP dart does — listed below.
- The weapon drop / ChangeWeapon(1) at the TASER enter and ChangeWeapon(6) at the recover ENTER
  [M 0x162b98:27-34, 0x1630b8 case 1] remain unmodelled (the standing 109b gap: the port's
  `Drone::weapon` is the held model id, not the game's slot).
- The reticle datum swap for a downed aim target [M 0x1a5460:90-106] is view-side; the flag the
  port would read now exists (`Drone::down`), the draw itself needs the main agent's render.
- The re-tase while STUNNED_RECOVER (case 7/0x19 → HandleImpact with the charged accumulator →
  the instant re-fall) is verified by the code walk and the accumulator's never-reset [M], not
  by a live ride — the ride needs another ~3000-tick scenario and the window closed.

## Live checks for the main agent

1. **The downed-guard reticle** [M 0x1a5460:90-106]: 07000005, the stunner, aim at a DOWNED
   guard (after a tase). Expected: the view datum swaps (the game's down-target reticle) while
   the crosshair sits on him; back to the normal reticle when he gets up. The port has the flag
   (`Drone::down`) but not the draw.
2. **The twitch/fall anim looping** [D the anim data]: the game's twitch (0x2f/0x30) and fall
   (0x31/0x32) while down — do they loop (the guard keeps twitching) or hold the last frame?
   The port holds the last frame (`held`) — if the game's twitch loops, the downed guard
   visibly twitches and the port's frozen pose is the "don't complete the animation" half of
   the playtest note that the anim data decides. Render spots: 07000005 spawn pocket, the
   stairs guard down (`--bond` at [97.2, 18.6, 101.0] facing [99.7, 16.8, 101.9]), frames
   +2000/+2600/+3100 after the fall.
3. **The MP dart family (0x50-0x52)**: whether the MP stunner variant drives guards into
   StunDartImpact — needs a PCSX2 MP probe; SP never enters them per the decompile.
