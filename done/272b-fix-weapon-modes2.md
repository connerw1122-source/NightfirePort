## Your task: `weapon-modes2`, land fix-weapon-modes on current master

Read `docs/reports/fix-weapon-modes.md` on `s/fix-weapon-modes` (4ed1519) first. Its findings are evidence-based and
wanted (the ○/M mode switch, the Scorpion's heat-seeker `Bullet_homing` 0x12f358, the op-1 sound table pinned, the
0x07000009 grants and the Q-Worm, the gadget id corrections). The main agent merged it on top of master (which had
gained fix-movies, fix-menus2, fix-sky and fix-restart-target since your base 6519d6f), and the suite went
**764 passed, 4 failed**, so it was backed out:

1. `fix_weapon_modes_live::p2k_mode_button_toggles_the_silenced_twin` (line 49): the switch reaches weapon 0x03 but
   `switch_state` is still **8** after 200 ticks ("and settles idle" expects 0). It passed on your branch.
2. `fix_weapon_modes_live::the_heat_seeker_homes_onto_a_guard_and_the_unguided_twin_does_not` (line 141): weapon 0x2e's
   guard-hit verdict is `false` (the homing round misses).
3. `portal_live::cold_loadout_arms_bond_on_every_story_level` (line 210): on 07000009 Bond draws weapon 67 after 30
   ticks, not the P2K. **The game agrees with your grants**: `Player_InitWeapon` 0x1a7ef8 case 0x7000009 equips the set
   and sets the switch request to `uVar6` (the dart) [M :192-202]. So that test's expectation for 07000009 is the
   one to update, with the citation. Check the other levels the same way.
4. (Also red in that run, but already fixed on master: `walk_0700000a_live`'s Restart expectation.)

Start from your branch, merge `origin/master`, reproduce 1 and 2, and find which of the newly merged changes interacts
with the switch cycle and the homing (candidates: fix-progress's runtime difficulty threading into `Data`, fix-menus2's
input/front-end handling in `lib.rs`, fix-restart-target's new `Drones` fields, fix-sky's tick additions). Fix it from
evidence. Don't weaken the tests. Run the full suite with `--no-fail-fast` and report the count.

Files: as fix-weapon-modes, plus `crates/nf-view/tests/portal_live.rs` (the 07000009 expectation only).

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail. Report the count.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.

**Branch:** `s/weapon-modes2` (start from `s/fix-weapon-modes`, then merge `origin/master` into it).

---
claimed-by: sandbox4 at 2026-10-13T17:46Z

---
## Report (done, 2026-10-13T19:04Z)

# fix-weapon-modes2 — landing fix-weapon-modes on current master

Branch `s/fix-weapon-modes2` = `s/fix-weapon-modes` (4ed1519) + merge `origin/master` (3576133, the
commit that held the backed-out merge). Every fact is tagged [M] decompile (`work/decomp/ACTION.ELF/`
file:line), [D] disc, [L] measured bytes, [S] suggested.

## What the task found (and what it actually was)

The main agent's merge run showed 4 reds; the premise was that newly merged changes interacted with the
mode switch and the homing. **They did not.** Both weapon-modes reds fail on the UNMERGED
`s/fix-weapon-modes` in this sandbox, with either ISO (`Nightfire-onfoot.iso` and the retail disc) —
and fix-weapon-modes' WORKLOG entry ends at "Suite: see below" with no count recorded (the sandbox
expired at 18:32 before the tally landed), so the "passed on your branch" premise was never backed by a
recorded run. The other two reds were as the task predicted:

1. `p2k_mode_button_toggles_the_silenced_twin` — `switch_state` stuck at 8 (fixed, see below).
2. `the_heat_seeker_homes_onto_a_guard_and_the_unguided_twin_does_not` — verdict false (fixed, see below).
3. `portal_live::cold_loadout…` on 07000009 — the game agrees with the port; the expectation was
   re-pointed with the citation ([M 0x1a7ef8:132, :192-202]).
4. `walk_0700000a_live` Restart — already fixed on master; green in every run here.

