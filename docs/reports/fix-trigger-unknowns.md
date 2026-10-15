# fix-trigger-unknowns (110f) — U-032 settled: the spawner pulse, the TouchOnce that never latched, the vision pause

**Status: done (2 of 3 sub-questions answered; the msg-0x1B sender narrowed but still open).**
The live snapshot's state — spawner 268 started while channels 0x22–0x25 and 0x44 all read 0 —
is reproduced end-to-end on the real input path and is **the steady state, not an anomaly**: no
reload needed. The port needed one behaviour added (the 15-frame sight pause); its trigger model
was already faithful and is now pinned by three tests.

## The disc facts (07000005 main map [D])

- Spawner **268** (type 0xF1, at 35.16, 0, 114.85): mode 2, group 2, **start ch 0x22** (p4=34),
  no stop/done, **kill ch 0x43** (p7=67), 1 wave.
- TouchOnce **305** → out **0x22**, mask 8 (player-only); TouchOnce **306** → out **0x44**, mask
  8; TouchOnces **21/22/23** → outs **0x23/0x24/0x25** (the sibling gate patches; 21 is the
  gatehouse courtyard the level-end route walks).
- MultiplexOrIn **25** → **out 0x22, inputs [0x23, 0x24, 0x25]**.
- **305 and 306 share the IDENTICAL volume** — both build to centre (6.22, 5.43, 65.41) [D].

## The answers (decompile, cited)

