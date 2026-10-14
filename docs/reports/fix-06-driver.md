# fix-06-driver — report

Branch `s/fix-06-driver` (the task card's bottom said `s/06-driver`; the header and rule 1 say
`s/fix-06-driver` — that is the branch pushed here, cut from `origin/master` 7785582). The task: the
playtest 1011-1535 (06) report on the friendly truck driver (07000006, skin 0x05000035, dtype 13 →
dtypenew 0x17, team 3): "he does get scared, but because of where he's standing, he like crouches on
top of the boxes that he's next to... Also, you can still shoot him, and he like flies back like a
guard would with the same guard death sound." Check both against the game, port the differences.

## What the game does (the evidence chain)

**The driver is damageable — shooting him is game-accurate.** `NDrone2_DealWithObjHit` case 5 (the
bullet, 0x13fff0:181-235): the team gate (`drone+0x44` 2..3 vs the shooter's) only applies when the
SHOOTER is a type-2 drone; the player's object type (3/0x36) hits unconditionally [M]. The port
already matched this (fix-drone3's reading), and the test pins it: the shots kill him.

**The driver's states have NO msg-0x14 (SOUND) arm — gunfire sounds never move him.** Read whole in
the decompile:

- `TruckDriverIdle` 0x160ed0: msgs 0x11/0x12/0x15/0x16/0x1e → `DroneVision_EnemyAlerts`; msg 0x14
  falls to the default (unhandled) [M 0x160ed0:95-103]. His scare paths are NearArmedBond (case 3,
  UPDATE), the bullets (case 8 → `HandleImpact(0x12, 0)`), the screams/shouts and the alarm.
- `TruckDriverInit` 0x160bd0: msgs 0x11/0x12/0x13/0x16/0x1e → 0x33 (InitAlert); NO 0x14 arm [M
  0x160bd0:97-105].
- `TruckDriverInitAlert` 0x160d98: cases 0/1/2/5/6/7/9/0x17/0x18/0x19/8 → `HandleImpact`, 0x21 →
  `ConsiderExplosive` — no sound message at all [M].
- (The party-family states DO handle 0x14 → EnemyAlerts — PartyGirl 0x160140, CivilianGuard 0x1603a8,
  CivilianMission 0x15d860, CivilianPatrol 0x15d610, Civilian 0x15d118 — the port already matched
  those and they are untouched.)

The port had `msg::SOUND | msg::ALARM` arms on all three driver states: every gunshot's sound alert
(`DroneFunc_HandleSoundAlerts`' msg 0x14) put the driver into the GUARD heard-noise chain (0xa2-0xa5),
whose attack path sent him to `DRAW_WEAPON` (0x66) — whose EXIT arms him (weapon 6) and switches his
anim set 12 → 4, the guard's set [M drone.rs DRAW_WEAPON EXIT]. From there his death played the
guard set's death anim — the playtest's "flies back like a guard". The probe showed exactly that
chain (state 0xa3 → 0x66, aset 12 → 4) before the fix.

**The death line.** `NDrone2_DeathTalk` 0x13eff8 switches on the drone's voice type: the guards'
(type 1) get line 4; the driver's type is 0xf (skin 0x35 [M 0x14b300]) → the default arm → line 1
[M 0x13eff8 default]. This level's voice bank gives lines 1 and 4 the SAME sfx (139) [L d.voices] —
the game itself gives the driver the "same guard death sound"; the sound was never the deviation.
The death ANIM (the guard set's fly-back, reached through the weapon draw) was.

**The cower.** The scare chain: `TruckDriverIdle` case 3 NearArmedBond → CivilianScared 0x12 [M
0x160ed0:60-70] whose ENTER — at alt mode ≠ 7; the driver's alt mode is 0 [L probe] — goes straight
to CivilianHiding 0x13 [M 0x15d3a0:31-43], whose ENTER plays the cower 0x19 [M 0x15d520:17-22].
The cower anim for his anim set is danim 159 → script 060000cd → seq 04000048, whose root track is
constant in x/z [D nf-extract: root first (−0.044, 0.566, 2.511) → last (−0.044, 0.566, 2.511)] —
he cowers IN PLACE. The port does exactly that (measured: dpos 0.000 through the cower). The
"he moves or crouches on the floor" half: the game's `Drone_FeetOnPoint` 0x140660 down-ray + the
port's `pos.y = floor + hip 1.0` keep his feet on the courtyard floor (floor 38.0 at his spot — the
floor-grid probe). The user's "crouches on top of the boxes that he's next to" was the OLD build:
his start script played under a no-gravity stand-in that let him drift onto the crate stack by the
gate truck's grille; fix-drone3 already restored the game's gravity (the "holds his truck-bed
height" commit), and the current build's scared pose sits at floor level. The test pins it.

## The fix

`crates/nf-game/src/drone.rs` (the only source file touched):

- `(TRUCK_DRIVER_INIT, msg::SOUND | msg::ALARM)` → `(TRUCK_DRIVER_INIT, msg::ALARM)` [M 0x160bd0:97-105].
- `(TRUCK_DRIVER_INIT_ALERT, msg::SOUND)` arm dropped — the game's InitAlert handles no sound msg [M 0x160d98].
- `(TRUCK_DRIVER_IDLE, msg::SOUND | msg::ALARM)` → `(TRUCK_DRIVER_IDLE, msg::ALARM)` [M 0x160ed0:95-103].

An unhandled msg returns false from `handle()` (the port's existing default), which is the game's
`default: return 0` — the driver ignores sound alerts exactly like the decompiled states.

## The verification

`crates/nf-view/tests/fix_06_driver_live.rs` (new), through the real paths (`nf_view::load` +
`Sim::step_bond`, tracked-aim P2K shots with the semi-auto pull and the clip refill):

- `gunfire_never_arms_the_driver_and_his_death_is_his_own` — shoots him dead from the open south
  side and tracks every tick: he never enters DrawWeapon 0x66, never the heard-noise chain 0xa2-0xa5,
  keeps anim set 12, dies (damageable — the game's own behaviour), his death sound is line 1's (sfx
  139, the voice-type-0xf default), and his protected-actor death raises ch 0x60 ("Killed Civilians"
  [M OnInitDeath 0x145bd0 reason 3]). **Shown failing on the pre-fix tree** (stash-verified): it
  fails at the heard-noise assert — the exact playtest mechanism. Sister: a team-1 guard in the same
  level still reacts to the same gunfire (the sound dispatch is intact for the states that have the arm).
- `the_scared_cower_keeps_him_in_place_on_the_floor` — armed Bond in his facing cone → 0x12/0x13
  with the cower 0x19; the cower happens where he stands (moved < 0.05); his feet stay on the
  courtyard floor (`pos.y − 1.0` = the down-ray floor ±0.05); sisters: alive and in the civilian set.

Both pass on the fixed tree (2/2). The full suite ran twice: pre-fix tree **1048 passed / 7 failed /
33 ignored (37 min)**, and again on the final merged tree — see below. The 7: the 3 `disc_golden`
(allowed on the slim ISO), `fix_0c_pylon_live` ×2 and `probe_pylon_render` (the 0c-pylon render
checks — another task's files, failing here on master too), `weapon_matrix_live` TIMEOUT (the
weapon-matrix sweep exceeds the 600 s cap in this sandbox — pre-existing). None touch drone.rs or
this task's test.

## Renders (and their gap)

The change is visible (the driver's reaction to gunfire), so I rendered before/after pairs with the
headless `--shot` path (lavapipe): `/tmp/driver_before*.png` / `driver_after*.png` at the truck park
spots (66.66, 39.2, 34.0) yaw π and (69.5, 40.3, 27.0) yaw −1.03, frame 1150 with `--pull 1000`.
**In every headless shot the driver does not draw at all** — present/started/enabled/llist all true,
`drawn` false (the visibility stand-in finds no angle: the crate stack by the truck's grille
occludes him from every ground spot; from directly overhead the renderer still shows nothing). The
tests carry the state-level before/after instead (the fail-first run above). The render gap is
reported below for the main agent's GPU pass.

## Live checks for the main agent

1. **PCSX2, the no-draw confirmation** (the fix's core): 07000006, arrive at the truck park (the
   plates raise ch 185), stand armed within 6 units of the driver, fire PAST him. Expected: he gets
   scared (cowers, hands over head) and NEVER draws a weapon. With gdbwatch on the driver object's
   `+0x10C` (state): never 0xa2-0xa5, never 0x66 during gunfire. In the pre-fix port he drew the
   weapon every time.
2. **PCSX2, the death**: shoot him dead. Expected: the civilian death anim (anim set 12's 0x40/0x42
   — no guard fly-back) and the death line 1 (sfx 139 — audibly the same grunt as a guard's line 4;
   that sharing is the game's own [L d.voices]).
3. **PCSX2, the step-out window** (pre-existing open question, not changed by this task): the
   driver's start script 0600011d is 880 frames; the body track's root steps 0.57 in frames ~720-880
   [D]. The [L] recording covers only 0-700 (he holds the seat). The port applies the step (face
   tracks are excluded from the root source) — he ends standing by the crate stack at (66.66, 38.0,
   28.70). Check frames 700-920 on hardware: does the game's driver step out of the cab, and does he
   land on the ground (38.0) or on the crates?
4. **nf-view --bond spots** (GPU): 07000006 `--skip-cutscenes --frame 1050 --pull 1000` at
   (66.9, 41.6, 31.0) yaw π pitch −0.62 (overhead of the driver's spot) and (66.66, 39.2, 34.0) yaw π
   (ground south): expected — the driver by the crates at the truck's grille; scared after the pull,
   cowering in place, unarmed. The headless path never drew him; confirm the windowed build does
   (if not, that is a render-task bug: present/enabled/llist all true, `drawn` false by the stand-in).

## What's left

Nothing on the task's two symptoms: the guard-set fly-back/death is fixed at its source (the
game's absent msg-0x14 arms), and the cower is verified in place on the floor at the game's
position. The renderer gap (the driver invisible in headless `--shot`) and the 700-880 step-out
window are reported above for the main agent.
