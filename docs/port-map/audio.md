# Port map: audio

Every ACTION.ELF / SFX.IRX function the audio subsystem involves: the address, the name, and the port status.
Tags: **ported** (the whole function, all branches, in the game's call structure), **partial** (some branches missing),
**stub** (signature only), **standin** (a different implementation, marked `STANDIN` in the code), **not-started**.

Sources: `work/decomp/SFX.IRX/` and `work/decomp/ACTION.ELF/` (the decompile), the Rust source in
`crates/nf-audio/src/{lib,sfx,music}.rs` and `crates/nf-game/src/{sound,music}.rs`.

## 1. EE sound layer (ACTION.ELF) — `crates/nf-game/src/sound.rs`

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x1dceb8 | `Sound_Ready` | partial | `sound::level_bank` (the bank-slot table); the `SFXLoadSoundBank` call and the `Music_Event(0,level)`/`(1,1)` hooks are in `nf-view/main.rs`. Not ported: the precache flag, the sound-init menu temporaries. |
| 0x1dcc68 | `Sound_Play` | ported | `Sounds::play` (2-D sound). |
| 0x1dcd10 | `Sound_Play3D` | ported | `Sounds::play3d` (3-D sound, custom radii for map ambients). |
| 0x1dce10 | `Sound_UpdateListeners` | partial | `MapSounds::update` + the `Sounds::update` listener push. **Not ported:** the `SFXSetEnvironment(room env, underwater)` call (the cel/room value isn't available — G-016; defaults to 0,0 in `nf-view/main.rs`). |
| 0x1dce68 | `Sound_UpdateSounds` | ported | `Sounds::update` (deferred start, grace period, stop, position update). |
| 0x1dcf20 | `Sound_Stop` | ported | `Sounds::stop`. |
| 0x1dcf48 | `Sound_StopAllWithId` | ported | `Sounds::stop_all_with_id`. |
| 0x1dcf88 | `Sound_SetPosition` | ported | `Sounds::set_position`. |
| 0x1dcfb8 | `Sound_IsFinnished` | ported | `Sounds::finished`. |
| 0x1dd118 | `HandleMapSoundAllocation` | ported | `MapSounds::update` (the radius in/out hysteresis). |
| 0x1ddb90 | `SFXIsSFXPlaying` | ported | `Driver::is_playing` (the `START_GRACE` 5-VBlank window). |

## 2. IOP sound driver (SFX.IRX) — `crates/nf-audio/src/sfx.rs`

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x2efc | `SFXrnd` | ported | `Rnd::rnd` (seeds 12341234/13123123, the exact update). |
| 0x2fb0 | `SFXfrnd` | ported | (unused in the SFX path; the port has it.) |
| 0x2ff0 | `SFXsfrnd` | ported | `Rnd::sfrnd` (−1024..1024). |
| 0x10250 | `PS2_SquareRoot` | ported | `ps2_sqrt` (10 Newton steps, truncating division). |
| 0x103bc | `PS2_DotProduct` | ported | inlined in `levels` (Σ/1000). |
| 0x10470 | `PS2_NormaliseVector` | ported | inlined in `levels`. |
| 0x1060c | `PS2_SFXCalculate3D` | ported | `Driver::levels` (the Pan1/Pan2/Pan3 math, quadratic falloff). |
| 0x10eb8 | `PS2_SFXSetup3D` / `PS2_SFXSetup2D` | ported | `Driver::levels` + the `Cmd::Play` volumes. |
| 0x11328 | `PS2_GetSampleVolume` | ported | inlined in `levels` (GSV). |
| 0x1300 | `ES_CombineVolumes` | ported | `Driver::combine` (music-class, outdoors, underwater, fade, ducker 1/3, square-law). |
| 0x3034 | `InitialiseSFXList` | ported | the 40-item pool (`MAX_ITEMS`). |
| 0x3118 | `NewSFX` | ported | `items.insert(0, item)` (head push). |
| 0x31c4 | `DeleteSFX` | ported | the kill pass (`remove`). |
| 0x329c | `ES_FillpsiSFXData` | ported | the psiSFX packet built before `psiInitialiseSound`. |
| 0x3524 | `StartSample` | ported | `Driver::start_sample` (the 3 `SFXsfrnd` calls in order: pitch, volume, pan; base pitch; reverb-send bit). |
| 0x3984 | `SetCueTimer` | ported | `Driver::cue_timer`. |
| 0x39f4 | `SFXPreCache` | **not-started** | the `SFXKeyOnFrameDelay` precache path; no port calls it (G-032). |
| 0x3a3c | `SFXStart` | ported | `Driver::start(None)` (2-D). |
| 0x3aac | `SFXStart3D` | ported | `Driver::start(Some(pos))` (3-D, the `VolumeOverRide` 100). |
| 0x40ec | `CountAllVoicesActive` | ported | `Driver::count_voices`. |
| 0x41e0 | `SFXGetImportance` | ported | inlined in `setup` (the priority × distance reject). |
| 0x4220 | `SFXSetup` | ported | `Driver::setup` (rejection, budget stealing, per-id limit, sample choice). |
| 0x4e20 | `SFXUpdateEnding` | ported | `Driver::ending` (delete or re-trigger after cue delay). |
| 0x4ee4 | `SFXUpdate` | ported | `Driver::update` (step 1–6: started decay, environment, ducker, fade, NisQuitten stop-ducker, per-item state machine, kill pass, active data). |
| 0x57e8 | `ES_GetMyPos` | ported | `Driver::get_my_pos` (TrackingType 1/3 centering). |
| 0x5cb4 | `ES_GetDist` | ported | `Driver::dist`. |
| 0x5efc | `SFXSetupListener` | ported | `Driver::set_listener`. |
| 0x6200 | `SFXSetNumberOfListeners` | **not-started** | always 1 (single player); MP not ported. |
| 0x6288 | `FindMusicTrackNumber` | ported | in `nf-game/src/music.rs` (the `MapMusic` table). |
| 0x83c8 | `ES_StartDucker` | ported | `Driver::start_ducker`. |
| 0x8418 | `ES_StopDucker` | ported | `Driver::stop_ducker` (NisQuitten → Duration=0). |
| 0x8438 | `ES_UpdateDucker` | ported | `Driver::update_ducker` (the s/6, s/15, sign-fixup, passed→100). |
| 0x8218 | `ES_MusicSetVolume` | ported | `music::Player::gain` + `Sound::batch` wires the live ducker/fade. |
| 0x8360 | `SFXMusicSetVolume` | ported | routes to `ES_MusicSetVolume`. |
| 0x839c | `SFXMusicGetVolume` | **not-started** | no caller needs the read-back. |
| 0xf8c | `SFXFadeUpdate` | ported | `Driver::fade_update` (2 s linear, FadeMusicVolume tracks). |
| 0x10e0 | `SFXFadeDown` | ported | `Driver::fade_down` (RPC cmd 6). |
| 0x1158 | `SFXFadeUp` | ported | `Driver::fade_up` (RPC cmd 7). |
| 0xdb4 | `SFXSetEnvironment` | ported | `Driver::set_environment` (underwater flag, reverb target, indoors target). |
| 0xe60 | `SFXUpdateEnvironment` | ported | `Driver::update_environment` (±16/−4 reverb, ±4 indoors, EVOL depth). |
| 0xd68 | `SFXResetEnvironment` | **not-started** | no caller; reset on bank change. |
| 0xc840 | `psiSetReverb` | ported | the `EVOL` depth sent to `Output` via `Cmd::Reverb`. |
| 0xc944 | `PS2_SetupReverb` | **standin** | the SPU effect preset (mode 4, delay 30, feedback 200) is a single-comb stand-in in `lib.rs::Reverb`; the `sceSdEffectAttr` layout is partially mangled in the decompile [M §9]. `STANDIN` in `lib.rs`. |
| 0xd544 | `PS2_SendKeyONKeyOff` | partial | the key-on/key-off dispatch; the 500 µs rear-voice delay is ported (`Cmd::Play { delay }`), the `ENVX` snapshot is **standin** (`Voice::playing` flag, not the ENVX register). |
| 0xda1c | `PS2_CheckKeyOn2` | **standin** | `Voice::playing` (not the ENVX register snapshot); `STANDIN` in `lib.rs`. |
| 0xdc68 | `PS2_CheckKeyOn` | **standin** | same. |
| 0xe420 | `psiRequestVoiceHandle2` | ported | `Driver::start_sample` (ADSR1=0x000F, ADSR2=0x1FC0, VoiceLooping, the 48 HW voices, 100 ES handles). |
| 0xe648 | `psiRequestVoiceHandle` | ported | (the ES handle pool). |
| 0xebcc | `PS2_VoiceSetReverb` | ported | the `reverb` bool on `Cmd::Play`. |
| 0xece4 | `psiSampleKeyOn2` / `psiSampleKeyOn` | ported | `Cmd::Play` (the key-on with delay). |
| 0xead4 / 0xea7c | `psiSampleKeyOff2` / `psiSampleKeyOff` | partial | `Cmd::Stop` → now begins the ADSR release (0075); was a hard stop before. |
| 0xeeec | `psiSetSampleFrequency` | ported | `Cmd::Set { pitch }`. |
| 0xf018–0xf384 | `psiSetSampleVolume*` | ported | `Cmd::Set { left, right }`. |
| 0xf46c | `psiSamplePause` / `psiSampleUnPause` | **not-started** | the pitch-0 / restore path (G-032). |
| 0x1f88 | `SFXRemove` | ported | `Driver::remove`. |
| 0x1994 | `SFXSetPositionAndVelocity` | ported | `Driver::set_position`. |
| 0x171c | `SFXPause` / `SFXUnPause` | **not-started** | the global pause (G-032). |
| 0x18dc | `ES_HandleNisPauseSfx` | **not-started** | the per-item NIS pause (PauseInNis). |
| 0x2500 | `SFXReset` | **not-started** | no caller needs it. |
| 0x25a4 | `SFXLoadSoundBank` | ported | `Driver::load_bank`. |
| 0x2164 | `SFXDeLoadSoundBank` | **not-started** | bank unload. |
| 0x2d24 | `SFXInitialise` | partial | the driver globals' init values are in `Driver::new`; the IOP thread/RPC is not ported (the port calls `Driver` directly). |

## 3. SPU output (nf-audio/src/lib.rs) — the cpal mixer

| Game function / hardware | Status | Notes |
|---|---|---|
| The SPU2 4-tap Gaussian interpolation | **standin** | Catmull-Rom 4-tap kernel (`catmull_rom` in `lib.rs`); the exact SPU2 coefficient table is hardware (not in SFX.IRX), deferred G-032d. `STANDIN` in `lib.rs`. |
| The SPU2 ADSR envelope (ADSR1=0x000F, ADSR2=0x1FC0) | partial | instant attack + max sustain ported (`env: 1.0`); the linear release is ported (`RELEASE_SAMPLES = 192`, ≈4 ms). **Not ported:** the exact SPU2 rate-table step count (the release length is a [S] estimate, not measured from the disc — G-032d). |
| The SPU2 reverb (the effect preset) | **standin** | single-comb per channel (`Reverb` in `lib.rs`); the exact `PS2_SetupReverb` preset coefficients are deferred (G-032d). `STANDIN` in `lib.rs`. |
| Voice allocation / stealing | ported | the 48 HW voices, 100 ES handles, the 500 µs rear delay. |
| Music stream resampling (32 kHz → device) | ported | linear resampling in the mix callback. |

## 4. EE music logic (ACTION.ELF) — `crates/nf-game/src/music.rs`

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x1de328 | `Music_Event` | ported | `Music::event` (the 64-entry event board). |
| 0x114e70 | `UpdateMusicalEvents` | ported | `Music::update` (the start/section, NIS state, attack hysteresis, death/fail, the `castle_a` per-level updater). |
| 0x112d78 | `UpdateMFX_01CastleA` | ported | `Music::castle_a` (07000005). |
| 0x111ed8–0x114d60 | `UpdateMFX_*` (20 updaters) | **not-started** | only 07000005's `castle_a` is ported; the other 19 levels' per-level music updaters are not (each is a small switch on `Music_Event` indices; the port's `update` handles the common cases). |
| 0x114e70 | `PickRandomCombat` / `PickRandomStealth` | ported | `Music::pick_combat` / `pick_stealth` (round-robin). |
| 0x145150 | `NDrone2_SeenAndAttacking` | ported | `AttackGate::seen_and_attacking` (the count/floor/window/distance vars, the latch). |
| 0x1de3a0 | (the music command queue) | ported | `Music::out` → `Cmd::Start`/`Jump`/`FadeDown`. |

## 5. IOP music stream player (SFX.IRX) — `crates/nf-audio/src/music.rs`

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x6eb4 | `SFXStartMusic` | ported | `Player::start`. |
| 0x79ec | `SFXStopMusic` | ported | `Cmd::MusicStop`. |
| 0x75c0 | `SFXJumpToMusicMarker` | ported | `Player::request`. |
| 0x75fc | `SFXJumpToMusicMarkerUpdate` | ported | `Player::update` (the jump cut). |
| 0xb560 | `UpdateMusicMarkers` | ported | `Player::update` (marker scan, buffer cutting). |
| 0xc2c8 | `ES_CheckFileBuffersForJumpMarkers` | ported | in `Player::update` (the deferred jump cut). |
| 0xc228 | `ES_HasTimerPassed` | ported | the lockout / complete timing. |
| 0x789c | `SFXStopStream` / `ES_StopStream` | ported | `Player::stop`. |
| 0xa1a0 | `SFXUpdateStreams` | partial | `Player::update` (the buffer plan); the async file reads are **standin** (the port pre-loads the SSD). |
| 0x85e0 | `ES_StartStreamMemCpy` / `ES_UpdateStreamMemCpy` | **standin** | the port decodes the whole SSD at start, not streamed through the ring. `STANDIN` in `music.rs`. |
| 0x874c | `ADPCM_Decrunch` | ported | `nf_formats::vag::decode`. |

## 6. Speech and streamed effects (ACTION.ELF)

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x141848 | `NDrone2_PlaySFX` | **not-started** | drone foley (`Drone_SfxInfo`, the 16-entry ring). G-032. |
| 0x141b00 | `NDrone2_SpeechSFX` | **not-started** | drone speech (the facial-anim path). G-032. |
| 0x1497e8 | `DroneFunc_ChooseSFX` | ported | `Rng::rand(b-a+1)` (the random line index). |
| 0x1c6ca8 | `AnimScriptPeekSfx` | **not-started** | the op-1 sfx id peek for speech precache. G-032. |
| 0x177800 | `DroneVision_AlertSound` | **not-started** | the alert sound routing. G-032. |
| 0x18bee0 | `MPSound_Play` | **not-started** | MP sound (not ported). |

## 7. Bullet fly-by (ACTION.ELF) — `crates/nf-game/src/bullet.rs`

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x12d890 | `Bullet_DoTrails` | ported | `Bullet::update` fly-by check (the `+0x6C & 0x40000` gate, the closest-point-to-viewer, `Rand_Rand(5)`, sound 0x1dc). **Not ported:** the smoke (`Gas_Init`), the `0x10000` swoosh (`Swoosh_Create`), the flash-bang block (`+0x70 & 0x200`), the satchel digits — those are other systems (particles, flash-bang, satchel). |
| 0x1df7e0 | `vecutil_calculate_closest_point_on_line` | ported | `closest_point_on_segment`. |

## 8. Output gain (ACTION.ELF)

| Address | Name | Status | Notes |
|---|---|---|---|
| 0x11b140 | `CreateSqueezeFrame` | **standin** | the `GAIN` constant (`SqueezeColourIn` 0x91 / 0x80 = 1.133) is a global measured only on the 07000005 snapshot; the writer is unknown (U-0xx). The port applies it as a uniform scale. `STANDIN` in `gpu.rs`. |
| 0x11e2e8 | `psiPreGameRun` | ported | `scene::fog_for` (the per-level fog table, confirmed against the decompile). |

## Summary

| Subsystem | Ported | Partial | Stub | Standin | Not-started |
|---|---|---|---|---|---|
| EE sound layer | 7 | 2 | 0 | 0 | 0 |
| IOP driver (SFX.IRX) | 38 | 4 | 0 | 3 | 12 |
| SPU output | 2 | 1 | 0 | 2 | 0 |
| EE music logic | 5 | 0 | 0 | 0 | 1 (20 level updaters) |
| IOP music stream | 8 | 1 | 0 | 1 | 0 |
| Speech | 1 | 0 | 0 | 0 | 4 |
| Bullet fly-by | 2 | 0 | 0 | 0 | 0 |
| Output gain | 0 | 0 | 0 | 2 | 0 |
| **Total** | **63** | **8** | **0** | **8** | **17** |