## Root causes and the fixes

**The P2K variant swap zeroed the live magazine and a reload stole the view script.** The port's
state-0→8 variant path round-tripped `gun.clip → clip[old]`, `clip[new] → gun.clip`. The game's variant
path [M `Player_WeaponSelect` 0x1a2cc0:18-29] writes only the weapon byte (`+0x62`), the mode byte, the
`+0xC8` script at 1.3× and view state 8 — no clip access anywhere (the twin is the same physical gun;
`clip[0x03]` is a never-equipped slot, so the P2K's 7 live rounds vanished into `clip[2]` and
`gun.clip` read 0). And the reload branches both gate on the VIEW STATE being 0 — [M
`Player_WeaponFiring` 0x1a5460:409 (the R-key branch, `if (0xf4 == 0)`) and :447-450 (the auto-reload
branch, `if (0xf4 != 0) return`)] — while the port's `busy` covered only the fire scripts, so the swap
tick started an auto-reload whose `+0xA4` script (06000827, len 155) overwrote the `+0xC8` variant
script (06000828, len 201); state 8 waits for `view_time ≥ len−1 = 200` while the reload drove
`view_time` to its 154 cap → stuck at 8 forever (traced: `state 0→8 variant-swap 06000828` then silence,
`reload_state=6`, `view_time` capped at 154). **Fixes** (`crates/nf-view/src/scene.rs`): the clip
round-trip is gone from both variant paths (the scripted 0→8 and the no-script instant swap), and `busy`
gains the switch states (`switching`), matching [M 0x1a5460:409,447-450] — a reload can only START from
the fully idle view.

**The heat-seekable gate used the wrong field and missed the state gate.** [M `Bullet_heatseekable`
0x12f248] admits a class-2 guard while its object exists (`+0xfc != 0` — the port's `drawn` cull
stand-in, per the spawner template comment in `drone.rs`), its health (`BL+0xac`) is > 0 and its view
state (`+0xf4`) is not 1 (`WAIT_SWITCH`). The port used `present && enabled && !corpse` — the game does
not read the `+0xF0` enable byte here, and a spawner-held template parked in WAIT_SWITCH (this level's
drone 0; its `enabled` gets cleared by the WAIT_SWITCH enter arm) was never seekable, so no target was
ever acquired (traced: `dir` frozen at the spread value, `target=None`). **Fix**
(`crates/nf-game/src/drone.rs`): `seekable = drawn && health > 0 && state != WAIT_SWITCH`. The homing
scan/steer itself (nearest ≤ 1000, dot > 0.66, `Collide_LineOfSight(…, 8)`, the `+0xfe & 1` re-scan, the
`clamp((to−dir)·0.0333, ±0.01)` steer) already matched [M 0x12f358] and needed no change.

**The homing test's geometry was unwinnable as written.** The start room's gate pillar walls off
dx ≤ +4 from z+6 on at bullet height (segment sweep [D]); the missile's first ticks fly the straight aim
ray, so it died on the pillar at t23 (~z+6) regardless of homing, and the eye-height LOS scan passed
over the lintel the bullet actually hits. The test now stands Bond past the pillar mouth
(`Bond::stand_at`, dx+6/z+4), scans the guard spot at the MUZZLE height (+0.7), places him at dx+1.2,
z+14 (the straight twin still misses his boxes by > 1 unit; the homing spiral converges and hits), and
pins the guard to his spot each tick — the teleport stands him on a floorless point (no navmesh there;
a real guard's floor snap is placement-driven), without the pin he falls through the world and out of
the cone (`y` 26 → −152 traced).

**The 07000009/0a/0b cold-loadout expectation.** [M `Player_InitWeapon` 0x1a7ef8 case 0x7000009] equips
the set (handgun 0x30, Stunner 999, Decryptor 0, Q-Worm 0x58, Q-Claw 0x50, the dart
`Upgrade_Weapon(0x43)` 999, Shaver 0x5b 999) and sets the request to `uVar6 = Upgrade_Weapon(0x43)` —
0x43 (67), the dart [M :132, :192-202]. Cases 0x700000a/b RamLoad and re-select the saved weapon,
falling into the case-9 loadout while the handgun is unowned — and a cold load skips `Player_RamLoad`'s
whole body [M 0x1957a8:9], so on cold 0a/0b the game's own answer is the case-9 dart (measured: weapon
67 after 30 ticks). `crates/nf-view/tests/portal_live.rs` now asserts 0x43 + the Q-Worm owned for
n ∈ {9, 0xa, 0xb} with these citations.

