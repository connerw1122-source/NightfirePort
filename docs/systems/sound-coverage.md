# Sound coverage: every ACTION.ELF sound call vs the port

Audit (audit-sound-calls, 2026-10-15). Every call site of `Sound_Play3D`, `Sound_Play`,
`Music_Event` and the ambient/loop starters in ACTION.ELF, with the port's status. The site list
is mechanical: a `jal`-scan of the ELF's .text (script `scripts/sound_call_scan.py` in the
session sandbox) cross-checked against the decompile's call lines (`work/decomp/ACTION.ELF`).
**195 direct call sites** in 46+28+28 caller files. The decompiler scrambles the float-taking
prototypes (`Sound_Play3D(uint id, VECTOR* pos, float, float, float, short, uint, int)` prints
the floats first), so every id below was read from the **disassembly's `$a0`** / the decomp call
line's id slot; `Sound_Play3D`'s constant pattern `(0x42c80000,0xbf800000,0xbf800000,ID,pos,0,0,0)`
is 100.0f/−1.0f/−1.0f/ID [M dis].

Statuses: **PORTED** (the port plays it — cite), **MISSING** (no port path), **N/A** (MP/driving/
front-end only). Ported-this-pass items link the test (`crates/nf-game/tests/sound_coverage_live.rs`).

## Summary

| Class | Sites | Status |
|---|---|---|
| EE pool + map ambients + script sounds + music engine (`Sound_UpdateSounds`, `HandleMapSoundAllocation`, `Sound_ReqestPlaySfx`, `Script_*`, `UpdateMusicalEvents`) | 30 | PORTED (`nf_game::sound`, `sound.rs` MapSounds, `music.rs`) |
| Player on-foot (jump/pain/hit/air/weapon-switch/death) | 40 | PORTED except the weapon-change 0x1DB (level-gated) |
| Weapons/bullets (fire, impacts, fly-by, casings) | 25 | PORTED except 0x28D (class-0x51 bullet-end) |
| World objects (doors, searchlights, pickups, hints, sound triggers, locks) | 30 | PORTED except the picklock beeps 0x5DB/0x5DA + activate sounds, `Hurt` loop |
| Drones/guards (foley, pain, gunfire, deaths, seen-music) | 25 | PORTED except class-specific extras (below) |
| Security cameras (`Sensor_Update`) | 7 | MISSING — the class (0x29) is unported whole |
| SpaceLaser (0x50), SS (0x25), Break (0x20) classes | 10 | MISSING — classes unported whole |
| Sky thunder (`View_DrawSky`) | 2 | MISSING — the lightning tick (`sky.rs`) is not wired into the sim (G-019) |
| Front-end menus (`P_*_Handler`, `Menu_PlaySound`) | 40 | N/A (front-end) |
| Vehicles/driving/MP (`Car_*`, `Copter_*`, `Sub_*`, `GT_*`, `MPSound_*`) | 22 | N/A |

**Ported this pass (5 ids, 4 hooks, each with an NF_ISO test):**
0x10E grenade-ammo launch, 0xA3 jump launch + 0xA3/0x5FB head bump, 0x249+0x5DD laser
burn-through, 0x148 special-death cry.

## The table

### Player on foot

