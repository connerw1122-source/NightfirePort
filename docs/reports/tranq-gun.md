# tranq-gun — the Korsakov K5's sleep dart

The user, 2026-10-16 evening: "tranq gun doesnt work". Done: the dart hit ran the plain-bullet
route (a 0.1-damage flinch + alert); the game's route — the K5's weapon records 0x43/0x44 → msg
0x19 → the StunDart family 0x50/0x51/0x52, no damage, a 120 s sleep — is ported and the real-path
test that failed on master now passes.

## What the game does (full map in docs/systems/tranq-dart.md)

- **The weapon**: Korsakov K5, records 0x43 (base) / 0x44 (upgrade twin), name label 0x05000041,
  ammo 18 "7.62mm dart", clip 5, damage 0.1 (dead — never applied), projectile mesh 0x020008bd
  [L ee_start_05.bin @ 0x2bf150 + 0x43·0x10C]. Carried ONLY on 07000009/0a/0b (Night of the Agent,
  Tower A/B/C) as the START LOADOUT with 999 rounds, selected [M Player_InitWeapon 0x1a7ef8 case
  0x7000009; the port's fresh_09]. No tranq pickups on any story archive; the fp skin 0x050000a3
  resolves only on those three [D archive scan].
- **The hit route** [M]: `NDrone2_DealWithObjHit` 0x13fff0 case 5 (:269-306) — the bullet's weapon
  record picks the msg: 1|0x4b|0x4d → 6, 0x4a|0x4c → 7, **0x43|0x44 → 0x19**, else 8
  (`Drone_BulletHit`). The dart route never calls `NDrone2_HitDamage` 0x145558 — the 0.1 damage is
  vestigial. `DroneFunc_HandleImpact` 0x1469b8 case 0x19 (:315-361): `PlrStat_LogShotHitEnemy(0)`
  (unconditional — no team gate), the resume stamp +0x118, the **Tower band** (levels 09/0a: the
  resume slot SMI+0x10 = 0x14 CivilianPatrol unsighted / 0x15 ReturnToPatrolPath sighted
  [:331-357] — the tower guards are team 3, `IsCivilian` = team != 1 [M 0x14aae0], so THIS resume
  is what their wake follows), then SetState(0x50).
- **The states** [M]: 0x50 StunDartImpact 0x163430 (RecoverTime → +0xCE0; the weapon drop for
  +0xDA ∈ {5,6,9}; AlertStatusSet(0) — fully calm; the sleep-fall anims 0x2B/0x2C → 0x51),
  0x51 StunDartLoop 0x163570 (down flag +0x2F; wake timer +0xCC4 = now + RecoverTime; punch →
  PainTalk + 0x42 KnockedOut_Anim; msg 7/0x19 re-arms; TIMER_C → 0x52 unless
  `DroneFunc_AnyoneNear(2.0)` 0x14a718 holds him; blind/deaf/unalertable otherwise),
  0x52 StunDartRecover 0x163700 (the get-up 0x76 → Attack 0x56 for team 1, the stamped resume for
  civilians; msg 0x19 re-sleeps).
- **The sleep length** [M 0x147188]: `FRAME_RATE_INT·k·0.01·(200 − +0xB9)`, 0x43 → k=60 → **7200
  ticks = 120 s** (0x44 → k=120 → 240 s; taser 1200; stun grenade 3600); `+0xB9 = 0` at run time
  [L all 13 drones in ee_start_05.bin]. The states WITHOUT case 0x19 (Attack 0x56 + the
  FirstAttack wrapper, COMBAT 0x58/0x5b, BulletImpact 0x55, the death/bot/ninja/astronaut
  families) drop the dart — a guard in full combat is immune [M state-table 0x29b300 × decompile
  cross-check: 89 of the port's 122 states carry it].
- **The drop bands**: +0xDA ∈ {5,6,9} ⟺ held weapon in {0x16,0x18,0x19,0x1a,0x1b} / {0x12,0x14,
  0x15,0x30,0x32,0x33,0x6a,0x6e} / {0x1c,0x1d,0x2a..0x2d} [M DefaultInit 0x14b300's second switch,
  keyed on the held weapon id; **[L] cross-checked live**: ee_start_05's drones read wep 22 (0x16)
  → +0xDA = 5, wep 36 (0x24) → +0xDA = 19]. Pistol-class (+0xDA = 4) and sniper-class (+0xDA =
  0x13) guards keep their gun.

## The port fix (crates/nf-game/src/drone.rs)

- `msg::STUN_DART = 0x19`; the bullet-hit routing sends 0x43/0x44 to it (the DealWithObjHit arm).
- `handle_dart` = the family: the 0x50 ENTER (RecoverTime → the new `dart_recover` (+0xCE0 stand-
  in), the drop, the mode-0x11/0x0C mission-fail arm with `mission_fail_reason(0xE)` — the
  default arm ch 99/no label [M 0x13e560], AlertStatusSet(0) → alert 0, the Rand(2) fall anims
  0x2B/0x2C → 0x51), the 0x51 loop (down, the timer, punch→0x42 with PainTalk, re-arms, the
  AnyoneNear gate), the 0x52 recover (get-up → Attack 0x56 for team 1, the resume for civilians).
- `on_dart_impact` = HandleImpact case 0x19: the LogShotHitEnemy outbox (`hit_log`), the resume
  stamp, the Tower band (0x14/0x15), SetState(0x50); non-carrying states return false → GLOBAL →
  dropped.
- `hit_recover_time` = RecoverTime 0x147188 (the weapon switch; the punch's class-5 gate is moot —
  every port bullet is class 5).
- The intercepts in `handle`: STUN_DART → handle_dart → on_dart_impact; the dart-family swallow
  block (a sleeper stays blind/deaf/unalertable); the TASE chain checks the dart loop first (a
  tase during sleep extends the sleep, not the accumulator [M 0x163570 case 7]); on_impact's
  stun-family arm covers the dart states (bullet → 0x44; punch in the LOOP → 0x42); on_flash
  drops a flash during the 0x50 fall (no case 0x18 there) and lets the loop/recover forward to
  0x4D like the game.
- New state `RETURN_TO_PATROL_PATH = 0x15` [M 0x172810: ENTER re-finds the path ([S] the
  ReFindMissionPath model) and falls to GO_TO_GOAL 0x63].
- New fields `dart_recover` (on its own lines, merge lesson 4), anims `DART_FALL_A/B = 0x2b/0x2c`.

## Reproduce-first (the failing run on master)

`cargo test --offline --release -p nf-view --test tranq_dart_live` on the pre-fix tree (the state
constants only added, behaviour untouched) — all 3 tests failed:

```
thread 'the_k5_dart_puts_a_guard_to_sleep' panicked at tranq_dart_live.rs:101:
assertion `left == right` failed: the K5 is the level's selected weapon at spawn [M 0x1a7ef8 case 0x7000009]
  left: 2
 right: 67
```
(the first run's assert rode the pre-draw tick — after riding the real loadout switch, the
honest master-behaviour failure was the sleep assert: `the dart hit puts the guard into
StunDartImpact 0x50 / StunDartLoop 0x51 [M 0x13fff0 case 5 …] — states seen: [(0x12, 137), (0x3f, 272)]`
— the guard flinched/alerted, never slept; the sisters failed the same way). On the fixed tree:
**3/3 pass** (`the_k5_dart_puts_a_guard_to_sleep`, `a_second_dart_extends_the_sleep`,
`a_punch_puts_a_sleeper_out_for_good`).

## Verification

- `nf-view --test tranq_dart_live` 3/3 — the real path on 07000009: the level's own loadout
  switch to the K5, a guard found in the llist [L dump: 6 team-3 guards, the car-park one at
  (−4.36, −376.08, 35.69)], Bond snuck 2.5 u behind via `Dev::teleport` (the merged stun tests'
  setup helper — the equip/aim/fire/flight/hit are the game's own paths), fire held, then:
  state 0x50 → 0x51 with `down`, health UNCHANGED (no HitDamage), `timer_c = now + 7200` (the
  K5's 120 s), the wake at the expiry into 0x52 → get-up → CIVILIAN_PATROL (the Tower band's
  stamped resume); sister: a second dart re-arms the countdown UP (fresh 7200); sister: a punch
  on a sleeper → 0x42.
- Regressions: `nf-game --lib` 251/251; `nf-game --test drones` 2/2; the taser family
  `stun_ai_recover_live` 3/3, `fix_stunner_press_live` 2/2, `tase_silent_live` 2/2.
- **The full suite did NOT run** — this sandbox kills background jobs between tool calls (the
  baseline `nohup python3 tools/test/suite.py` died silently twice) and the 2-hour window closed
  before a 13-minute foreground run could fit after the fix landed. The touched-crate binaries
  above are the verification that exists. Next session: run the full suite on this branch.
- Sister-assertion honesty: every assertion carries its [M]/[L] address; the wake-timer assert
  pins 7200 exactly (±5 slack for the frame capture), the health assert pins the NO-damage route,
  the wake assert pins the Tower band's 0x14.

## What's left / deviations (all [S], kept out of pass/fail)

- The Tower band's sighted-arm plumbing (+0x5a4=0x89, +0x16=0, +0x4fc=0, the +0xc48 scratch,
  +0xc5=9, +0xc6=7) is unmodelled — the same block the port's flash path already skips
  (fix-guard-drops-flashbang precedent). The port stamps `impact_resume` = 0x15 instead; the
  observable wake difference is nil for non-civilians.
- `ReFindMissionPath` 0x153a30 is not modelled — RETURN_TO_PATROL_PATH falls straight to
  GO_TO_GOAL [M 0x172810 minus the path refind].
- The +0xD8 skin gates on the drop classes (0xb/0x18 skins) are unmodelled — no tower skin hits
  them in the [L] data I could read.
- `+0x4F8 |= 0x10000` at the 0x52 enter (the game's vision re-enable) not modelled, matching the
  taser recover precedent.
- Live checks for the main agent (PCSX2, fresh boot, 07000009): (1) tranq a car-park guard,
  `watch drone+0xCC4` on the dart hit → expect +0xCE0 = 7200 written then +0xCC4 = frame+7200;
  (2) shoot a lobby guard already in full combat → expect NO state change (dart dropped);
  (3) the sighted wake: alert a guard, de-alert, tranq → the get-up ends in 0x15 → 0x63.

## Usage

- Input tokens: not visible to me (no token/credit figures are shown in this environment).
- Output tokens: not visible to me.
- Context used: not visible to me.
- Cost: not visible to me.
