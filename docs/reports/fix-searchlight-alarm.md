# fix-searchlight-alarm — 07000006 searchlights and the guards' reaction time

Branch `s/fix-searchlight-alarm`, final commit 185e9c4 (from master 12bae32). Session 07:46–09:46 UTC
2026-10-13, sandbox deadline honoured.

## Done

1. **The searchlights raise the alarm now** (the user's "no alarm"). Root cause: the courtyard heads
   (`0xDE` kind-1 placements, on-channels 64/63/61/62, alarm channel 55) start in state 3 and only
   light when their on-channel rises. The disc stages exactly that: NIS `0x06000066` stream 0 raises
   ch 64 at wait 1, ch 63 at 59, ch 61 at 116, ch 62 at 167 [D 07000006 kind-7 NIS 0x06000066, dumped
   with `nf_formats::nis::Stream::decode`]. **Nothing in the level's placements, triggers, sub-scripts
   or anim streams references that script** (every placement param scanned; the raw hash bytes occur
   in 07000006.bin only inside compressed-data noise), so the game's activation beat is not derivable
   from the disc. The port boots the disc's own script once at level start, mode 1, through the real
   Scripts pipeline (`Scripts::new`, the `spawn_once` hidden-one-shot precedent [M 0x1dbc08's mode
   dispatch]). With it the heads sweep, the spot ray arms on Bond (dist ≤ 2.0 [M Searchlight_Update
   0x1cb900:251-263]) and state 1 writes alarm channel 55 every tick [M 0x1cb900:142-144]; the guards'
   second switch (`Drone+0x135` = 55, tag 100 on the courtyard guards [L placements 436-450]) then
   sends them into their attack states [M PreDroneControl 0x148db0:51-61].
2. **The aiming gate in `choose_combat_move`** gained the game's armed half: the gate's "opponent is
   aiming at me" is `DroneWeap_OpponentIsAimingAtMe` = armed Bond AND the `Drone+0x2A` threat latch
   [M 0x17a500:12-29], the latch stamped by `Check_Target` [M 0x1a7810:149-150] — the port's
   `aimed_at` latch existed but an unarmed Bond counted as "aiming". The gate's other arms were
   already faithful (`Rand(15) < 7` / +0x17 = `DroneMove_NoBunching`'s result [M ControlSTANDARD
   0x1490f0:224-225] = the port's `bunched`).

## How it was verified

- `crates/nf-view/tests/fix_searchlight_alarm_live.rs` (NF_ISO, the real `nf_view::load` +
  `Sim::step_bond` path), fail-first — pre-fix the probes showed the heads frozen at state 3 through
  400 ticks with Bond in the beam and ch 55 never rising:
  - `a_courtyard_searchlight_arms_and_raises_the_alarm_channel`: a head leaves state 3 from the boot;
    Bond at the 06-searchlight.trace's beam spot (4.8, 42.09, −16.0) arms a head (state 1); ch 55
    rises; after the consumption window ≥ 3 guards sit in attack states.
  - `the_first_shot_never_lands_inside_the_game_s_reaction_floor`: at the 06-firstsight.trace sighting
    spot (Bond 83.58, 35.13, 33.21; d9 15.7 units, d1 8.7 units) the first shot lands 26–180 ticks
    after the alertness jump — the game's floor (reaction 10 [M 0x14ac60 default arm] + BurstDelay's
    15 [M 0x179ba8] + interval [M 0x179d40]) to just above the game's slowest measured chain (128 [L]).
- Full suite on the merged tree (`--no-fail-fast`, slim ISO): **589 passed, 3 failed — only the 3
  allowed `disc_golden` tests**.
- `crates/nf-view/tests/fix_drone3_live.rs`'s light test re-premised: it asserted the head starts
  state 3 after 2 ticks; the boot's ch 64 fires at wait 1, so it now asserts the lit behaviour with
  the manual channel raise removed (stronger: the boot alone lights it).
- Trace numbers used as anchors [L tests/traces/06-firstsight.trace]: alertness 0.1 → 1.0 in one
  frame (d9 @148595 at 15.7 units → AIM_STAND 0x67 for 31 frames → COMBAT_NO_SIGHT 1 frame →
  RUN_FOR_COVER; first hit +70; d1 @148537 at 9.6 units → STEP_AIM_LEFT 0x73 for 116 frames →
  AIM_STAND_FIRE 0x68; first hit +128, 12 frames inside the fire state).

