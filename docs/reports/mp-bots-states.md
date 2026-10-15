# mp-bots-states — the bot state machine (multiplayer phase 4, after 401)

**Status: done for the task's four priority groups; the gaps below are named.** The states the
`BotGlobal` skeleton dispatches to are ported behind the `mp-bots` feature in
`crates/nf-game/src/mp/bots.rs` §6 (`BotBrain` + `BotVars` + `BotCtx`), with 5 unit tests in the
same file (`mod brain_tests`) and 2 NF_ISO tests in `crates/nf-view/tests/mp_bots_live.rs` that
run one bot on Arena 1 (0x07000023) through the real load path. The SP sim is untouched — every
new item is `#[cfg(any(test, feature = "mp-bots"))]` and nothing outside the cfg calls it
(nf-game lib 175/175 with the feature off; byte-identical SP behavior by construction).

## What's ported (per group, with addresses)

1. **The global state (0xC5 `BotGlobal` 0x16E060)** — the msg-3 heartbeat as the driver's
   forced-to-0xC5 delivery (`Drone_SM_RouteMsgDCV` 0x171df8:78-86 sends ControlSTANDARD's
   msg 3 [0x1490f0:81] to 0xC5 first for bots), then the ported arms: the +0x72C requested-state
   step [M :121-130], the HP-regen arm [M :209-233], the type dispatch with the type-5/9
   `setNearestNavNode` + `processGoals` path and the 5 s sight-latch → `ReactToOpponentSighted`
   [M :498-513]. Corrections to the 401 skeleton encoded here: msg[1] is never read (the live SM
   state is), the ignore block returns 1 / default returns 0, msg 3 is the heartbeat (the death
   branch lives inside it), 0x42 also SetStates 0xDE, 0x3A has a type-4 early-out.
2. **Move-to-goal / roam** — 0xF9 `BotIdle` 0x171b80 (the grudge write [M :55], the zero-prefs
   hunt pick [M :57-59], the latch/mode gate verbatim [M :66-72], the guard-site arm) and 0xEB
   `BotGotoGoalPosition` 0x171240 (the +0x48 route code write [M :40], the visit stamp [M :43-45],
   the 0xF6 door detour with the `Drone+0x5A6` resume [M :60-63], the `+0x72C` resume read
   [M :31-35]). **There is no SetState(0xE9) anywhere in the decompile** — `BotStuck` 0x175220 is
   an orphan stub whose only arm returns to 0xF9; 0xEB's exits are 0xF9 and 0xF6.
3. **The weapon pickup visit** — `pickGoal` 0x126b88's flow: the deathmatch fall-through to slot
   0 [M :317-319], the grudge hunt [M :327-342], the 4-pass pickup scan (the zero-weight → 1.0
   default [M :353-356], the pass-2 equal weighting [M :375-379], the class filters [M :431-477],
   the 5.0/flag-2 skip [M :420-421], the score = ((u8)+0x47+1)−dist ×weight with the dist
   tiebreak [M :449-453]) and the **MP_PICKUP+0x80 visit floats**: skip until pass 3, then
   ×max(|now−visit|·50, 1) with the ×4500 future-stamp blacklist [M :407-415]; the stamp is
   `setPickupVisitTime` 0x129958's now+45.0 (+0.1 if 0) [M :10-15]. Plus `initGoal` 0x1294c0,
   `uninitGoal` 0x129438, `setGoalComplete` 0x129510, `getActiveGoal` 0x129548,
   `setGoalPickPrefs` 0x128298 (the (1−hp/max)·25 auto-derive + the 25.0 clamps + the −2 dist
   store), `gotoGoal` 0x1278a8 (the deadline +0x24/+0x28 := now/`VIDEO_FRAME_RATE·5` = 300.0 [M
   :50,58], the subtype wander rolls 600/600/500 [M :95,109,119], the teardown), and
   `processGoals` 0x127c78 (the deadline expiry [M :70], the type-3 validity, the 20.25/|Δz|<2
   melee gate [M :109-113], the arrival via route code 3 / the complete flag [M :165-205]).
4. **The attack state** — 0xCF `BotAttack` 0x16f198 (the ENTER `Rand_Rand(3)` →
   `chooseCombatMove` [M :22-28], the unseen<16 && dist<engage shortcut [M :46-50], the route
   block, the sighted latch [M :85-88]) and 0xD2 `BotAttackFire` 0x16f5e8 (the ENTER trigger pull
   → `DroneWeap_Fire` 0x17a8c0 → `NextBulletTime` 0x179d40: the interval min-60 for ids
   6/0xA/0xE..0x10, the burst rolls `Rand_Rand(3)+2` / `Rand_Rand(2)+1`, the ≥4 ×aggression min-3
   scaling [M :26-134]; the latch routing verbatim [M :42-52]; the route/vision block [M :56-93]),
   plus the per-tick firing pipeline (`ControlSTANDARD`:231): the unseen<16 LOS gate, the
   ammo gate (`AmmoInGun` = +0x144+w·0xC), the 0.9998 aim-cone [M 0x178c48]. Target choice:
   `getPreferredTraitOpponentObjIndex` 0x128b50's trait switch (2 same-team min-dist, 4
   top-score, 5 keep-current/nearest, 8 lowest-health) + `setOtherPlayerInfo` 0x125390's
   candidate/same-team/dist² pass + a minimal `FindOpponent` (nearest seen candidate — [S]).

