# The tranquilizer dart (Korsakov K5) — the sleep path

The user, 2026-10-16 evening: "tranq gun doesnt work". The dart hit ran the plain-bullet route
(0.1 damage + a flinch); the game's route puts the guard to sleep. This doc maps the whole
mechanism; the port's gap and the fix are in `docs/reports/tranq-gun.md`.

## The weapon

| fact | value | tag |
|---|---|---|
| records | **0x43 Korsakov K5** (base) / **0x44** (the upgrade twin — `DartGuns 0x2bee68 = [0x43,0x44,0x44,0x44]`, `Upgrade_Weapon(0x43)` with `Upgrades=0` → 0x43) | [L ee_start_05.bin @ 0x2bf150 + 0x43·0x10C; [M] crate pickup.rs:443-448] |
| name label +0x38 | 0x05000041 → "Korsakov K5" (USATxt) | [L + nf_view lang.bind] |
| damage +0x0C | 0.1 (vestigial — never applied, see the route) | [L] |
| ammo type +0x90 | 18 ("7.62mm dart", label 0x05000024) | [L] |
| clip +0x92 / rate +0x40 | 5 / 20 (0x44: 30) | [L] |
| speed +0x20 / range +0x1C | 1.5 (0x44: 2.0) / 45 (0x44: 100) | [L] |
| projectile mesh +0x5C | 0x020008bd (the dart) | [L] |
| bullet_flags +0x6C / +0x70 | 0xA00 / 0x2 — **+0x70 bit 0x2 is NOT the sleep trigger**: `Bullet_CollisionHandler` 0x12ce80:183-185 turns it into `Effect_Create` flags 0x61 (the world-impact bundle; the P2K carries 0x12 and nobody sleeps from it) | [M 0x12ce80:183-192] |
| carried where | the START LOADOUT of 07000009/0a/0b only (Night of the Agent, Tower A/B/C): `Player_InitWeapon` 0x1a7ef8 case 0x7000009 equips `Upgrade_Weapon(0x43)` with 999 rounds and SELECTS it; no tranq pickups exist on any story archive; the fp skin 0x050000a3 resolves only on 09/0a/0b's own packs | [M 0x1a7ef8 case 0x7000009; [D] archive scan (weapon-model-coverage.md); [L] probe] |

## The hit route — the game's [M] chain

1. `Bullet_update` 0x12e7b0 flies the dart → `Collide_Jointy` 0x1ec180 builds the HITDATA.
2. `Drone_CollisionHandler` 0x140968 → `NDrone2_DealWithObjHit` **0x13fff0 case 5** (:269-306):
   the weapon-id switch on the bullet's record — `1|0x4b|0x4d → msg 6`, `0x4a|0x4c → msg 7`,
   **`0x43|0x44 → msg 0x19`** (:278-285), else `Drone_BulletHit` (msg 8). The 6/7/0x19 paths bump
   the hit count `+0x1D8` and stamp last-hit-by `+0x2B4` first. **`NDrone2_HitDamage` 0x145558 is
   never called on the dart route** — the record's 0.1 damage is dead.
3. `Drone_Message(0x19, HITDATA, 0)` → the SM routes it: the carrying states' case 0x19 forwards to
   `DroneFunc_HandleImpact`; the non-carrying states (ATTACK 0x56 and the whole FirstAttack wrapper
   0x164210, COMBAT 0x58, COMBAT_NEW_SIGHTING 0x5b, BulletImpact 0x55, death/stun families, the
   bots/ninjas/astronauts, WaitSwitch, GLOBAL) return unhandled → GLOBAL 0x15b830 → dropped. The
   carrying set (state-table cross-check, fn 0x29b300 vs the decomp, 2026-10-17): IDLE/ALERT/PATROL,
   PLAY_SCRIPT, the hostage/civilian/ally/mission/sniper/aim-combat-move/under-cover families,
   SURRENDER_*/UNSURRENDER, PUNCH_IMPACT 0x53, STUNNED 0x4b, STUNNED_RECOVER 0x4c, STUN_GRENADE_LOOP
   0x4e, STUN_GRENADE_RECOVER 0x4f — 89 of the port's 122 states. **A guard in full combat ignores
   the dart.**