| Caller @ addr | Target | Id | Trigger | Status |
|---|---|---|---|---|
| `Player_HandleJump` 0x1916b0:111 | Sound_Play | 0x1DB | level 0x0700007 & ch 0x48 off: the refused jump | PORTED `nf-view/src/scene.rs:2294` |
| `Player_HandleJump` 0x1916b0:184 | Sound_Play3D | 0xA3 | jump press: the launch | PORTED `player.rs` `sfx_out` → `scene.rs:2324` + test |
| `Player_HandleJump` 0x1916b0:195-198 | Sound_Play3D | 0x5FB / 0xA3 | head bump ends the rise; 0x5FB when `level_tag+0x50` ∈ {0xD,0xE} | PORTED (0xA3 arm; the tag+0x50 gate is unparsed — `LadderCtx::level_bump_5fb`, live check below) |
| `Player_HandlePain` 0x1902c8:113 | Sound_Play3D | 0x88 | pain (1-in-4 roll) | PORTED `drone.rs:6725,8333` |
| `Player_DealWithObjHit` 0x194458:231-255 | Sound_Play/3D | 0x600, 0x412, 0x171 | Bond hit by an object (the 0x180 gate) | PORTED `drone.rs:8335-8350` |
| `Player_CheckForDeath` 0x190810:41,88 | Music_Event / Sound_Play3D | 6 / 0x89 | Bond dies: the music pulse + the death grunt | PORTED `music.rs` event 6, `death.rs:215` |
| `Player_MonitorAir` 0x191a70:106-141 | Sound_Play/3D/Stop | 0x60B, 0x60C, 0x27 | the dive loop 0x27 on under-entry, the gasp 0x60C on exit, 0x60B low-air | PORTED `scene.rs:2381-2394`, `sound.rs` swim gate |
| `Player_ChangeSubState` 0x190630:83-85 / `Player_Init` 0x1910f8:41-43 | Sound_StopAllWithId | 0x12, 0x3D, 0xF0 | substate change: the loop stops | PORTED `audio.stop_all_with_id` (`scene.rs:2398,2712`) |
| `Player_SetWeaponAnimObj` 0x1a2e38:227-295 | Sound_Play | 0x283/0x280 holster, 0x281/0x282 draw | the weapon-change machine (gadgets 0x4A..0x5E vs weapons) | PORTED `scene.rs:2715,2744,2787,2851` |
| `Player_WeaponChange` 0x1a6f80:128 | Sound_Play | 0x1DB | the change refused, `level_tag+0x62 == 1` | MISSING (the tag byte is unparsed) |
| `Player_WeaponInitBullet` 0x1a4bf0:262 | Sound_Play | 0xEC | per bullet init | MISSING (the port plays fire through the anim script op 1; the per-weapon op-1 remaps are G-graded, sound.md) |
| `Player_Zoom` 0x1a1a70:31-84 / `Player_Weapon` 0x1a24c8:164 | Sound_Stop | [BL+0x898] | scope/weapon exit stops the gadget loop | PORTED `scene.rs:2712` (`gadget_loop_ids`) |
| `Player_Creep` 0x1969d0:85 / `Player_Wire` 0x197658:107 | Music_Event | 0xD | the creep/wire stealth music | PORTED `music.rs` event 13 |
| `Player_Update` 0x193600:121 | Music_Event | `level_tag+0xA1` byte | the level's constant music event | MISSING (byte unparsed; low impact) |
| `HUD_UpdateStatusPane` 0x19c310:221-233 | Sound_Play | PlayMe1st=**0x5FC** / 0x1D4, PlayMe2nd=**0x1D5** / 1 (id table [M dis 0x19c618-0x19c664]; [L 0x30ccbc=0x5fc]) | the objective banner: the flash jingle / the new-objective sting | PORTED `scene.rs:3925` (1532/469/468) |

### Weapons and bullets

