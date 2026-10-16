## Your task: `guard-barks-first-alert` — guard barks and FirstAlertState

Audit gap ⑨: the guard voice barks and `FirstAlertState` 0x39 aren't ported. Port:
- **The barks:** which line plays on spotting Bond, losing him, taking cover, reloading and dying; the
  voice-cooldown and selection rules; the sound ids [M + D: the voice banks].
- **FirstAlertState:** the guard's first-alert reaction.

Tests on 07000005: spotting Bond plays the game's bark once, within the cooldown rules. A guard's first alert goes
through state 0x39 with the game's timing. Tests are silent (NF_SILENT).

The uncited-gameplay audit (task 710, merged) has this gap's description, caller chains and acceptance criteria in `docs/systems/uncited-audit.md`. Read that section first and meet its criteria.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/guard-barks-first-alert`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T21:32Z

---
## Report (done, 2026-10-16T22:28Z)

Closed by the driver: the report is on `s/guard-barks-first-alert` (docs/reports/guard-barks-first-alert.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T22:42Z)


---
## Report from the worker (done, 2026-10-16T22:42Z)

# guard-barks-first-alert — guard barks and FirstAlertState 0x39 (audit gap ⑨)

Branch `s/guard-barks-first-alert` (from `origin/master` 65bf8a09). Task 731.

## What's done

### 1. `DroneFunc_FirstAlertState` (0x147f58) → state 0x39, the first-alert route [M]