4. `DroneFunc_HandleImpact` **0x1469b8 case 0x19** (:315-361):
   - `PlrStat_LogShotHitEnemy(0)` — the accuracy numerator, UNCONDITIONAL (no team gate; unlike
     `NDrone2_HitDamage` 0x145558:57-60's team-1 gate on the msg-8 route) [M 0x1469b8:316; 0x199840];
   - the resume stamp `+0x118 = current state` when valid (≠0, not 0x53..0x55) [M :317-330];
   - **the Tower band** (`GameState._12_4_ + 0xf8fffff7 < 2` ⟺ level ∈ {0x07000009, 0x0700000a};
     0x0700000b fails the `+1 < 2` test) [M :331-357]: `+0x4F8 & 0x4000000` clear (never sighted) →
     `SMI+0x10 = 0x14`; set → the alert-plumbing block (+0x4f8 &= ~0x02000000, +0x5a4=0x89, +0x16=0,
     +0x4fc=0, the +0xc48==100 scratch, +0xc5=9, +0xc6=7, +0x4f8 &= ~0x04000000, +0x5a2=0x14) and
     `SMI+0x10 = 0x15`. `SMI+0x10` is the impact-resume slot; the dart's recover state hardcodes
     Attack 0x56 for non-civilians, so the 0x14/0x15 write is dead for the tower guards (the port
     stamps `impact_resume` with it — see the deviation note in the report). The IDENTICAL block
     sits in case 0x18 (:280-312) — the port's flash path skips it too (fix-guard-drops-flashbang).
   - `Drone_SM_SetState(SMI, 0x50, 0)` [M :358-361].

## The sleep state family (states 0x50/0x51/0x52)

State table `NDrone2_StateFuncs` 0x29b300: 0x50 → 0x163430, 0x51 → 0x163570, 0x52 → 0x163700
([L] table dump; docs/fields/dronetype-tables.md:263-265).

**0x50 StunDartImpact** [M 0x163430]:
- ENTER: `+0xCE0 = DroneFunc_RecoverTime(HITDATA)`; the weapon drop — `+0xDA < 5` keep,
  `+0xDA ∈ {5,6} ∪ {9}` → `DroneWeap_DropWeapon` + `ChangeWeapon(1)`. `+0xDA` is DefaultInit's
  anim-set byte (0x14b300 second switch, keyed on the held weapon id): pistol class
  {2,6,10,0xb,0xe,0x10,0x3b..0x40} → 4 (KEEP — the same class the taser's `+0xDA==4` keeps),
  {0x16,0x18,0x19,0x1a,0x1b} → 5, {0x12,0x14,0x15 (+0xd8≠0xb), 0x30,0x32,0x33,0x6a,0x6e (+0xd8≠0x18)}
  → 6, {0x1c,0x1d,0x2a..0x2d} → 9, {0x1e,0x1f,0x24,0x25} → 0x13. **[L ee_start_05.bin]**: the Paris
  guards read wep 22 (0x16) → +0xDA = 5, wep 36 (0x24) → +0xDA = 19 — the switch's 5 and 0x13 arms
  both confirmed. (mode +0xC5 == 0x11 or 0x0C → `SetMissionFailReason(0xE)` → default arm: ch 99,
  NO label [M 0x13e560 — 0xE is not in the 0x4000033 list]); `AlertStatusSet(0)` — the guard goes
  fully calm (the taser's arm is 2); the sleep-fall anim `Rand_Rand(2)`: 0 → **0x2C**, else **0x2B**
  (the dart-only anims — no other DSTATE calls them), follow-on state 0x51.
- msg 8/9 → Death_Anim 0x44. msgs 0/10 → handled-noop. everything else → unhandled → dropped.

**0x51 StunDartLoop** [M 0x163570]:
- ENTER: `+0xCC4 = frame + +0xCE0` (the wake deadline), `+0xCC8 = 0`, `+0xCC0 = 1` (TIMER_C armed),
  **`+0x2F = 1` (the DOWN flag)**. EXIT: `+0x2F = 0`.
- msg 6 (punch): `NDrone2_PainTalk` + **0x42 KnockedOut_Anim** — punching a sleeper puts him out
  for good (the Surrender path's shape).
- msg 7 / 0x19: re-arm `+0xCC4 = frame + RecoverTime(HITDATA)` — a second dart (or a tase) extends
  the sleep from NOW.
- msg 8/9 → 0x44.
- **msg 0xC (TIMER_C)**: `DroneFunc_AnyoneNear(2.0)` [M 0x14a718] false → 0x52 Recover; true →
  re-arm `frame + FRAME_RATE_INT·5` (300 ticks) and keep sleeping — Bond (or any drone) loitering
  within 4.0 xz / 3.0 y holds him under.
- msg 0x18 → `HandleImpact(0x18)` → the flash family 0x4D (the 0x50 state does NOT carry 0x18 —
  a flash during the fall is dropped).
- everything else unhandled → dropped: **asleep = blind, deaf, unalertable** (`AlertStatusSet(0)`
  cleared the alert status; the state has no vision arm).

**0x52 StunDartRecover** [M 0x163700]:
- ENTER: `+0x4F8 |= 0x10000`; non-civilian → get-up `CallAnim(0, 0x76, 0, 0x56)` — **wakes straight
  back into combat**; civilian → follow-on `SMI+0x10` (the resume slot).
- msg 7/0x18 → `HandleImpact` (a tase re-runs the accumulator; a flash re-stuns); msg 8/9 → 0x44;
  **msg 0x19 → back to 0x50** (a dart during the get-up re-sleeps him).

## The sleep length — `DroneFunc_RecoverTime` 0x147188

`FRAME_RATE_INT · k · 0.01 · (200 − Drone+0xB9)`, the weapon switch on the HITDATA's bullet record
(gated on the bullet obj class 5): **0x43 → k=60 (0x3c), 0x44 → k=120 (0x78)**, 0x35 → 30,
0x4a/0x4c → 10, default → 60. `FRAME_RATE_INT = 60` and `+0xB9 = 0` at run time
([L ee_start_05.bin: all 13 drones +0xB9 = 0]):

| hit weapon | sleep |
|---|---|
| K5 0x43 | 7200 ticks = **120 s** |
| K5 0x44 | 14400 ticks = **240 s** |
| taser 0x4a/0x4c | 1200 ticks = 20 s (the port's `tase_recover`) |
| stun grenade 0x35 | 3600 ticks = 60 s (the port's `stun_until` arm) |
| default | 7200 |

The timer engine: `NDrone2_PreDroneControl` 0x148db0:97-111 fires msg 0xC when armed and the frame
passed the deadline; `RouteMsgDCV` 0x171df8:185 disarms on every state change.

## The port

`crates/nf-game/src/drone.rs` today: the msg set has no 0x19; the bullet routing (:8282-8284) sends
a K5 dart down msg 8 — 0.1 damage, a flinch, an alert. No states 0x50-0x52 (the `down` field's doc
already lists `DSTATE_StunDartLoop` 0x163570 as "unported"). The taser family (0x4a-0x4c) and the
stun-grenade family (0x4d-0x4f) are the ported precedents this fix mirrors.