**For each state the requested inventory** (address, fields, transitions, RNG) is in the code
comments with [M] line cites; the RNG call sites and orders are pinned by
`pick_goal_rng_order_is_deterministic` (a pickup pick consumes no RNG — subtype 0 skips
gotoGoal's rolls) and the burst-schedule assertions.

## Verification

- `nf-game` lib (feature off): 175/175 — SP untouched. With the feature on: the 401 golden tests
  + the 5 new `brain_tests` pass (the round trip, the stand sister, the fire decision with the
  decompile's timing bounds, the visit decay, the RNG order).
- `mp_bots_live` 2/2 on Arena 1 (NF_ISO, real 0x25/0xF0 placements through
  `register_spawn_point`/`get_spawn_point`): the bot leaves its spawn, reaches a pickup (the
  visit float = arrival+45.0, the bot ends within 2 u of it, the trace went 0xF9→0xEB→0xF9), and
  takes a shot at a stationary player (aim +x, tick ≥ the burst deadline, through 0xEC→0xCF→0xD2).
  Sisters inside: the stand preset holds position exactly; the decay/RNG sisters are in the unit
  tests.
- Priority batch on the final tree: nf-game lib 175/175, nf-view lib 8/8, mp_arenas_live 1/1,
  mp_core_live 5/5, mp_bots_live 2/2, rng_live 1/1.
- **Full 4-crate suite: NOT run end-to-end in this window** (the sandbox's background runner
  died on a PATH issue at 13:05 and the foreground window went to the port; the next session
  should run `NF_ISO=… python3 tools/test/suite.py` on the merged tree — my changes are
  cfg-gated, so the risk is confined to the mp/bots compile, which the batch covered).

## Gaps (the next session's list)

1. The move states 0xD4/0xD5/0xDB/0xDC (`chooseCombatMove`'s targets): the roll consumes the RNG
   in the game's order but `checkAttackMove`'s stand-in returns None → the bot stays 0xCF. The
   same for 0xD0/0xD7/0xD3/0xF6 targets (the transitions set them; only 0xF6's resume marker and
   0xD3's id are carried).
2. The cover family (0xE0..0xE8) and the death branch of the heartbeat (the caller owns
   aliveness; 0xF2/0xF4 are not ported).
3. The `BotGlobal` arms' helper calls beyond the skeleton (ReactToDroneAlertMsg 0x1437A0's bot
   SetStates 0xEE/0xED/0xCF, combatWeaponChangeChoice 0x128708, cancelGoalToObj 0x128590 is
   ported as a field teardown — the setStateChange(0xF9) arm is in).
4. The AINetwork route wiring: `gotoGoal`'s `AINetwork_SetupGoalPosition[ToObj]` and
   `MoveToGoalPosition`'s `FollowRoute` — the port walks straight-line at the stats' speed
   multiplier (0.7/1.0/1.3 [M atlas §4]) with the arrival radius 1.0 [S]. The campaign nav module
   exists; the arenas' nav coverage is unverified.
5. `BOTWEAP_InitWeapon`'s default weapon (the PickupMatrix row) — the tests set the weapon id +
   burst params directly; `aggression_mul`'s exact table (`BOT_getAggressionMul`, switches on
   Drone+0xB5 per the research — a possible stats-block naming swap for bot-vars.md) is [S].
6. The "stand" preset's game-side identity: the port's `ctx.stationary` maps the attack states'
   prop-0x40/`Drone+0x4F8`-bit-0x10 gate [M 0x16f198:42-45] and (as an extension, [S]) the 0xEB
   walk; which stats row/prop the game's bot menu calls "stand" needs the live check below.

## Shared files touched

- `docs/fields/bot-vars.md` — 4 row corrections (+0x28 the 300.0 constant, +0x38 the f32
  reading, +0x76E the reader found, +0x767 the round-robin cursor) + live-check #1 re-pinned.
- `docs/atlas/multiplayer.md` — §4 got the phase-4 status paragraph (append into the section).
- `docs/WORKLOG.md` — the `mp-bots-states` entries (append-only).
- No source file outside `crates/nf-game/src/mp/bots.rs` and the new test was touched.

## Live checks for the main agent

1. `gdbwatch` `0x26D660+0x28` (w) — expect **300.0** after a goal is picked (gotoGoal
   0x1278a8:58; re-pinned from 5.0).
2. `gdbwatch` `MP_PICKUP+0x80 + botidx*4` (w) — expect now+45.0 at the grab (0x129958:14; the
   original check stands).
3. In an MP match, watch `Drone+0x108` (the SM state) of a bot drone with the stationary prop —
   does the game ever leave 0xF9 (does a "stand" bot still walk to pickups, or does prop 0x40
   gate `FollowRoute` too)? This settles the stand-preset mapping.
4. `fieldprobe` poke `BOT_vars+0xAB := 4` (trait 4) in an FFA with a scoring leader — expect the
   bot to hunt the top scorer (the 0x128b50 case-4 read of `MPGame+slot*0x30+0x18` as score).
5. nf-view render spots once the drones render in MP: `--mp` on 07000023 with 1 bot — the bot
   should walk from its spawn toward the nearest pickup placement; the state trace in the test
   output names the expected transitions (0xF9 → 0xEB → 0xF9).

## ntfy failed

(none — notification sent at finish)
