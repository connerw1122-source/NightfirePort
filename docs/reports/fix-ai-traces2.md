# fix-ai-traces2 — report

Branch `s/fix-ai-traces2` (the task card's bottom said `s/ai-traces2`; the task header and the
reply-line format say `s/fix-ai-traces2` — that is the branch pushed here). The task: land
fix-ai-traces' PlayScript work (harness + the state-3 port, cherry-picked as 14efbfa/71b9e99/2a16d85)
on master green — the merge had gone **677 passed / 6 failed**.

## The six failures, each decided from the decompile + the traces

1. **`fix_drone3_live::the_truck_driver_reaches_his_idle_state_and_never_twists`** — the driver's test
   reading was wrong. The decompile: `NDrone2_DSTATE_Global` 0x15b830 msg 1 routes **dtypenew 0x17 →
   SetState(0x32) TruckDriverInit BEFORE its start-script branch** (WaitSwitch 0x15b960 case 3, his
   channel firing, does the same); `NDrone2_DSTATE_TruckDriverInit` 0x160bd0 case 1 then plays the
   start script **from TruckDriverInit and holds the state** (SetScript(`+0x554`, 0x24, 0x11a, 0x1e,
   0, 5, 0), `+0x554 = 0`), case 5 → 0x34 (0x35 only when the AI-path counter `+0x9f2` is set —
   InitAIPath 0x153720's node cursor, not ported). [L] 05-truck's 07000006 segment: the driver (drone
   13) is in state 0x32 for the whole 700-frame window. The port got the Global arm (mode 0x17 →
   TRUCK_DRIVER_INIT), the ENTER script branch, and the ANIM_DONE → idle arm. The driver's param 1 is
   a real script, 0600011d (880 frames) — the "script-less branch" the test cited never applied.
   Test updated to the evidence: 0x32 at tick 2, 0x34 after the script (920 ticks), then the scare.

2. **`fix_npcs_live::an_idle_gate_guard_sees_and_attacks_bond_on_the_same_floor`** — d15's 1244-frame
   start script ended into **ATTACK** because the port had mapped `+0xCD8` (the *impact* resume,
   cases 6/7/8/9 → `HandleImpact(+0xCD8)`) onto the script's natural end. `NDrone2_DSTATE_PlayScript`
   0x15ba68 case 5 (msg 5 = ANIM_DONE) is a different thing: `+0x554 = 0` then — default /
   0x60005e6/0x60008ae → `SetState(+0x5A2)` (the START state); 0x6000168/0x6000169 →
   `CallAnim(0, 10, 0, 0x68, 0, 0)` + AttackTalk with the state HELD; 0x60007ac → `SetDAnimNow(0xf4)`
   + the start state. Ported via `play_script(…, next_ai = 0, done_msg = ANIM_DONE)` (set_state(0) is
   a no-op, so msg 5 lands in the state's own arm) and a new `(PLAY_SCRIPT, msg::ANIM_DONE)` arm.
   d15 now settles into IDLE and the test passes unchanged.

3–5. **`fix_objects_live::a_death_draws_its_drop…`, `fix_player3_live::death_drops_match…`,
   `render2_live::dropped_guns_dim…`** — all three "the guard died". The 07000001 first guard's start
   script 06000169 (181 frames) **walks him 6.8 units** [D nf-extract script-info: root 0.05 → 6.87],
   and the game's own recording shows him moving inside the script [L 01-doors: x −4.919 → −6.03
   within the first 120 lines, state still 3]. The tests aimed ONCE at his pre-script position, idled
   10 ticks, then fired at the stale line — the guard had walked out of it. The tests now track him
   (re-aim at his live pos each tick, the player's own behavior); every assertion after the kill is
   unchanged. (A probe confirmed a live-aim shot kills him at t20 with the drop.)

6. **`gpu_palette_matches_the_cpu_pose_on_07000005_06_07`** (anim.rs:70 index 22 of len 22) — the
   start scripts are TWO-window (a 222-channel body + a 22-channel face track, e.g. 0600011b →
   04000020/21), and `Drone::local_pose`'s `script_pose` picked the LAST covering window (fix-drone2's
   last-wins) → the 22-channel face track full-posed on a 222-channel skeleton. The game starts every
   opcode-0 window as its own playing anim (`AnimProcessScriptCmds` 0x1c1b38) and `AnimFrameCopy`
   0x1c5668 writes only the tracks each carries. New `nf_game::anim::script_pose_drone`: every
   started window plays, **per bone the LAST-started window that covers it owns it** — the direction
   is settled by [L] evidence, not the old [S]: the 07000006 driver holds position to 0.01 over 700
   frames while his body track's root runs 0.57, because his face track (started last, all-zero root)
   owns his root bone; a single-window script (the guard's 06000169) keeps its root motion. The widest
   window fills the bones no window reaches; a shared human track riding another rig's record byte
   (0x0400028a, drone-port-check #3) counts as full-coverage when its channels cover the skeleton.
   `Drone::script_frame` (the root source) is last-covering-window the same way.
   Belt-and-suspenders: `Sequence::value_with` (nf-formats/src/anim.rs:68) returns 0.0 for a channel
   past the sequence's list — the neutral track — instead of panicking (the bounds fix the task named).

## Not ported (evidence-gated)

- PlayScript's impact resume (`+0xCD8` per variant) and the ENTER-side AttackTalk/`0x400000` flag
  side effects — no test exercises them; noted in the code comment.
- The UPDATE arm's `NDrone2_SeenAndAttacking` on `alert >= 1.0` — fix-ai-traces left it out pending
  live check 2 (the game's guard alert never reaches 1.0 in the PlayScript window); no new [L]
  evidence, so it stays out (the d15 attack comes from IDLE, unaffected).

## Merged tree, suite counts

Merged origin/master (fix-impacts + research-driving; WORKLOG kept both appends in order) and ran the
full suite `--no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`:

- **nf-game + nf-formats + nf-audio: 284 passed / 3 failed** — the 3 allowed `disc_golden`.
- **nf-view part 1 (66 binaries): 272 passed / 0 failed.**
- **nf-view part 2 (66 binaries): 137 passed / 1 failed** — `walkreal_07000001_live::
  walk_from_the_spawn_to_the_exit_with_the_contact` (below).
- **Total: 693 passed / 4 failed** (the 3 allowed + the contact walk).

Two merge-wave items, neither caused by this task's code (both verified: `fd4ec1b` — this branch
before the master merge — passes them):

1. `ai_05_truck`'s drone-13 pin moved 7049 → **5508**: fix-impacts' effect draws (casings/debris on
   every user shot) joined the shared RNG stream and the guard's idle-timeout walk starts seconds
   earlier (state 4 still matching, dmax 3.72). Re-picked with the reason in the test — master's own
   precedent is ad5fb7f (the ai_live seed re-pick for the same join).
2. The **contact walk** stalls: the contact parks in PLAYER_IN_WAY (0x1e — `route_blocked && dist <=
   3.0` [M 0x15e4f0's arm]; he resumes when Bond is out of the way band, dist > 3.0 [M 0x15ea28 case
   3]) and the old recovery ("close on the contact") walked Bond INTO his lane, forever. The recovery
   now steps Bond BACK off the lane (away from the plate) when the contact is in PlayerInWay
   [b2afee4]. **Unverified on the merged tree**: with the recovery the walk runs past the sandbox's
   10-minute tool cap (the pre-merge tree passes in 26 s; the merged-tree RNG shift makes the escort
   longer). The edit is mechanism-cited, but the next session must confirm it green — it is the only
   open red.

## Files touched

`crates/nf-game/src/drone.rs` (Global's 0x17 arm, TruckDriverInit's script branch + case 5,
PlayScript's ENTER/ANIM_DONE arms, script_frame last-window, local_pose → script_pose_drone — all
additive arms, fields unchanged); `crates/nf-game/src/anim.rs` (new `script_pose_drone`); 
`crates/nf-formats/src/anim.rs` (the bounds fix only); the harness pin; the 4 test files named above
(only where the evidence said so) + `walkreal_07000001_live.rs` (the PlayerInWay recovery).

## What's left

1. Verify the contact walk green on the merged tree (the step-back recovery; if the stall persists,
   trace the lead's `route_blocked` set point against the shifted RNG).
2. The PlayScript impact resume + the SeenAndAttacking arm (live check 2) when evidence arrives.
3. The harness's warm-up still aligns on state only — the PlayScript drones' script PHASE is not
   aligned (fix-ai-traces' report #4, unchanged).