| Caller @ addr | Target | Id | Trigger | Status |
|---|---|---|---|---|
| `Bullet_init` 0x12c5a0:273 | Sound_Play3D | `param_7` | the caller-passed fire sound (guards' `weapon_data` fire id) | PORTED `drone.rs:7595,7641,7653` |
| `Bullet_init` 0x12c5a0:285-291 | Sound_Play3D | **0x10E** | ammo type (+0x90) 0x0E = the grenade pool: the launch whoosh | **PORTED this pass** `drone.rs` `fire_player` + test |
| `Bullet_init` 0x12c5a0:293-296 | Sound_Play3D | 0x5EF | weapon class (+2) 0x45: the grenade-bounce kin | PORTED `drone.rs:5173,8108` |
| `Bullet_CollisionHandler` 0x12ce80:316-432 | Sound_Play3D | 0x4A7, 0x446, 0x51C, surface id / 0x15, 0x13 | bullet impacts by surface/material | PORTED `drone.rs:8379-8543`, `SurfaceSounds` |
| `Bullet_DoTrails` 0x12d890:250 | Sound_Play3D | 0x1DC | the fly-by (closest-to-viewer + 1-in-5 roll) | PORTED `drone.rs:8355` (WORKLOG 0076) |
| `Bullet_DoTrails` 0x12d890:292,411 | Sound_Play3D | 0x16, 0x14 | explosion/gas trail events | PORTED `drone.rs:8607,8658` |
| `Bullet_DoTrails` 0x12d890:442 | Sound_Play3D | 0x447 | the smoke-trail tick | PORTED `drone.rs:8512` |
| `Bullet_DoTrails` 0x12d890:473 | Sound_Play3D | **0x5DD** | the Lazer-Watch beam's burn life ends (0x4E/0x4F) | **PORTED this pass** (with the lock clunk) + test |
| `Bullet_update` 0x12e7b0:104,290,327 | SetPosition / Stop / Play3D 0x27C | — | the in-flight loop follows the grenade; the bounce | PORTED `grenade.rs:114`, `drone.rs:8506` |
| `Bullet_Delete` 0x12e660:14,53 | Stop / Play3D 0x28D | — / 0x28D | the loop stop; the last-hit object class 'Q' (0x51) plays 0x28D | 0x51: control_funcs[0x51] is NULL (no update) — a static class the port has no list for: MISSING |
| `Bullet_casing_update` 0x12faa0:97 | Sound_Play3D | 0x1FE-0x204 by material | the casing lands | PORTED `casing.rs:168-223` |
| `Break_Kill` 0x12b970:69-75 | Sound_Play3D | 0x5F2 (41% roll) / 0x5F3 / 0x2E | a breakable (class 0x20) dies | MISSING — the Break class is unported (`parsemap` case 0x20 → `Break_Create`; no port list) |

### World objects

| Caller @ addr | Target | Id | Trigger | Status |
|---|---|---|---|---|
| `Door_SetState` 0x135618:11-18 / `Door_Activate` 0x135be0:84 | Sound_Play3D | DOOR data +0x60/+0x62/+0x64 | entering state 8 / 0x10; the locked-hit feedback | PORTED `door.rs:337-345` (`SoundEvent` outbox) |
| `Lock_Update` 0x1ae3d8:92 | Sound_Play3D | **0x249** | the laser burn drops +0x105 below 4 → `Lock_Unlock` | **PORTED this pass** (with 0x5DD) + test |
| `Lock_Update` 0x1ae3d8:282 | Sound_Play3D | (unlock click) | the picklock sequence's unlock | MISSING — the picklock minigame is unported (lock.rs covers the burn only) |
| `Lock_Update` 0x1ae3d8:368,374 | Sound_Play | 0x5DB, 0x5DA | the picklock ticks / the success | MISSING (same) |
| `Lock_Activate` 0x1aed00:27-32 / `Lock_Unlock` 0x1b0650:18 | Sound_Play3D | LOCK data [+2]/[+4]/[+6] u16s | the E-interaction locked/burnt feedback, the unlock | MISSING (no play in `lock.rs` — the ids come from the lock record) |
| `Searchlight_Update` 0x1cb900:401,455 | Sound_Play3D | 0xE5, 0xE6 | the searchlight's sweep/alarm | PORTED `searchlight.rs:279,449` |
| `Sensor_Update` 0x1af618:103-255 | Sound_Play3D | 0x4AA (activate), 0x4A9 (accelerating beep: 0.5→0.25→0.125 s), 0x4DD (sweep ends) | the security camera | MISSING — class 0x29 unported (`parsemap` case 0x29; the port has no sensor list) |
| `Pickup_Handler` 0x18e650:336 | Sound_Play3D | PICKUPINFO +0x2A (kind overrides 0xF5/0x141/0x14C) | the pickup taken | PORTED `pickup.rs:81-126` |
| `Hint_Update` 0x1b3380:26 | Sound_Play | HINT data [0] | the hint volume fires | PORTED `scene.rs:2295` |
| `SoundTrigger_Update` 0x1b2790:30-59 | Play3D/Play/Stop | TRIGGER data `puVar2[3]`, loop stop | the channel-gated emitter | PORTED `sound.rs` `SoundTriggers::tick` |
| `MusicTrigger_Update` 0x1b31a0:24 | Music_Event | TRIGGER data [0] | the music trigger volume | PORTED `music.rs` (the trigger volumes feed the event table) |
| `Hurt_Update` 0x17e5d0:120 | Sound_Play3D | HURT data +0x1A | the hurt volume (fire/spikes) loops | MISSING — the Hurt object (0x27) loop is not in the port's object lists |
| `Light_Update` 0x1cafb8:27 | Sound_Play3D | LIGHT data +0x2C | the flicker counter wraps through 0xF000 | MISSING in code but **dead data**: the placement call sites never write +0x2C (`light.rs:145-148`) |
| `AnimObjectUpdate` 0x1befd8:205 | Sound_Play3D | 0x286 | the anim object's event | PORTED `scene.rs:2402,2424` |
| `AnimProcessScriptCmds` 0x1c1b38:320,388 | Play3D / StopAllWithId | the anim script's sound cmd | the animation-script sounds | PORTED (`anim_sounds`, `d.sfx` paths) |
| `RB_RestingContact(Rot)` 0x1d7168:111 / 0x1d73a0:121 | Sound_Play3D | *(*(RIGIDBODY+0x54)+8), loop [RB+0x158] | a rigid body's contact list has a class-5 (bullet) object: the shot-body thud | MISSING — the port's props carry no rest-contact sound outbox |
| `SS_Update` 0x1a9948:86 | Sound_Play3D | SS data +0x14/+0x26 | the SS (0x25) playback state 0x40 | MISSING — the port's `ss.rs` runs activation only, not the playhead states |
| `SpaceLaser_Update` 0x1aa3f8:82-194 | Play 0x5DE / Play3D 0x46A / Stops | — | the laser grid: the trip hum (2-D) and the beam hit | MISSING — class 0x50 unported |
| `View_DrawSky` 0x1e5ad0:104-107 | Sound_Play | 0x5D (`LightningType==0`) / 0x5E | the sky slot's thunder countdown hits 1 | MISSING — `sky.rs` computes the event (`SlotTick.thunder`) but no sim/render caller wires it (G-019); live check below |
| `HandleMapSoundAllocation` 0x1dd118:69-77 | Play3D / Stop | the map's ambient list | the listener inside R starts / 1.2·R² stops | PORTED `sound.rs` `MapSounds` |
| `GunImp_Update` 0x1320d8:263-333 / `GunImp_Deactivate` 0x132c18:62 | Play3D 0x5E4/0x5E5/0x5E6 + the TurretGuns fire id / Stop | — | the gun implacement's servo/fire | MISSING (class unported; SP-visible only in 0700000C/0D) |
| `GT_Update` 0x180498:291-518 | Music_Event 2 / Play3D fire id, 0x10A | — | the gun turret | N/A (the port has no GT list; the drivable sections only) |

### Drones, guards, music, script

| Caller @ addr | Target | Id | Trigger | Status |
|---|---|---|---|---|
| `NDrone2_PlaySFX` 0x141848:121 | Sound_Play3D | `Drone_SfxInfo` entry | the state arms' foley one-shots | PORTED (`foley_reqs` drain, fix-room-sound) |
| `NDrone2_DoHitEffects` 0x1458e0:108-111 | Sound_Play3D | 0x601 (HITDATA+0x52==5) / 0x413 | the guard body hit | PORTED `drone.rs:6343-6346` (0075) |
| `NDrone2_DSTATE_SpecialDeath_Anim` 0x163c68 case 5 | Sound_Play3D | **0x148** | the special death of script 0x0600021F, `glb_players != 0` | **PORTED this pass** (`sfx_reqs` drain) + test |
| `DroneWeap_FireWeapon` 0x178f68:277 | Sound_Play3D | the weapon's fire sound | the guard fires | PORTED (the weapon-data fire path) |
| `BOT_handlePain` 0x125ed8 / `BOT_soundEffect` 0x126618 | Playing_Ref / Stop_Ref | [DCVars+0x73C] | the pain-grunt loop ref | PORTED (`pain_grunt`, fix-room-sound) |
| `DroneVision_HaveOpponentSight` 0x175ef0:90 / `NDrone2_SeenAndAttacking` 0x145150:68 / `DroneFunc_CheckAlarmRaised` 0x1453a8:15-42 / `Drone_EnableAll` 0x1383e8:28 / `Drone_InitComms` 0x139520:195 / `NDrone2_DSTATE_HostageSaved` 0x15ccf0:21 | Music_Event | 0xF seen, 0x2 combat, 0xE comms, 0x10 hostage | the combat-music state machine | PORTED `music.rs` (`seen_and_attacking`, `attack_gate`, events) |
| `Mission_Update` 0x182c60:25-36 / `Mission_MonitorObjectives` 0x1827d8:122 / `Player_CheckForDeath` 0x190810:41 | Music_Event | 8 objective, 7 fail, 6 death | the mission music pulses | PORTED `drone.rs:7108-7110`, `music.rs` |
| `Trigger_Activate` 0x1b2318:109 | Music_Event | 3 | the exit trigger's end sting | PORTED `drone.rs:7612` (LOAD_LEVEL) |
| `Copter_Update` 0x17dbb8:189 / `Sub_Update` 0x1ad200:116 / `Sub_DeployTorpedoes` 0x1acf68:74 / `MiniSubState_Attack` 0x181b00:216 / `GT_Update` 0x180498:291 | Music_Event | 2 | the vehicle combat music | N/A (driving) |
| `Script_SoundStart` 0x1d8820:36-40 / `Script_KillStream` 0x1d9860:83 / `Script_Interp` 0x1d9a08:224-233 | Play/Play3D, Stop, SetVolume/SetPosition | the script stream's id | the NIS/script sounds (follow, fade, stop) | PORTED (`script_sounds`, sound.md) |
| `Script_CameraStart` 0x1d9140:48 / `Script_FFwd` 0x1dac78:55 / `Script_KillStream` 0x1d9860:74 / `Script_Play` 0x1db588:40 | Music_Event | 0xB, 5, 0xC, 6 | the NIS music markers | PORTED `music.rs` (`nis_state`) |
| `Sound_UpdateSounds` 0x1dda70:95-98 | `Sound_ReqestPlaySfx` / `Sound_DoSubtitle` | — | the pool's state machine / the subtitle hook | PORTED (`sound.rs` the pool; subtitles are presentation) |
| `Sound_Ready` 0x1dceb8:108-109 | Music_Event | 0, 1 | the sound init per GameState | PORTED `music.rs` |
| `UpdateMusicalEvents` 0x114e70 | SFXStartMusic/SFXStopMusic | `MusicNowPlaying` | the IOP music engine | PORTED `nf-audio` + `music.rs` |
| `MPSound_Play` 0x18bee0:6 | Sound_Play | `param_1` | the MP wrapper | N/A (MP) |
| `SSys_Msg` 0x1ad878:12-43 | Sound_Play | `Laser2_135`/`Cam2_133` tables | the camera/laser alarm HUD messages | MISSING (the alarm message sounds; the tables are static data) |

### Front-end (N/A for the on-foot Sim)

`Menu_PlaySound` 0x1f3278:46 (0x1D9) and the twelve `P_*_Handler` screens' SFXStart(0x1D7 move,
0x470 select) × 2 each + `P_CREDITS` SFXStartMusic/SFXStopMusic(0x27) — the front-end UI sounds.
The port's front-end (`nf-view/menu.rs`) plays none of these; out of scope for the on-foot Sim.

## Live checks for the main agent

1. **Head-bump variant** (`level_tag+0x50`): in PCSX2, jump under a low ceiling on 07000005 and
   07000007 with gdbwatch on `Sound_Play3D`'s `$a0` (0x1dd548): the id at the bump (0x5FB vs
   0xA3) identifies each level's byte; the port's `LadderCtx::level_bump_5fb` then gets set
   per level.
2. **Thunder**: on 07000005 (island night, lightning), gdbwatch `Sound_Play` 0x1dd880's `$a0`
   through a storm cycle: expect 0x5D (or 0x5E if `LightningType != 0`) each time a sky slot's
   +0x48 countdown leaves 1. The port's `sky.rs::tick_slot` already computes the same event —
   wiring it needs the render-side call point.
3. **Objective jingles**: complete an objective on any level; the game plays 0x1D5 at the notice
   and 0x5FC on the banner's flash (0x1D4/1 on the alternate-id levels). The port plays
   1532/469/468 — confirm no audible difference.

## nf-view spots

Not visual — the four tests in `crates/nf-game/tests/sound_coverage_live.rs` are the checks
(NF_ISO-gated, silent).
