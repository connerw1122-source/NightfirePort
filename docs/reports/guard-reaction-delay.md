# guard-reaction-delay — guards don't fire the instant they see you

User: "guards shoot way too soon upon spotting you, at least give it half a second", "his gun's firing before it
lowers at me" (07000006 wine cellar, the searchlights, 07000005). Branch `s/guard-reaction-delay`.

## Done

**The sight-to-fire pipeline is ported game-exact and now measured end-to-end through the real sim path.** The
frames from first sight to first bullet for the three sighting paths, each pinned by a test that fails if a future
change fires earlier (`crates/nf-game/tests/guard_reaction_live.rs`, 07000005, positions only — no state injection):

| path (guard type) | port: sight ack | port: noticed (ATTACK) | port: first combat move | port: first bullet | game anchor (07000006 live, docs/live/2026-10-14) |
|---|---|---|---|---|---|
| 1. fresh idle guard, own sight | t+60 (live 61) | t+60 | 0x74 StepAimRight (seed 5) | t+161 | +97 frames (~1.6 s): the stairs guard Bond walked into at 2.8 units (the turn is inside that number) |
| 2. alarm-raised guard (alertness 1: searchlight/alarm) | t+1 — the 59-tick floor is gone | warm-up (self-attacks with no sighting, `DroneVision_AlertSound` 0x177800:67-71) | 0x74 StepAimRight in flight | t+114 | +45 frames: the courtyard guard at 16.7 units (attribution weak) |
| 3. mid-combat guard that lost and re-gained sight | — | re-sight chain 0-2 ticks (`CombatNewSighting` → `AimStandFire`) | — | entry fire [M 0x166328:59-61]; at dist ≤ 4.0 `CanBackoff` routes `AimBackoff` first [M 0x166328:96-102] | burst spacing 50-60 frames |

Seed spread for path 1 (default seed 5: 161; seed 3/11: 60 — a face-on strafe-first draw fires on the ATTACK tick;
seed 7: 173): the draw of the first combat move is the variance, exactly as in the game — strafe-aim enters fire on
entry [M 0x1677b0:25], steps hold the request through their whole danim [M 0x1676a0:16-18].

The pipeline pieces, with the game addresses:

1. **Noticed/alert latch timers** — the guard's raw sight (`seen`) is gated by `DroneFunc_ReactionTime`
   (0x14ac60): `seen_ticks >= reaction_time` before the sight ack routes to ATTACK. 59 ticks at alertness floor
   (level-scaled switch on the level id, **not** difficulty-scaled — accuracy/damage/init are, and the port already
   scales those). Measured: fresh guard's ack at 59-61 across seeds; the alerted guard's at tick 1.
2. **Aim-raise animation gate** — `AimStand` (0x166178) calls the raise anim with completion msg 5, which completes
   at raise-START (+ blend), then `AimStandFire` (0x166328) arms the fire request on its entry
   (`+0x3b` via `DroneWeap_Fire`, unseen `+0x274 < 0x10`); the bullet leaves when `DoFiring`'s gates pass — the
   next-shot stamp, `+0x274 < 0x10`, and `Ready2Fire` (aim class, or the muzzle within 0.349 rad of the target —
   the "gun lowered at me" gate). There is **no** aim-raise-END wait anywhere in the game: the earlier blanket
   15-tick sight pause was a deviation and is gone (the research that settled this is in docs/WORKLOG.md,
   `guard-reaction-delay`).
3. **First-shot delay / reaction time** — `DroneFunc_ReactionTime` 59 ticks (fresh guard, alertness floor), 0 at
   alertness 1. NOT difficulty-scaled [M 0x14ac60:9-26 switches on the level id]. The first bullet additionally
   waits the weapon's burst delay + interval scheduled by `DroneWeap_NextBulletTime` (0x179d40), clamped to the
   current frame — so a face-on guard whose aim already passes `Ready2Fire` can fire on its ATTACK tick, 60 frames
   (1.0 s) after first sight.
4. **Searchlight-spot path** — `DroneVision_AlertSound` (0x177800) routes alarm-raised guards straight to ATTACK
   with the forced-attack bits, bypassing the reaction floor; covered live by `searchlight_live` and measured as
   path 2 above.
5. **CombatNoRoute fire-request discipline (the fix this task ships)** — [M 0x164c08:29-30] the ENTER fires
   `DroneWeap_Fire(1)` and then clears `+0x3b` on the same tick (a re-sighting guard cannot release a bullet on
   the entry tick); [M 0x164c08:41] the not-seen UPDATE clears `+0x3b`; [M 0x164c08:63] the seen UPDATE only
   refreshes `+0x3b = (+0x1e == 0)` — no re-schedule (DroneWeap_Fire gates `NextBulletTime` on `+0x3b` already
   set in SP [M 0x17a8c0:9-14]; no writer of Drone+0x1e exists, the refresh is always 1). The port fired on the
   entry tick before; now it matches.