1. **Spawner start = a one-tick poll, latched.** `DroneSpawner_Control` 0x137e68:53-64 polls
   `switch_channels[tag+0xE]` every tick while the gate byte (+0x1E) and the kill latch (+0x20)
   are clear; one ON tick suffices (no edge/history), then it zeroes its OWN copy of the channel
   id (+0xE := 0) and latches +0x1E = 1. Starting never writes `switch_channels`. (A spawner with
   start channel 0 starts immediately at `DroneSpawner_Init` 0x137aa0:72-74 — not 268's case.)
2. **0x22 is a pure pulse — the OR gate is its clearer.** `Trigger_Update` 0x1b1a78 case 0xc
   writes `switch_channels[out] = (OR of the inputs)` — a PLAIN assignment, 0 included — every
   frame. OR 25 therefore clobbers TouchOnce 305's latch the moment no sibling patch (0x23/24/25)
   is on. A type-1 output alone has no clearer (cases 1/2 only `|=`, :74-88), but 0x22 has a
   second writer. (Type-2 Touch outputs are pulses by design: `Trigger_Calc` 0x1b1900:28-42
   pre-clears them each frame, case 2 OR-sets while touched.)
3. **The snapshot is the steady state.** The control list is prepend-order (newest first,
   `control_add_object_to_list` 0x134698), so within one frame: #305 (TouchOnce) writes 0x22=1 →
   spawner #268 (its poll sits between #305 and #25) reads 1 and latches → #25 (OR) clobbers
   0x22 back to 0. Frame-end state: spawner started, every channel 0 — **exactly the snapshot**.
   The spawner's +0x1E latch persists; ch 0x43 (its kill channel) is NOT set in the early session
   [probed — the kill latch is not the explanation].
4. **0x44 = 0 ⇒ 306 never fired.** A type-1 output on a channel NO other writer touches (the
   writer census in the worklog) cannot return to 0. 305 and 306 share the identical volume, so
   the live game delivered the crossing contact to 305's object but not 306's — the row's own
   floor-contact/`Collide_Update` hypothesis: the pick list sorts by distance with cel hits first
   (fake distance −1000+n·0.1, `Collide_Pick` 0x1eb320:350), `QuickSort` ascending (0x1ea398:141,
   comparator 0x1d33c0), and the delivery loop **aborts at the first hit without a continue flag**
   (0x1ea398:216-229) — farther contacts (the second same-spot object) are dropped. The port's
   toucher model delivers to both (its documented stand-in, trigger-port-check #11) — a live
   check below settles which object wins.
5. **The vision pause (third sub-question) — answered and ported.**
   `DroneVision_ProcessDroneSight` 0x176a28:68-69 runs sight — and `DroneVision_FindAlertedDrones`
   — only once `DAT_00298fb8 + 15 <= now && DAT_00298fbc + 15 <= now`. fb8 is stamped by
   `Drone_PostLoad_Init` 0x136938:46 at every level load; fbc by `Drone_EnableAll(1)` 0x1383e8:25
   (EnableAll(0) does not stamp). 15 flat ticks (no FRAME_RATE_MUL in the disasm). Hearing is NOT
   gated; `NDrone2_PlaySFX` 0x141848:74 / `NDrone2_SpeechSFX` 0x141b00:23 additionally mute drone
   speech for ~30 frames after load.
6. **msg 0x1B (still open).** The RECEIVER is verified: `NDrone2_DSTATE_Patrol` 0x15c498:94-98
   case 0x1b → `NDrone2_ObstructedPatrolPath` 0x14ad50 (= `return 0x61`) → state 0x61
   `NDrone2_DSTATE_Obstructed` 0x173390 (entry: CallAnim(0,0x21), +0xcc4 timer, route invalidation);
   the same arm in 11 navigation states. No literal 0x1B send site exists in ACTION.ELF (every
   literal sent id enumerated — 24 ids, none 0x1B); the anim-end path (`drone+0x598` →
   `DroneAnim_SetEndAIState` 0x13dd00 `Drone_SM_SendMsgSelf`) or a computed id remains.

## The port change

- `crates/nf-game/src/drone.rs`: `Drones.sight_pause_until` (own lines, merge lesson 4) —
  `frame + 15` at construction (the fb8 stamp) and re-stamped at every `DroneEnable(1)` (the fbc
  stamp, in the event-5/7 handler); the sight + FindAlertedDrones block is gated on
  `frame >= sight_pause_until`, the 0x97/hearing passes after it are NOT (mirroring 0x176a28:68-70
  vs 0x139520:172+). Unit sister `the_15_frame_sight_pause_is_stamped_at_load`.
- `crates/nf-game/src/triggers.rs`: **untouched** — the calc pre-clear, the case-0xc plain
  assignment, the TouchOnce latch and the spawner's channel consumption were already faithful;
  the new tests pin them.
- `crates/nf-view/tests/fix_trigger_unknowns_live.rs` (new, 3 tests):
  1. `the_gate_spawner_starts_on_the_0x22_pulse` — Bond crosses patch 305; the spawner latches
     (the mid-frame pulse's only observable); at frame end 0x22–0x25 all read 0 (the snapshot);
     0x44 latches too (the port fires both same-volume TouchOnces); the latch persists.
  2. `touch_once_306_latches_0x44_only_when_touched` — the same crossing latches 0x44 (and it
     cannot clear); the courtyard route starts the spawner with 0x44 still 0.
  3. `sight_waits_15_frames_after_the_re_enable` — after 07000006's intro re-enable, no
     seen-ticks accumulate before re-enable+15 (regression guard; the fail-first for the field is
     the compile-error class — the runtime shift is below the natural wake-up latency at the
     beam spot).
- `crates/nf-view/examples/probe_trigger_unknowns.rs`, `probe_kill43.rs` — the extraction/probe
  scratch (committed as examples; no game bytes).
- `docs/UNKNOWNS.md` — U-032 updated: partly answered (msg 0x1B sender open).

## Verification

- New/affected on the final tree — **191 passed / 0 failed**: nf-game lib 167 (incl. the new
  sister), fix_trigger_unknowns_live 3, level_end_live 6, spawner_kill_live 5,
  fix_guard_first_shot_live 1 (the sight gate did not shift the calibrated reaction chain),
  level_reload_live 9.
- Base-tree full suite (fast runner, before the change): **1278 passed / 3 failed / 66 ignored
  (55 min)** — the 3 are the allowed slim-ISO `disc_golden` trio.
- Fail-first: the unit sister and the `sight_pause_until` field do not compile on the pre-fix
  tree (documented class); the trigger tests pin pre-existing correct behaviour (the task's
  "port any behaviour the port gets wrong" — there was nothing to fix in the trigger path; the
  decompile reading, not a port bug, was the deliverable).

## Shared files touched (small, additive)

- `crates/nf-game/src/drone.rs` — the `sight_pause_until` field (own lines), the gate around the
  sight block, the re-stamp in the DroneEnable arm, one unit test. No shared constructor line
  touched (the field rides its own line, merge lesson 4).
- `docs/UNKNOWNS.md` — the U-032 row.
- `docs/WORKLOG.md` — append-only entries.

## What's left

- The msg-0x1B sender (needs a live trace — see below), and the live confirmation of the
  305-vs-306 contact delivery.

## Live checks for the main agent

1. **The 305/306 contact delivery** (settles the 0x44 half): in PCSX2 on 07000005, `gdbwatch`
   `Trigger_Hit` 0x1b1fc8 (r) while Bond walks over the shared patch at (6.2, 5.4, 65.4) —
   expect the contact list to carry 305's object and NOT 306's (or 306's Trigger_Hit returning
   0), i.e. one of the two same-spot objects never delivers a contact (the pick sort/early-out).
   Also read both trigger objects' `obj+0xD0` contact lists in the same walk.
2. **The msg-0x1B sender**: breakpoint `Drone_SM_RouteMsg` 0x172128 filtering id 0x1B during a
   patrol walk in a corridor — the return address identifies the sender (expected: the anim-end
   path via `DroneAnim_SetEndAIState` 0x13dd00, or a computed id at the call site).
3. **The 15-frame sight pause**: poke `DAT_00298fbc` (0x298fbc) := the current `GameState+52`
   value mid-play — expect all drone sight to freeze for 15 frames (the HUD alert state and the
   guards' seen-tick counters stop).
4. nf-view `--bond` spots (non-visual task — the main agent may skip): the patch at
   `--bond 6.22,5.43,65.41,0,0` (both gate TouchOnces) and the courtyard
   `--bond 60.15,30,96.43,0,0` (TouchOnce 21) — the spawner-268 guards should spawn either way.

## ntfy failed

(none — notification sent at finish)