## Verification

- `fix_weapon_modes_live` 6/6 (the twin toggle both ways + the no-twin sister, the homing hit vs the
  unguided sister through the real trigger, the label ids, the Q-Worm grant, the op-1 table).
- `portal_live` 4/4 (including the re-pointed cold-loadout sweep over 13 story levels).
- Full suite, `NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
  -p nf-audio -p nf-view -- --test-threads=1`: nf-formats + nf-audio all ok; nf-game 31/31 binaries ok
  on two consecutive runs; nf-view 108-binaries ok in the first pass plus every remaining binary
  (walkreal_*, wire, world_fx, worldobj) ok — **0 failures; even the 3 `disc_golden` tests passed**
  (`driving_golden` on the slim ISO). One earlier nf-game run under full `-j2` load showed 3 failures
  (a 66-test and a 3-test binary) that two clean consecutive re-runs could not reproduce — flaky under
  load, not pinned to a test name; the main agent's own merge run is the arbiter.
- Nothing was weakened: the two red tests kept their verdicts and gained only [M]-cited setup that
  mirrors the game's own gates.

## Live checks for the main agent (PCSX2)

1. **The variant swap keeps the magazine** — 07000005, fire 3 rounds, ○ (mode switch) to the silenced
   P2K, then △/wheel full-switch to another weapon and back: watch the clip counter across the ○ swap.
   Expected (the port now): the ○ swap does NOT refill or empty the magazine — the rounds shown before
   the swap are the rounds after ([M 0x1a2cc0:18-29]). Watch `obj+0xf4` (view state) reach 8 once and
   return to 0 with no `+0xA4` reload anim queued (a reload mid-swap would show state 6).
2. **The reload's view-state gate** — with the P2K, empty the clip and hold fire: the auto-reload only
   starts once the view state is 0 ([M 0x1a5460:447-450]); during a holster/draw (state 1/3) pressing R
   does nothing until the draw completes ([M :409]).
3. **The heat-seeker's live targets** — 07000011/07000013 (copter levels) or any guard level: fire the
   AT-600 'Heatseeker' (0x2E) past a live guard within ~49° of the missile's direction; it bends and
   hits. Fire at a guard whose spawn group has not triggered (a WAIT_SWITCH template — e.g. guards
   behind the 07000005 gate before its channel fires): the missile should NOT lock them ([M
   0x12f248's `+0xf4 != 1`]). gdbwatch on `BU+0x40` (the target pointer) at `Bullet_homing` 0x12f358:
   non-zero only while a drawn, alive, non-WAIT_SWITCH object is in the cone.
4. **The island cold loadout** — boot into 07000009 cold: Bond should draw the dart gun (67), with the
   Q-Worm in the gadget cycle ([M 0x1a7ef8:132, :192-202]).

## What's left

Carried from fix-weapon-modes' own list (unchanged by this task): the briefcase's 'Phoenix Ronin'
deployable minigun (`GT_DeployMiniGun` 0x181560 — a new gt.rs), `Bullet_init`'s mode spread
(`+0x28 + mode·2` vs the shot counter), the grenade mode pair (0x60/0x61, the only mode-count-2
records), the decryptor decode mini-game, the camera's first-equip hint, the `gadgets.md` id-table fix,
and the +0x60 fire_sound vs view_anim_datum contradiction (needs one [L] watch to settle). Also open
from this task: pinning down the flaky-under-load nf-game failures with names (needs a run with per-test
timestamps).