The game's first-attack flow (`DroneFunc_FirstAttack` 0x148010) consults
`DroneFunc_FirstAlertState` when the attack is **not the drone's own sighting** (`+0x4F8 &
0x8000000` clear) and a pending alert is being consumed (`+0x4F8 & 0x70000000`). FirstAlertState
reads the receiver's `+0x4FC` — whose high half is the alerting drone's `+0x4FC` shifted by 16
(`DroneVision_ConsiderAlerted` 0x176250:226 copies `source+0x4FC << 16` into the receiver) — and
returns state **0x39** when the source was **interrogating** (its 0x20 marker → the receiver's
0x200000) and neither side carries a disqualifying marker (the receiver's own hurt 2 / attack
0x10 / interogate 0x20; the source's hurt 0x20000 / surrender 0x40000 / death 0x80000 / attack
0x100000), on levels **{07000005, 07000007, 07000041}** [M 0x147f58 decomp]. The ported route:

- `alert_pending` (new `Drone` field) models `+0x4F8 & 0x70000000`: 0x20000000 set by
  `DroneFunc_DoForcedAttack` 0x148c60:28 (whose SP gate `(word & 0x2020000) == 0x20000` is now
  enforced — the old arm attacked unconditionally), 0x10000000 by the sound alerts (AlertToPosition
  0x165270:115, SearchArea 0x165bf0:79, HeardNoiseSuspect 0x16b0e0:123, AlertSound 0x177800:70),
  0x40000000 by the alarm (EnemyAlerts 0x177c18:114). No decomp writer clears them; the receiver's
  own +0x4FC markers neutralise later consults (the game's own mechanism).
- `alert_source` models `+0x528` (stamped by the port's ConsiderAlerted loop next to the shifted
  copy it already made); `TestEndInterogationAssist` is evaluated per tick from the live source
  bits.
- The consult sits in a new `(ATTACK, msg::ENTER)` arm ahead of the alt-state arm (the game's
  order). *Documented ordering caveat*: the port's unarmed `weapon <= 1` arm sits above it, so an
  unarmed forced-attack recipient would draw his weapon first — no 07000005 guard is unarmed
  ([L] weapon 10 for every guard), and the armed path is exact.

### 2. States 0x39 / 0x3a [M]

- `INTEROGATE_ASSIST = 0x39` (`NDrone2_DSTATE_InterogateAssist` 0x1616c0, StateFuncs[0x39] [L
  0x29b300+0x39·4]): ENTER — no opponent (`+0x170 == 0`) → NoOpponent 0x60; else
  `SetInterogateAssist` (`+0x4FC |= 0x20` [M 0x178308]) and **timer C at now + Rand(120)** (0–2 s
  [M 0x161704-24]). TIMER_C — no opponent → 0x60; else invalidate the route and the goal at the
  opponent's object with the **10-unit arrival radius** (`SetupGoalPositionToObj` 10.0 [M
  0x15add0]; `MoveToGoalPosition` arrives when the route distance drops under it [M
  0x1517d8:80-83]) — reachable → return state 0x3A (`+0x5A2`) + GoToGoal 0x63, else → 0x3a
  directly. UPDATE is unhandled in the game (returns 0 → the base state). No EXIT arm.
- `INTEROGATE_ASSIST_WAIT = 0x3a` (`NDrone2_DSTATE_InterogateAssistWait` 0x161860): ENTER —
  `+0x4F8 |= 0x10000 | 0x2000` (the port's vision bit; 0x2000 may-notice is unread in this
  state), `SetInterogateAssist`, the **alert idle 0x21 when the drone's anim-object weapon byte
  (`+0xDC->+0x62` — the port's `weapon`) is under 2, else the aim stand 10** [M 0x1618d0-f4].
  UPDATE — no opponent → 0x60; the source gone or no longer interrogating
  (`TestEndInterogationAssist` 0x178348: its +0x4FC keeps only 0x20/0x200000) → Attack; else face
  the opponent. EXIT — `EndInterogateAssist`: `+0x4FC = (& 0xffdfffdf) | 0x10` [M 0x178318].
  msg 0x1f → DoForcedAttack (the gate fails for an assisting guard — swallowed, as in the game);
  impacts → HandleImpact(resume 0x56, allow 1) via `impact_args`; 0x11/0x12/0x14/0x15/0x16/0x1e →
  EnemyAlerts (the port's shared SCREAM/SHOUT/SOUND/ALARM routes, the state added to their
  patterns); 0x21 → ConsiderExplosive(0x56) (the shared EXPLOSIVE route).

### 3. The barks: the complete trigger table and the new ports

Already ported (WORKLOG 0069, unchanged): PainTalk (hit), DeathTalk (dying), AttackTalk (attack
entries), SeenPlayerTalk (spotting Bond), PatrolTalk. This task adds the rest of the family:

- `DroneFunc_HeardNoiseTalk` (0x149920): from the HeardNoiseSuspect (noise < 3) and
  HeardNoiseAlert enters, when no drone saw the opponent last frame — **type 1 says 0x26**
  (sfx 114, no roll, entry has no facial anim [L 0x2812d0+0x26·32]). Type 2 (07000005): silent.
- `DroneFunc_NewSightTalk` (0x13f420) / `LostSightTalk` (0x13f550), from
  `DroneVision_ProcessDroneSight` 0x176a28:204-236 with the global latches `DAT_00298fa8/fa9`
  (port: `sight_bark_latch`/`lost_bark_latch` on `Drones`): only voice types 5/6 on levels
  {0700000c, 070000d} speak (0xd6+Rand(2) / 0xd3+Rand(3), the LostSight twin needs MP == 0 and
  the player as the drone's target). **On 07000005 both return before any draw** — the pass only
  churns the latches.
- **The MP chatter family** — `InjuredTalk` 0x13fa20 (BulletImpact survival), `RunForCoverTalk`
  0x13fd08 (RunForCover enter with a claimed cover node `+0x6F4`), `UnderCoverTalk` 0x13fe80
  (UnderCoverInit/Return enters), `UnderCoverAimTalk` 0x13fb98 (UnderCoverAim enter): **all four
  open with `MPSettings+0x184` (0x2a4924) ≠ 0** — [M disasm of each: `lw v0, 0x4924(v1)` then
  `beq v0,zero` → return] — while `SpeechSFX` refuses when it IS non-zero [M 0x141b00], so the
  family is **dead in both modes**; 0 in single player [L ee_start_05.bin @0x2a4924]. The port
  models the gate (`mp_gate`, constant false) ahead of everything — no draws, no handle clears —
  with the exact per-level line tables in `mp_chatter_talk` (Injured: 0c/0d 0xcc+Rand(6),
  11/12/13/4a 0xb9, 14–16 0x133+Rand(5); RunForCover: 0c/0d 0xd0+Rand(2), 11–13/4a 0xbd, 14–16
  0x11b+Rand(7); UnderCover: 0c/0d 0xcf+Rand(3), 11–13/4a 0xbd+Rand(2); UnderCoverAim: 0c/0d
  0xd0+Rand(2), 11–13/4a 0xba+Rand(2); the 14–16 alt halves 0x138+/0x122+ need the unmodelled
  `+0x108 & 1` bit) and the call sites wired (RunForCover/UnderCoverInit/UnderCoverAim/
  UnderCoverReturn enters + the BulletImpact survival path). The audit's "5 s global throttle"
  (`DAT_00298fc8 = now + 5·FRAME_RATE_INT`) is **write-only**: the only decomp hits on 0x298fc8
  are these four writers, no reader [M].
- **No bark exists on reload**: the reload states (`NDrone2_DSTATE_SniperReload` 0x15ff80,
  `NDrone2_DSTATE_UnderCoverSniperReload` 0x16a8b0) call no talk function [M decomp scan]; the
  task's "reloading" item is satisfied by the table itself.
- Voice banks / sound ids [D]: the 07000005 bank is SB_4 (hash 5); the spot-bark entries
  0x49..0x4e are streamed ids 191/192/194/195/186/178, the attack entries 0x37..0x3f stream ids
  175..198, pain 0x13 → bank sample 138, death 5 → 139 (docs/systems/speech.md §3, unchanged).

## Verification

- New tests `crates/nf-game/tests/guard_barks_live.rs` (opt-in NF_ISO, silent via NF_SILENT) —
  **4 passed**:
  - `spot_bark_plays_once_within_cooldown`: Bond steps into the gate guard's view; exactly one
    SeenPlayerTalk line (seed 2: entry 0x4b, sfx 194 at tick 59) inside the first sighting
    window, after the 30-frame post-load gate; re-barks only across full lose/regain cycles
    (observed at 1032/1269, each ≥ 16 ticks apart).
  - `first_alert_routes_through_state_0x39`: the forced attack routes 0x56 → 0x39, drawing
    exactly once (the ENTER's Rand(120)); the timer sits inside the 120-tick window;
    SetInterogateAssist's 0x20 goes up; TIMER_C lands in 0x3a (both the unreachable-goal else
    branch and, with Bond stepped to 8 units, the within-10 branch); the assist holds while the
    source keeps only the interrogate markers and ends into Attack with `EndInterogateAssist`'s
    `(& 0xffdfffdf) | 0x10` bits on the exit.
  - `first_alert_suppress_bits_fall_through_to_attack`: a source-attacked alert (0x100000) is
    disqualified — the normal FirstAttack tail runs.
  - `mp_chatter_is_silent_in_single_player`: the four chatter sites draw **zero** RNG and queue
    no chatter line; a survived bullet never queues one.
- `cargo check --offline -p nf-game` clean; the whole touched crate's tests: 470/0/2 (below).

## Test-suite runs (what I ran)

- `NF_ISO=$NF_ISO cargo test --offline --release -j2 -p nf-game -- --test-threads=1` — the whole
  touched crate on the final tree: **470 passed / 0 failed / 2 ignored** (55 test binaries; exit
  0; log `nf-game-test.out`, not committed).
- The new file itself: `guard_barks_live` 4/4 (included in the total above).
- The fast runner `tools/test/suite.py` was started three times in the first minutes (21:41,
  21:54, then detached via setsid) — **every background launch was killed by the sandbox between
  tool calls** (suite.out empty, no process), so no suite.py line exists for this session.
- The suite's other three crates were attempted in the foreground
  (`cargo test --offline --release -j2 -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`,
  with the user-prefix ALSA at `PKG_CONFIG_PATH=/home/z/alsa-prefix/...` and
  `RUSTFLAGS="-L .../alsa-prefix/usr/lib/x86_64-linux-gnu"`): they do not link in this sandbox —
  the lld link of the wgpu-sized nf-view test binaries dies with `collect2: fatal error: ld
  terminated with signal 7 [Bus error]` (the `-L` path IS on the linker line, so the library is
  found; the crash is environmental). Those crates are **untouched by this diff** (drone.rs + one
  new test file only); the next message on this branch should merge `origin/master`, re-run the
  suite, and report the total line.

## What's left

- The full-suite line on the final tree (the runner above; merge `origin/master` first).
- The GoToGoal branch of the assist walk (TIMER_C with a *reachable* >10-unit goal) is asserted
  only conditionally (return state 0x3a when the walk passes through GoToGoal); a map-fixed
  Bond spot 10–20 units from the gate guard but outside his view would pin it deterministically.
- The 14–16 chatter alt halves (the `+0x108 & 1` second-weapon-set bit) are unmodelled — no
  ported drone carries the bit.
- Live-check candidates for the main agent (below).

## Live checks for the main agent

1. **07000005, first alert**: start the level, let the first guard spot Bond at the courtyard
   (67.31, 26.5, 99.47); `gdbwatch` a *second* guard's `Drone+0x4FC` and `Drone+0x10C`.
   Expected: when the receiver's +0x4FC gains 0x10000..0x300000 (the shifted copy), his state
   (+0x10C) passes 0x56 → 0x39 within ~2 s if a guard was interrogating, else the combat states.
   On 05 without an interrogator the 0x200000 flag never rises, so 0x39 should NOT appear — the
   route exists for the interrogate levels; if 0x39 does appear on 05 naturally, my
   "who sets 0x20 on 05" analysis is incomplete.
2. **07000007 bouncers**: after Bond draws a gun at a bouncer mid-interrogation, `gdbwatch` a
   second bouncer's +0x4FC (expect 0x200000 set via ConsiderAlerted) and +0x10C (expect 0x39 →
   0x3a within Rand(120) ticks, i.e. ≤ 2 s), then 0x56 when the interrogator stops.
3. **fieldprobe**: poke a guard's `+0x4FC = 0x200000`, `+0x4F8 |= 0x20000000`, `+0x170` = the
   player obj, then send msg 0x1f — expected: state 0x39, +0xCC4 (timer C) = now + ≤ 120, and
   `+0x4FC |= 0x20` on the enter.

## Usage

not visible to me — this sandbox exposes no token, context-window or cost figures for the
session; no numbers to copy.