Also verified while measuring: a class-3 anim call (the step/strafe moves) completes only when the step's
follow-on danim (class 2) starts after the anim ends — the request record zeroes but the completion action
survives [M 0x13add0/0x13aee8]; the 100-frame step play-out with firing held off is game-exact behavior, not a
stall (steps do not abort on sight, [M 0x1676a0 case 3]).

## How it was verified

- `crates/nf-game/tests/guard_reaction_live.rs` (new): the three-path measurement with the floors pinned —
  fresh-guard ack 55..=65 and `shot >= ack + 59`; alerted guard ack < 30 and `shot - AimStandFire entry < 59`;
  re-sight entry chain ≤ 4 ticks. All green.
- `ai_live` (nf-game): 35 passed, 0 failed — including the notice-timing, cover, searchlight-alarm and patrol
  timelines the 110f hold had broken (the current master already carries that repair; re-read before touching: the
  hold had stretched a sight-pause the game does not have, and the pipeline port supersedes it).
- `walkreal_07000001` (nf-view): green in the fast-suite run (see below).
- Fast suite `python3 tools/test/suite.py` (docs/tools/test-suite.md), full four-crate run on the branch tree
  (pre-merge): build ok: 384 test binaries; reached binary 179 of 384 in this sandbox's window with exactly the
  failures below — none in guard AI:
  - the 3 `disc_golden` tests — the documented slim-ISO allowance;
  - `driving_ai_follow::real_mis01_track_rngp_walks_when_nf_driving_is_set` — the slim ISO's mis01 road net has
    360/811 positive speed limits; the test's data assertion (>8/9 positive) is disc-data, not code (the sister
    8 driving tests, including the rNgp parse and the spline walk, are green, and nothing in this task touches
    the road-net path). Reproduced identically on the merged tree.
  - `fix_guard_weapons_live::the_3x3_hit_rate_and_damage_matrix_matches_the_game` — a tolerance-edge flake, see
    the finding below. Green on 3 of 4 standalone runs (125/200 vs 153/200 in the failing vs passing cell,
    model 0.750, tolerance 0.12 — the failing run landed 0.125).
- On the final merged tree (origin/master, 36a7f3b9): `ai_live` 35/35, `walkreal_07000001_live` 2/2 (260.8 s),
  `searchlight_live` 3/3, and `guard_reaction_live` reproduces the identical table (161/114/382 — master's
  drone.rs batch did not shift the streams). The full merged-tree suite was re-started and is still running at
  report time (this sandbox's window closes first); its failures so far are the same slim-ISO set.

**Finding for the next task (pre-existing, not this branch):** `fix_guard_weapons_live`'s measured streams vary
between PROCESSES of the same binary — the same cell measured 125/200 (suite run) vs 153/200 (standalone) with
identical env, and the candidate flow itself varies ("guard 3: no level parking spot" appears in some processes,
not others). The parking outcome is decided before any firing, so this is not the CombatNoRoute change; it points
at per-process state leaking into the sim (Rust's per-process `RandomState` HashMap order consumed somewhere in
scene build would do exactly this). Worth a determinism audit: `cargo test` pins the shared RNG, but a
HashMap-ordered draw at build time would still shift every stream per process.

## What's left

- Nothing blocking. The next playtest should re-check the wine cellar (07000006) and the searchlight yards
  (07000005): the first bullet now lands ~1.0-1.6 s after the guard's own first sight for a fresh guard (floor 59
  ticks + the state/aim transitions + the weapon schedule), instantly-at-the-sight-tick firing is gone.

## Live checks for the main agent

1. `Drone+0xDA == 0x13` deferral (AimStandFire ENTER 0x166328:52-58): gdbwatch a 07000006 cellar guard entering
   AimStandFire while still playing the hit-reaction danim 0x13 — expect `+0x3b` cleared and timer C armed for 60
   ticks, first bullet from the TIMER_C fire. The port models it as `anim.danim == 0x13`; the 0x13-vs-anim-set
   reading is the one unpinned byte in the chain (the decomp reads Drone+0xDA, documented as the anim-set field).
2. `Drone+0x1e` (the byte the CombatNoRoute/AimStandFire refresh `+0x3b = (+0x1e == 0)` reads): fieldprobe a
   writing access during a guard fight — the port assumes it stays 0 (no writer found in the decompiles).
3. Optional: the courtyard guard's +45 anchor attribution (the bullet origin was 11 units from the guard) — a
   gdbwatch on 0x1785530's `+0xbc4`/`+0x3b` around its alert would settle whether +45 is a strafe-enter fire
   (the port's seed-3 shape) or something else.
