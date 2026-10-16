## Your task: `ninja-melee` — the Ninja melee chain

Audit gap ③: the Ninja drones' melee chain, DSTATE 0xab-0xb3 [M], isn't ported. Port:
- the approach, the attack selection, each strike's anim and damage;
- the block/dodge states;
- the transitions.

Do it on the level that places Ninjas [D].

Tests: a Ninja engages Bond in melee with the game's sequence and damage; his states follow the game's transitions
over a scripted fight.

The uncited-gameplay audit (task 710, merged) has this gap's description, caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that section first and meet its criteria.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/ninja-melee`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T20:41Z

---
## Report (done, 2026-10-16T21:31Z)


---
## Report from the worker (done, 2026-10-16T21:31Z)

# ninja-melee (queue 728) — the Ninja drones' melee chain, DSTATE 0xab–0xb3

Branch `s/ninja-melee` (from origin/master 170bcf01). The ported NinjaAttack router (0xa6–0xaa +
0xb2) collapsed the close band into standing and shooting; this task ports the whole chain the
game's `NDrone2_StateFuncs` table wires for dtype 0xf (mode 0xd), on the level that places the
Ninja (07000004, the assassin [D docs/levels/07000004.md]).

## State-id ground truth (settles the audit's ids)

`NDrone2_StateFuncs` (EE 0x29b300, 4-byte fn pointers) read from the EE snapshot [L ee_snapshot,
file offset = EE address]: 0xab → 0x16cb70 NinjaGetCloseToPlayer, 0xac → 0x16d100 NinjaSword,
0xad → 0x16d298 NinjaSomersault, 0xae → 0x16d430 NinjaBackflip, **0xaf → 0x173e08 NinjaSideflip
(the ±3 router)**, **0xb0 → 0x16cdd0 (anim 0x68)**, **0xb1 → 0x16cf68 (anim 0x69)**, 0xb2 →
0x16d5b8 NinjaStandFire, **0xb3 → 0x16d878 NinjaNoRoute**. `uncited-audit.md` gap ③ lists
0xaf/0xb0/0xb1/0xb3 shifted by one state (it calls 0xb1 NoRoute); the table wins — kiko.rs's doc
records the correction. (matches docs/fields/dronetype-tables.md.)

## What was ported (crates/nf-game/src/drone.rs, addresses in crates/nf-game/src/kiko.rs)

- **0xab GetClose** (0x16cb70): ENTER anim 0x6b, TIMER_C at +180, Fire(1); UPDATE ≤2 → 0xac,
  MoveToObject(2.0) direct+clear-5.0-hop+5<dist<7 → 0xad, moving → anim 0x67 + the
  stay-while-anim gate, 10 → stay, else → 0xa7; TIMER_C → 0xaa.
- **0xac Sword** (0x16d100): ENTER ≤2 → the 0x6d slash with next state 0xa7 (the decompiler
  dropped CallAnim's blend 0; every sibling strike passes an explicit next state, and a sword
  state with none could never exit), else → 0xa7; UPDATE seen + angle. The slash's damage rides
  the already-ported anim melee event (`DroneWeap_CloseCombatImpact` — anim events 5/10/0xB →
  the melee Bullet from record 0x65/0x66/100 with the 1.75 range and the 60° facing gate).
- **0xad Somersault** (0x16d298): invalidate route, anim 0x6a → next 0xac, Fire(1).
- **0xae Backflip** (0x16d430): anim 0x66, variant Rand(3)!=0, next 0xa7, Fire(1).
- **0xaf Sideflip** (0x173e08): ENTER-only ±3 dispatcher — both clear → Rand(2) between 0xb0/0xb1,
  only +3 → 0xb0, only −3 → 0xb1, neither → 0xa7.
- **0xb0/0xb1 flips** (0x16cdd0/0x16cf68): anim 0x68/0x69, variants Rand(3)==0/!=0, next 0xa7,
  Fire(1); EXIT clears +0x3B (firing).
- **0xb2 StandFire** (0x16d5b8) — replaced the stand-in with the real handler: anim 0x6b,
  TIMER_C at +180, Fire(1); UPDATE the somersault / 10% sideflip alternatives after the first
  held tick, else re-fire; TIMER_C → 0xa7.
- **0xb3 NoRoute** (0x16d878): ENTER alert 1.0, TIMER_C at +30, anim 0x6b, Fire(1) — the star
  throw; UPDATE re-fire + angle + `+0x3B = (+0x228 >> 2) & 1`; TIMER_C MoveToObject(2.0): a
  route again (≤3) → 0xa7, else re-arm at +30.