## Shared files touched (small, additive)

- `crates/nf-game/src/nis.rs` — `Scripts::new`: the pinned 07000006 light-boot player (one additive
  block; no existing lines changed).
- `crates/nf-game/src/drone.rs` — `choose_combat_move`: `aimed` now also requires `bond_armed` (one
  line + comment).
- `crates/nf-view/tests/fix_drone3_live.rs` — the light test's premise updated for the boot (its
  manual ch-64 raise removed; asserts unchanged in intent).

## What's left / live checks for the main agent

1. **The 0x06000066 activation beat** (the one [S] in this task): the disc's script exists and stages
   the channels, but no parsed reference fires it. In PCSX2 with `NF_SCRIPTLOG`-equivalent logging
   (or `gdbwatch` on the script-player list), walk the truck-arrival path into the courtyard and
   record which ScriptPlayer runs 0x06000066 and on which beat (level load? corridor exit? a
   `SP_Activate`?). If the beat is NOT level start, move the port's boot to that trigger.
2. **"Spotlights look weird" — needs the render** (no GPU here; `docs/fields/searchlight.md`'s
   port-mismatch list is the checklist). Suggested `--bond` spots on the fixed build:
   - `--bond 4.8,42.1,-16.0,-2.2,0` — the entrance courtyard: all four heads sweep; beams land on the
     walls/ground, the ground spots glide, coronas show when a lamp faces you.
   - `--bond 7.1,48.6,-23.6,-1.4,0.2` — head #4 (7.1, 47.6, −23.6) close up: beam cone matrix and the
     corona billboard (alpha `(dot−0.2)·4·255`, scale ×1.5 when LOS-clear [M 0x1cb900:334-346]).
   - `--bond 5.5,56.7,-50.1,0.2,0.2` — head #8 (5.5, 55.7, −50.1), the steep pitch (mid 56°).
   Known visual deltas to check against the game: the corona LOS origin is the **camera** in the game
   (`Collide_LineOfSight(glb_viewer+0xc0, …)` [M 0x1cb900:338-340]) vs the port's Bond eye; the
   kill/break paths hide the mount in the port while the game keeps it (searchlight.md mismatches
   1/2/5). Note the pre-fix build had the lights permanently DARK — much of the "weird" may simply be
   gone now that they sweep.
3. **Reaction band, second pass**: the port's machinery matches the decompile (the combat-move gate
   [M 0x150c18:59-79], immediate fire on entering AimStandFire [M 0x166328:59-61], BurstDelay 15-60
   [M 0x179ba8], NextBulletTime `+0xBC8 += BurstDelay` [M 0x179d40], reaction 10 on 07000006 [M
   0x14ac60]). The measured port chain (first shot +62 after the jump) vs the game's +128 is the
   combat-move roll outcome (the game's d1 rolled a 116-frame STEP_AIM). If a retest still feels "too
   soon", re-record `06-firstsight.trace` holding the visibility accumulator AND the misses before the
   jump (the recorder note) — then the per-frame `+0x50C`/state numbers can pin the exact divergence.
4. **RUN_FOR_COVER never fires in the port** (observed: the port's d9 sat in the cover chain through
   400 ticks with zero shots; the game's d9 hit at +70 from that choreography). The game's
   RunForCover fires on timer C while Bond is seen [M 0x169150 case 0xC: `DroneWeap_Fire(1)`]; the
   port's `(RUN_FOR_COVER, …)` handlers have no timer fire. Not fixed here (the timer's arming site
   needs its own pass).
5. `+0x2A`/`+0x2C4` threat latch: the port's `aimed_at` (Check_Target's 30-tick window) stands in for
   the game's latch [M 0x1a7810:149-150 + PreDroneControl's 30-frame decay]. A live check: poke
   `drone+0x2A` = 0 while Bond aims at a guard — the guard should stop preferring combat moves.

## Doc updates

- `docs/WORKLOG.md` — this task's entry (appended).
- `docs/fields/searchlight.md` needs no correction (its port-mismatch list stands; items 1/2/5 remain
  the visual follow-ups).