- **0xa7** ENTER/UPDATE now runs the game's real routing (`ninja_attack_route`): the bands
  (>12/6/2) and the close band's evasion rolls (ENTER: 10% sideflip, backflip test, Rand(2) →
  sword 0xac / stand-fire 0xb2; UPDATE: backflip first, then the 10% sideflip), plus `+0x4F8 |=
  0x80000000` on ENTER. **0xa8/0xa9** now carry their real UPDATEs incl. the no-route → 0xb3
  arms and theTIMER_C alternatives (0xa9).
- **Support**: `Drone+0xEC` melee band (`melee_min_dist`: 4.0 generic [M 0x14b300:378], 2.0 +
  health 100.0 for mode 0xd [M 0x14b300:744-753], set in `type_setup`); `Drone+0x2C8` attack-hold
  counter (`attack_hold`, incremented while seen in the tick [S — the +0x2A writer is untraced]);
  the ninja anim-state constants 0x66/0x67/0x68/0x69/0x6a/0x6b/0x6d.

## Tests (crates/nf-view/tests/ninja_melee_live.rs, both green)

- `the_assassin_gets_close_and_swings`: 07000004, Bond beside the assassin for 900 ticks — the
  close band is reached (0xab or 0xac seen), the game's transitions appear (0xab → 0xac /
  0xac → 0xa7 / an evasion state), the throws discharge (`shots`) and the damage lands
  (`player_hits`; point-blank the projectile reaches Bond inside its spawn tick — dist < 3
  always hits, fire_weapon — so the flight is observed through the landed damage).
- `no_route_throws_stars_and_recovers`: NoRoute entered directly throws on ENTER/UPDATE
  (projectiles in flight observed), sets alert 1.0, and its TIMER_C (+30) with a route again
  returns to the attack chain (0xa7, whose ENTER re-routes by distance inside the same send).
- `fix_finale_live.rs`'s assassin assertion widened to the full fight-state set (`kiko::NINJA_FIGHT_STATES`).

## What I ran

- `cargo test --offline --release -p nf-game`: 54 test binaries, all `ok`, 0 failures.
- `cargo test --offline --release -p nf-view --test fix_finale_live --test ninja_melee_live
  -- --test-threads=1`: 4 + 2 passed, 0 failed.
- The full four-crate fast runner (`tools/test/suite.py`) was started twice per the driver's
  instruction; this sandbox kills background processes between tool calls (both runs died
  silently during the build phase, `suite.out` 0 bytes — same as the foreground `timeout 25`
  probe, which prints nothing until its build completes). With ~20 minutes of session left there
  was no window to run it to completion in the foreground. The touched crates (nf-game,
  nf-view's drone/finale tests) were run directly as above. The next message should run the full
  suite on this tree (`NF_ISO=<iso> nohup python3 tools/test/suite.py > suite.out 2>&1 &`).

## Risks / notes for the next message

- The ninja's placement weapon is 0x12 [D 07000004 placement]; the thrown projectile is a
  `Bullet` with that weapon id, exactly as the game's `DroneWeap_Fire` → `FireWeapon` chain
  initials it. If the disc's star visual differs, that is a weapon-data/render question, not a
  sim one.
- The sword CallAnim's blend argument is decompiler-dropped; ported as 0 (all siblings do), with
  the reasoning cited at the 0xac arm. A disassembly pass (no MIPS objdump in this sandbox) can
  double-check.
- `attack_hold` (+0x2C8) is gated on the seen latch [S]; a gdbwatch on 0x148db0's +0x2A writer
  would settle the gate.
- 0xaa's UPDATE min-dist evasion roll is dead at the ninja's 2.0 band inside 2<dist≤6 (ported
  anyway; live only for a generic +0xEC 4.0).
- The 0.5/1.0 DoHitEffects scale differences between the ninja states (0xac–0xb1 use 0.5,
  0xa7/0xa8/0xaa/0xb2/0xb3 use 1.0) ride the port's GLOBAL pain/death handler, which does not
  model the scale — a pre-existing deviation, unchanged by this task.

## Live checks for the main agent

The fight is visible on 07000004 — after the intro story scene, Bond is placed next to the
assassin at (−3.87, 5.44, 30.30); the assassin stands at (−2.64, 5.50, 30.08) [D]:
- `--bond -3.87,5.44,30.30,<yaw>,0` (yaw facing the assassin ≈ −2.0 rad): expect the ninja to
  run in (anim 0x6b), then alternate sword slashes (0x6d), star throws, and the flips
  (0x66/0x68/0x69/0x6a) instead of standing and shooting.
- `--bond 2.0,5.5,26.0,0.5,0` ~5–6 units away: expect the somersault-toward-player alternative
  (0xad) or the sideflips when the TIMER_C alternatives fire.

## Usage

My own session's token/context/cost figures are not visible to me (no such readout in this
environment). The subagent runs I made did report figures, read from their result footers:
research agent "map port ninja internals" — `Tokens -> prompt:190862 completion:6450
total:197312`; research agent "map headless test infra" — `Tokens -> prompt:127093
completion:3768 total:130861`.
