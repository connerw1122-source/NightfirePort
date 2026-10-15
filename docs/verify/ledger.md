# The verify ledger — which ported sim functions are proven bit-exact against the real game's calls

Idea from REA (morluto/rea, MIT): `compare_functions` / `verify_reconstruction` judge a reconstruction against a
finite, explicit specification, and **a pass covers only the declared claims**. NightfirePort already has that
machinery for three functions (task 045n-verify-bitexact: `tools/trace/gdbcall.py` records real calls in PCSX2,
`docs/tools/call-traces.md` is the format, `tests/calls/*` the recordings, `crates/nf-game/tests/diff_calls.rs`
the replay harness). This ledger is the claim inventory: **one row per ported game function the sim depends on**,
with the strongest claim the evidence supports. A "bit-exact" claim here is enforced by the guard test
(`ledger_claims_match_the_recordings` in `diff_calls.rs`): it fails if the row has no recording behind it, no
adapter in the harness, or fewer recorded calls than the row claims.

## Status grammar (shared with the guard test — other strings fail the test)

```
Status := "bit-exact on " N " recorded calls"     N ≥ 1; the claim is enforced (see the guard)
        | "within tolerance (" eps ")"            eps is an f32, e.g. 1e-5; Notes must cite the evidence
        | "unverified"                            nothing recorded yet
        | "unverified (" prose ")"                e.g. "unverified (recording exists; no adapter)"
        | "known divergent (" reason ")"          reason cites a GAPS id or a doc
Address  := "0x" hex digits | "-"              (the game's entry address, as in the symtab)
Port fn  := "crate::module::fn" | "(adapter in diff_calls.rs …)" — a greppable path when one exists
Recording := "<Function>.calls" | "-"          (a file under tests/calls/)
```

Guard rules (enforced in `diff_calls.rs`): a `bit-exact on N recorded calls` row must have (a) the recording file
in `tests/calls/`, (b) the file's `function=` header equal to the row's Name, (c) at least N parsed calls, (d) an
adapter registered for that Name in `adapters()`, and (e) a real address (not `-`). Unknown status strings,
duplicate addresses, and a file name that disagrees with the header fail the test. Non-bit-exact rows are free
text; the guard only checks that they use the grammar above.

Recording a new function (the main agent, in PCSX2):
```
python -I tools/trace/gdbcall.py <Function|0xaddr> --spec "<regions>" [--calls N] [--seconds S] --out tests/calls/<Function>.calls
```
with a level loaded (`tools/trace/goto_level.py <level>`). Region syntax and the file format: `docs/tools/call-traces.md`.

## Verified now (3 functions, 40/40 calls each)

| Address | Name | Port fn | Status | Recording |
|---|---|---|---|---|
| 0x120700 | PS2Sinf | `trig::sinf` → `anim::sinf` | bit-exact on 40 recorded calls | `PS2Sinf.calls` |
| 0x1e3e80 | Quat_QuatToMat | (adapter in diff_calls.rs — the decompile's EE-FPU op order) | bit-exact on 40 recorded calls | `Quat_QuatToMat.calls` |
| 0x1e3738 | Rand_Rand | `drone::Rng::rand` | bit-exact on 40 recorded calls | `Rand_Rand.calls` |

Notes: the PS2Sinf port is the game's own polynomial (range reduction on the EE FPU, then the VU0 lane polynomial);
`PS2Sinf2` 0x11f968 / `PS2Sinf3` 0x11fc20 are the same polynomial on 2/3 VU0 lanes and are separately unverified.
The Quat_QuatToMat port is the adapter itself (the decompile's operation order with `ps2math` ops, registered in
`diff_calls.rs::adapters()` — there is no crate function for it).
`Check_Target` (0x1a7810) and `Player_AutoAim` (0x1a7c98) also have recordings (40/80 calls, level 07000005 under
play) but no adapter yet — their rows are in the hot-path table below.

## Hot paths — the functions the sim leans on hardest (seeded from the port map + the atlas)

Compact table; the specs and situations follow below. "Adapter" = can a `diff_calls.rs` adapter replay the
recorded calls from registers + captured memory alone (yes / partial / no + why).

| Address | Name | Port fn | Status | Recording | Adapter |
|---|---|---|---|---|---|
| 0x1e89e0 | Vision_InView | `nf-view/src/scene.rs` visibility culling (signed-distance form) | unverified | - | yes (4 plane dots) |
| 0x1e4188 | Quat_Slerp_Acc | `anim::slerp` | unverified | - | yes |
| 0x1df2a8 | Mat_Align2Dir | `searchlight::align2dir` | unverified | - | yes |
| 0x1ec940 | Collide_SphereIntersect | `world.rs` capsule/sphere paths (`closest_point_scoped` et al.) | unverified | - | partial (bool + centre push; the hit-list pool is the game's) |
| 0x10a8c8 | Intersect_RayGeom | `world.rs` ray paths (`segment_hit`, `segment_surface`, …) | unverified | - | partial (dist/normal/point/surface; v0/v1 vertex indices are don't-care) |
| 0x1e3780 | Rand_FRand | `drone::Rng::frand` | unverified | - | yes |
| 0x1e3810 | Rand_FRandHalf | `effect::frand_half` | unverified | - | yes |
| 0x1e38b0 | Rand_FRand_MVar2 | `effect::frand_mvar2` | unverified | - | yes |
| 0x1e36f0 | Rand_Random | (no direct port; = the draw `Rng::frand` converts) | unverified | - | yes (needs a port fn) |
| 0x1dfef0 | Rand_FRandHalf_Vec | `effect::frand_half_vec` | unverified | - | yes |
| 0x1e00d0 | Rand_FRand_MVar2_Vec | (no port) | unverified | - | yes (needs a port fn) |
| 0x145558 | NDrone2_HitDamage | `damage::hit_damage` | unverified | - | yes (exposes the port's armour:0/mode:0 stand-in) |
| 0x179740 | DroneWeap_DoBulletAccuracy | `drone.rs::fire_weapon` (accuracy half) | unverified | - | partial (port is marked partial — known deltas below) |
| 0x17c518 | Explode_Propagate | `explode::propagate` / `grenade::explode_propagate` | unverified | - | no (targets come from the scene, not memory) |
| 0x1c75a0 | AnimFrameSet | `anim::sequence_pose` | unverified | - | no (channel blob is variable-length, pointer-chased; port consumes disc formats) |
| 0x1a7810 | Check_Target | `drones.rs` crosshair scan (scene-coupled) | unverified (recording exists; no adapter) | `Check_Target.calls` | no (needs the scene) |
| 0x1a7c98 | Player_AutoAim | `Drones::auto_aim` (scene-coupled) | unverified (recording exists; no adapter) | `Player_AutoAim.calls` | no (needs the scene) |

### Recording requests (exact commands for the main agent; PCSX2-gdb with the level loaded)

All `--out tests/calls/<Name>.calls`. Registers ride the format's `gpr`/`fpr` lines automatically — float args in
`f12..f15` and every GPR need no region; regions below capture the memory the function reads (`in`) and writes
(`out`/`io`). EE ABI (eabi-style): pointer args `a0..a3,t0..t3`, 9th GPR arg at entry-`sp+0x00`, floats `f12+`.

**07000005, at the start (guards idle, nobody shooting)** — math library, fires every frame:

```
python -I tools/trace/gdbcall.py Quat_Slerp_Acc 0x1e4188 --spec "in:a1:16 in:a2:16 out:a3:16" --calls 40 --seconds 60
python -I tools/trace/gdbcall.py Mat_Align2Dir 0x1df2a8 --spec "out:a0:0x40 in:a1:0x10 in:a2:0x10 in:a3:0x10" --calls 40 --seconds 60
python -I tools/trace/gdbcall.py Vision_InView 0x1e89e0 --spec "in:a0:16 in:@(a1+0x70):0x40" --calls 40 --seconds 60
python -I tools/trace/gdbcall.py Rand_FRand 0x1e3780 --spec "io:0x30d0a0:4 io:0x30d0a4:4" --calls 40 --seconds 60
```

- `Vision_InView` [M 0x1e89e0 decompile]: reads the 16-byte position at `a0` and the viewer's four 16-byte planes
  at `viewer+0x70..0xB0` (VU0 `vmul`+`addbc` dot per plane, [M] decomp loop to 4); `dot < -f12` → `v0=0`. The
  viewer planes are runtime state, so the replay adapter replays the four dots from the recorded planes — no
  level data needed. Record while the camera moves (culling runs per frame with varied planes).
- `Quat_Slerp_Acc` [M 0x1e4188]: `f12 t`, quats at `a1`/`a2` (16 B each), writes `a3` (16 B). Internally
  `acosf` + `PS2Sinf` — already bit-exact in the port, so the adapter composes `anim::slerp` directly.
- `Mat_Align2Dir` [M 0x1df2a8]: writes the 0x40 MATRIX at `a0` from the direction `a1`, up `a2`, fallback `a3`
  (16 B each). Fires on bullet tracers/decals — firing a few shots first gives varied directions; the idle start
  also calls it (object placement).

**Rand family — one session, all share the seed spec** (SEED_X 0x30d0a0 / SEED_Y 0x30d0a4 [M symtab]):

```
python -I tools/trace/gdbcall.py Rand_FRandHalf 0x1e3810 --spec "io:0x30d0a0:4 io:0x30d0a4:4" --calls 40 --seconds 60   # shoot walls: Debris_Create draws per particle
python -I tools/trace/gdbcall.py Rand_FRand_MVar2 0x1e38b0 --spec "io:0x30d0a0:4 io:0x30d0a4:4" --calls 40 --seconds 60 # sustained fire: Bullet_init's casing draw
python -I tools/trace/gdbcall.py Rand_Random 0x1e36f0 --spec "io:0x30d0a0:4 io:0x30d0a4:4" --calls 40 --seconds 60      # 07000001 walking: Player_Update draws per frame
python -I tools/trace/gdbcall.py Rand_FRandHalf_Vec 0x1dfef0 --spec "out:a0:12 io:0x30d0a0:4 io:0x30d0a4:4" --calls 40 --seconds 60  # shoot walls: debris spin vector
python -I tools/trace/gdbcall.py Rand_FRand_MVar2_Vec 0x1e00d0 --spec "io:a0:12 io:0x30d0a0:4 io:0x30d0a4:4" --calls 40 --seconds 90  # Sub_Laser only — needs the sub level (07000019/0700001a); lowest priority
```

`Rand_FRand_MVar2_Vec` ACCUMULATES onto `a0` (`+=`) [M decomp], hence `io`; the Vec variants overwrite/accumulate
12 bytes (a VECTOR's xyz). Adapter self-check on the first recording: the `a0` bytes must change per call [S].

**Collision — 07000005, stand ~2 m from a wall:**

```
python -I tools/trace/gdbcall.py Collide_SphereIntersect 0x1ec940 --spec "io:a0:16 io:t0:4 in:sp:4" --calls 60 --seconds 45  # fire 3-4 shots into the wall + throw one grenade + walk a few steps
python -I tools/trace/gdbcall.py Intersect_RayGeom 0x10a8c8 --spec "io:a0:0xa0 io:a1:0x60" --calls 40 --seconds 90          # fire ONE shot into the wall (very hot: guard LOS rays also hit it)
```

- `Collide_SphereIntersect` [M disasm of the prologue + 3 call sites]: radius in `f12`; centre VECTOR at `a0` is
  **in/out** (pushed out along the hit normal — a real output the players/explosions consume); `t0` is the
  HITDATA** list head (in = 0, out = pool head or 0); `t1/t2/t3` restrict/type/filter and the flags word at
  entry-`sp+0x00` ride the gpr line. `v0` = hit-list non-empty. The cel/geometry graph is pointer-chased — the
  replay adapter must supply geometry from the port's own level data and compares `v0`, the 16 centre bytes and
  `(head != 0) == (v0 != 0)` [S]. Verify `in:sp:4` == 4 on the first trace (both traced callers store 4) [M].
- `Intersect_RayGeom` [M 0x10a8c8 decompile]: `a0` = the caller's 0xA0-byte HITTEST (start +0x20, end +0x30,
  dir +0x40, query cel +0x80, COLLDATA root +0x88 — pointer-chased, not captured), `a1` = the 0x60-byte HITDATA
  out (dist +0x0C, packed normal ×2^-14 +0x10, point +0x40, surface byte +0x50). `v0/v1` = the hit triangle's
  packed vertex-index quad — the port stores world-space vertices and cannot reproduce it (don't-care for the
  adapter). Expect heavy but harmless slowdown while the breakpoint is armed.

**Combat — 07000005, aggro the courtyard guards, SP only (MPSettings+0x184 must be 0 [M gate]):**

```
python -I tools/trace/gdbcall.py NDrone2_HitDamage 0x145558 --spec "io:a0:0x10 in:@(a0+4)+0x14:1 in:@(a0+4)+0x44:1 io:@(a0+4)+0xac:4 in:@(a0+4)+0xbb:1 in:@(a0+4)+0xc5:1 in:@(a0+4)+0xd8:2 in:@(a0+4)+0xe0:4 in:@(a0+4)+0x150:4 in:@(a0+4)+0x170:4 in:@(a0+4)+0x2b4:4 out:@(a0+4)+0x3a:1 in:0x2a3774:4 in:0x2a3790:4 in:0x2a4924:4 in:0x30cab0:0x2c" --calls 40 --seconds 120
python -I tools/trace/gdbcall.py DroneWeap_DoBulletAccuracy 0x179740 --spec "io:a0:0x10 in:@(a0+4)+0x41:1 in:@(a0+4)+0xb4:1 in:@(a0+4)+0xda:2 in:@(a0+4)+0x170:4 in:@(a0+4)+0x1a0:4 in:@(a0+4)+0x1c0:0x10 in:@(a0+4)+0x1dc:4 in:@(a0+4)+0x270:4 out:@(a0+4)+0x200:0x10 in:@(a0)+0xec:4 in:0x2a3774:4 in:0x2a3790:4 in:0x2a379c:4 io:0x30d0a0:8 in:0x30caf0:0x2c" --calls 40 --seconds 120
python -I tools/trace/gdbcall.py Explode_Propagate 0x17c518 --spec "in:a0:0x10 in:a1:4 in:a2:4" --calls 20 --seconds 180  # grenades via tools/trace/cheats.py into prop clusters; ~1 call per blast, expect a smaller N
python -I tools/trace/gdbcall.py AnimFrameSet 0x1c75a0 --spec "in:a0:4 in:a1:0xb0 in:@(a2)+0x30:1 in:@(a0+0xdc)+0x110:4 out:@(a1+0x84):0x800" --calls 40 --seconds 90  # walk forward with a weapon out near a patrolling guard
```

- `NDrone2_HitDamage` [M 0x145558 decompile; DCVars layout docs/fields/drone-statemachine.md]: `a0` DCVars*
  (0x10-byte header; `+0x04` = the Drone*), `f12` dmg, `a1` part, `a2` weapon, `a3` forced. Void in the game — the
  outputs are the Drone writes: `+0x150` f32 last damage, `+0x3A` u8 head latch, `+0xAC` f32 health (the io/out
  regions above capture all three; +0xAC also read). Reads the armour bits `+0xBB`, mode `+0xC5`, kind `+0xD8`,
  skin `+0xE0`, opponent `+0x170` vs killer `+0x2B4`, level 0x2a3774 (GameState+0x0C), difficulty 0x2a3790
  (GameState+0x28), MP gate 0x2a4924, and the tuning run 0x30cab0..0x30cadc. **This is the one genuinely
  replayable-as-adapter combat target** (a closed arithmetic fold) — and the recording will measure the real
  +0xBB/+0xC5, which the port currently stands in with `armour: 0, mode: 0` (`drone.rs:2142`) [M]. Vary the part:
  ~25 torso, ~8 leg, ~7 head shots + 1-2 grenade blasts (part = -1 arm).
- `DroneWeap_DoBulletAccuracy` [M 0x179740 decompile]: `a0` DCVars*; writes Drone+0x200 (16 B aim offset);
  consumes one `Rand_FRand(100.0)` draw, so the seeds are captured (`io:0x30d0a0:8`). Known port deltas [M
  drone.rs::fire_weapon]: the first-stopped branch (level 0x0700000c..d, +0xDA==0x13, ×
  DroneFiring_TargetFirstStopped_Accuracy) is missing, the wobble is yaw-only instead of
  `RotMatrix(Drone+0x1C0)`, the seen-ticks gate is hardcoded 120 vs `TimeToHit·FRAME_RATE`, and per-level
  accuracy constants are hardcoded vs the tuning floats 0x30caf0..0x30cb1c — the recording pins all of them.
- `Explode_Propagate` [M 0x17c518]: dispatches damage immediately per hit object — no persistent memory output,
  so only a census recording (inputs) is possible; the port side replays against its own Targets lists. Record
  the inputs + the RNG state if falloff replays are wanted later: add `io:0x30d0a0:8` if so [S].
- `AnimFrameSet` [M 0x1c75a0 + AnimFrameCopy 0x1c5668]: reduced claim only — the seq clock `+0x9C`, frame counts
  `+0xA2/+0xA4`, skeleton bytes (`+0xA9`, skin `+0x30`), the LOD gate `obj+0xDC → +0x110..0x113` and the
  evaluated frame buffer head (first 0x800 of `*(seq+0x84)`) pin the frame arithmetic (⌊f⌋+1 clamp, the
  |frac| ≤ 0.0002 gate) without the variable-length channel blob [S].

### Check_Target with a hit (carried over from 045n)

```
python -I tools/trace/gdbcall.py Check_Target 0x1a7810 --spec "io:@(a0+0xe0)+0x130:4" --calls 80 --seconds 120  # 07000005, aim mode with a drone actually under the crosshair
```

The committed 40 calls are all no-hit (+0x132 = 0, +0x133 = 2) [L docs/tools/call-traces.md]; an adapter will
still need the scene, but the hit-case recording pins the output fields.

## Appendix
 — every other ported sim function (seeded from docs/port-map/, status "unverified" unless claimed above)

Generated from the port-map tables (`guard-ai`, `core-loop`, `levels`, `weapon-switching`, `audio`; the port fn
column quotes the port map). The port map row keeps the full notes; this is the claim inventory, so every row
starts "unverified" and moves up only through the recording + replay route above.

| Address | Name | Port fn | Status | Recording | Notes (subsystem; port-map status; summary) |
|---|---|---|---|---|---|
| 0xd68 | SFXResetEnvironment | - | unverified | - | audio; not-started; no caller; reset on bank change. |
| 0xdb4 | SFXSetEnvironment | - | unverified | - | audio; ported; Driver::set_environment (underwater flag, reverb target, indoors target). |
| 0xe60 | SFXUpdateEnvironment | - | unverified | - | audio; ported; Driver::update_environment (±16/−4 reverb, ±4 indoors, EVOL depth). |
| 0xf8c | SFXFadeUpdate | - | unverified | - | audio; ported; Driver::fade_update (2 s linear, FadeMusicVolume tracks). |
| 0x10e0 | SFXFadeDown | - | unverified | - | audio; ported; Driver::fade_down (RPC cmd 6). |
| 0x1158 | SFXFadeUp | - | unverified | - | audio; ported; Driver::fade_up (RPC cmd 7). |
| 0x1300 | ES_CombineVolumes | - | unverified | - | audio; ported; Driver::combine (music-class, outdoors, underwater, fade, ducker 1/3, square-law). |
| 0x171c | SFXPause / SFXUnPause | - | unverified | - | audio; not-started; the global pause (G-032). |
| 0x18dc | ES_HandleNisPauseSfx | - | unverified | - | audio; not-started; the per-item NIS pause (PauseInNis). |
| 0x1994 | SFXSetPositionAndVelocity | - | unverified | - | audio; ported; Driver::set_position. |
| 0x1f88 | SFXRemove | - | unverified | - | audio; ported; Driver::remove. |
| 0x2164 | SFXDeLoadSoundBank | - | unverified | - | audio; not-started; bank unload. |
| 0x2500 | SFXReset | - | unverified | - | audio; not-started; no caller needs it. |
| 0x25a4 | SFXLoadSoundBank | - | unverified | - | audio; ported; Driver::load_bank. |
| 0x2d24 | SFXInitialise | - | unverified | - | audio; partial; the driver globals' init values are in Driver::new; the IOP thread/RPC is not ported (the port calls Driver directly). |
| 0x2efc | SFXrnd | - | unverified | - | audio; ported; Rnd::rnd (seeds 12341234/13123123, the exact update). |
| 0x2fb0 | SFXfrnd | - | unverified | - | audio; ported; (unused in the SFX path; the port has it.) |
| 0x2ff0 | SFXsfrnd | - | unverified | - | audio; ported; Rnd::sfrnd (−1024..1024). |
| 0x3034 | InitialiseSFXList | - | unverified | - | audio; ported; the 40-item pool (MAX_ITEMS). |
| 0x3118 | NewSFX | - | unverified | - | audio; ported; items.insert(0, item) (head push). |
| 0x31c4 | DeleteSFX | - | unverified | - | audio; ported; the kill pass (remove). |
| 0x329c | ES_FillpsiSFXData | - | unverified | - | audio; ported; the psiSFX packet built before psiInitialiseSound. |
| 0x3524 | StartSample | - | unverified | - | audio; ported; Driver::start_sample (the 3 SFXsfrnd calls in order: pitch, volume, pan; base pitch; reverb-send bit). |
| 0x3984 | SetCueTimer | - | unverified | - | audio; ported; Driver::cue_timer. |
| 0x39f4 | SFXPreCache | - | unverified | - | audio; not-started; the SFXKeyOnFrameDelay precache path; no port calls it (G-032). |
| 0x3a3c | SFXStart | - | unverified | - | audio; ported; Driver::start(None) (2-D). |
| 0x3aac | SFXStart3D | - | unverified | - | audio; ported; Driver::start(Some(pos)) (3-D, the VolumeOverRide 100). |
| 0x40ec | CountAllVoicesActive | - | unverified | - | audio; ported; Driver::count_voices. |
| 0x41e0 | SFXGetImportance | - | unverified | - | audio; ported; inlined in setup (the priority × distance reject). |
| 0x4220 | SFXSetup | - | unverified | - | audio; ported; Driver::setup (rejection, budget stealing, per-id limit, sample choice). |
| 0x4e20 | SFXUpdateEnding | - | unverified | - | audio; ported; Driver::ending (delete or re-trigger after cue delay). |
| 0x4ee4 | SFXUpdate | - | unverified | - | audio; ported; Driver::update (step 1–6: started decay, environment, ducker, fade, NisQuitten stop-ducker, per-item state machine, kill pass, active data). |
| 0x57e8 | ES_GetMyPos | - | unverified | - | audio; ported; Driver::get_my_pos (TrackingType 1/3 centering). |
| 0x5cb4 | ES_GetDist | - | unverified | - | audio; ported; Driver::dist. |
| 0x5efc | SFXSetupListener | - | unverified | - | audio; ported; Driver::set_listener. |
| 0x6200 | SFXSetNumberOfListeners | - | unverified | - | audio; not-started; always 1 (single player); MP not ported. |
| 0x6288 | FindMusicTrackNumber | - | unverified | - | audio; ported; in nf-game/src/music.rs (the MapMusic table). |
| 0x6eb4 | SFXStartMusic | - | unverified | - | audio; ported; Player::start. |
| 0x75c0 | SFXJumpToMusicMarker | - | unverified | - | audio; ported; Player::request. |
| 0x75fc | SFXJumpToMusicMarkerUpdate | - | unverified | - | audio; ported; Player::update (the jump cut). |
| 0x789c | SFXStopStream / ES_StopStream | - | unverified | - | audio; ported; Player::stop. |
| 0x79ec | SFXStopMusic | - | unverified | - | audio; ported; Cmd::MusicStop. |
| 0x8218 | ES_MusicSetVolume | - | unverified | - | audio; ported; music::Player::gain + Sound::batch wires the live ducker/fade. |
| 0x8360 | SFXMusicSetVolume | - | unverified | - | audio; ported; routes to ES_MusicSetVolume. |
| 0x839c | SFXMusicGetVolume | - | unverified | - | audio; not-started; no caller needs the read-back. |
| 0x83c8 | ES_StartDucker | - | unverified | - | audio; ported; Driver::start_ducker. |
| 0x8418 | ES_StopDucker | - | unverified | - | audio; ported; Driver::stop_ducker (NisQuitten → Duration=0). |
| 0x8438 | ES_UpdateDucker | - | unverified | - | audio; ported; Driver::update_ducker (the s/6, s/15, sign-fixup, passed→100). |
| 0x85e0 | ES_StartStreamMemCpy / ES_UpdateStreamMemCpy | - | unverified | - | audio; standin; the port decodes the whole SSD at start, not streamed through the ring. STANDIN in music.rs. |
| 0x874c | ADPCM_Decrunch | - | unverified | - | audio; ported; nf_formats::vag::decode. |
| 0xa1a0 | SFXUpdateStreams | - | unverified | - | audio; partial; Player::update (the buffer plan); the async file reads are standin (the port pre-loads the SSD). |
| 0xb560 | UpdateMusicMarkers | - | unverified | - | audio; ported; Player::update (marker scan, buffer cutting). |
| 0xc228 | ES_HasTimerPassed | - | unverified | - | audio; ported; the lockout / complete timing. |
| 0xc2c8 | ES_CheckFileBuffersForJumpMarkers | - | unverified | - | audio; ported; in Player::update (the deferred jump cut). |
| 0xc840 | psiSetReverb | - | unverified | - | audio; ported; the EVOL depth sent to Output via Cmd::Reverb. |
| 0xc944 | PS2_SetupReverb | - | unverified | - | audio; standin; the SPU effect preset (mode 4, delay 30, feedback 200) is a single-comb stand-in in lib.rs::Reverb; the sceSdEffectAttr layout is partially mangled in the de... |
| 0xd544 | PS2_SendKeyONKeyOff | - | unverified | - | audio; partial; the key-on/key-off dispatch; the 500 µs rear-voice delay is ported (Cmd::Play { delay }), the ENVX snapshot is standin (Voice::playing flag, not the ENVX reg... |
| 0xda1c | PS2_CheckKeyOn2 | - | unverified | - | audio; standin; Voice::playing (not the ENVX register snapshot); STANDIN in lib.rs. |
| 0xdc68 | PS2_CheckKeyOn | - | unverified | - | audio; standin; same. |
| 0xe420 | psiRequestVoiceHandle2 | - | unverified | - | audio; ported; Driver::start_sample (ADSR1=0x000F, ADSR2=0x1FC0, VoiceLooping, the 48 HW voices, 100 ES handles). |
| 0xe648 | psiRequestVoiceHandle | - | unverified | - | audio; ported; (the ES handle pool). |
| 0xead4 | psiSampleKeyOff2 / psiSampleKeyOff | - | unverified | - | audio; partial; Cmd::Stop → now begins the ADSR release (0075); was a hard stop before. |
| 0xebcc | PS2_VoiceSetReverb | - | unverified | - | audio; ported; the reverb bool on Cmd::Play. |
| 0xece4 | psiSampleKeyOn2 / psiSampleKeyOn | - | unverified | - | audio; ported; Cmd::Play (the key-on with delay). |
| 0xeeec | psiSetSampleFrequency | - | unverified | - | audio; ported; Cmd::Set { pitch }. |
| 0xf018 | psiSetSampleVolume* | - | unverified | - | audio; ported; Cmd::Set { left, right }. |
| 0xf46c | psiSamplePause / psiSampleUnPause | - | unverified | - | audio; not-started; the pitch-0 / restore path (G-032). |
| 0x10250 | PS2_SquareRoot | - | unverified | - | audio; ported; ps2_sqrt (10 Newton steps, truncating division). |
| 0x103bc | PS2_DotProduct | - | unverified | - | audio; ported; inlined in levels (Σ/1000). |
| 0x10470 | PS2_NormaliseVector | - | unverified | - | audio; ported; inlined in levels. |
| 0x1060c | PS2_SFXCalculate3D | - | unverified | - | audio; ported; Driver::levels (the Pan1/Pan2/Pan3 math, quadratic falloff). |
| 0x10eb8 | PS2_SFXSetup3D / PS2_SFXSetup2D | - | unverified | - | audio; ported; Driver::levels + the Cmd::Play volumes. |
| 0x11328 | PS2_GetSampleVolume | - | unverified | - | audio; ported; inlined in levels (GSV). |
| 0x109520 | psiPreGame_Run / psiPostGame_Run | nf-audio (its own thread) | unverified | - | frame/triggers; standin; The port runs audio on a host thread instead of the IOP; per-frame ordering with the tick is not the game's. |
| 0x10b2b0 | PS2WorldViewVU1Kick2 | - | unverified | - | level load; standin; the depth zb = N1 + (F1−N1)·n·(f/d−1)/(f−n) is computed in the shader; the VU1 microcode is not ported. STANDIN in gpu.rs. |
| 0x10b960 | psiDrawFogPass | - | unverified | - | level load; ported; the fog screen pass in the shader (gpu.rs); the floor(zb/256)/128 factor is now clamped to [0,1] (0078). |
| 0x10ba38 | psiFileLoad | - | unverified | - | level load; ported; IsoImage::read_bytes (the offset + size read). |
| 0x10bd08 | psiFileOpen | - | unverified | - | level load; ported; disc::files_bin_entries (the FileList lookup by name). |
| 0x111ed8 | UpdateMFX_* (20 updaters) | - | unverified | - | audio; not-started; only 07000005's castle_a is ported; the other 19 levels' per-level music updaters are not (each is a small switch on Music_Event indices; the port's update h... |
| 0x112d78 | UpdateMFX_01CastleA | - | unverified | - | audio; ported; Music::castle_a (07000005). |
| 0x114e70 | PickRandomCombat / PickRandomStealth | - | unverified | - | audio; ported; Music::pick_combat / pick_stealth (round-robin). |
| 0x114e70 | UpdateMusicalEvents | - | unverified | - | audio; ported; Music::update (the start/section, NIS state, attack hysteresis, death/fail, the castle_a per-level updater). |
| 0x118718 | CreateMoveRG2BA | - | unverified | - | level load; ported; (the Z-bits-to-alpha move; in the shader). |
| 0x119228 | CreateFogSprite | - | unverified | - | level load; ported; (the full-screen fog sprite; in the shader). |
| 0x11b140 | CreateSqueezeFrame | - | unverified | - | level load; standin; GAIN constant (SqueezeColourIn 0x91/0x80); the writer is unknown. STANDIN in gpu.rs. |
| 0x11e2e8 | psiPreGameRun | - | unverified | - | level load; ported; scene::fog_for (the per-level fog colour + Ps2Far1; confirmed against the decompile). |
| 0x121578 | psiFog | - | unverified | - | level load; ported; (empty in the game — no GS fog; the port's fog is a screen pass). |
| 0x12c5a0 | Bullet_init | bullet.rs::spread + Drone::fire_weapon | unverified | - | drone AI; partial; Direction = norm(norm(dir) + spherical→cartesian(r, FRand 2π, FRand π)) with r = (FRand(2s) − s)·0.0014, s = +0x24·(0 if +0x68 & 0x400000, else Drone+0xC0=1)... |
| 0x12cd98 | Bullet_handle_object_destruction | bullet.rs::Bullet::update | unverified | - | drone AI; stub; When a bullet hits, state = 2 (dead). The per-object destruction dispatch is not ported. |
| 0x12ce80 | Bullet_CollisionHandler | — | unverified | - | drone AI; standin; The ricochet path is not ported. The grenade bounce in grenade.rs::Grenade::update is the only bounce modelled. |
| 0x12d890 | Bullet_DoTrails | - | unverified | - | audio; ported; Bullet::update fly-by check (the +0x6C & 0x40000 gate, the closest-point-to-viewer, Rand_Rand(5), sound 0x1dc). Not ported: the smoke (Gas_Init), the 0x10000... |
| 0x12e660 | Bullet_Delete | bullet.rs::Bullet::update | unverified | - | drone AI; stub; state = 2 marks the bullet for removal in Drones::update_player_bullets. |
| 0x12e7b0 | Bullet_update | bullet.rs::Bullet::update | unverified | - | drone AI; partial; First tick (0.5 + FRand 0.5)·speed; subsequent ticks speed; dies past range; collision ray max(step, 1); nearest of world and player wins. *Deviation*: the p... |
| 0x1336f8 | control_movement_object_handler | drone.rs:3684 (tick) | unverified | - | frame/triggers; partial; Steps 5 (cel channel watch), 8 (delete-pass pointer scrubbing), 9 (angle-wrap + prev-transform copy is implicit per-object; the +0x40-teleport pass absent), ... |
| 0x134020 | control_create_object | scene.rs build + per-system create | unverified | - | frame/triggers; partial; Only the placement-driven creators exist; runtime control_create_object callers (spawner drones, bullets, explosions) use the port's own lists. |
| 0x134310 | Init_SwitchChannels | triggers.rs:68 (Channels::default) | unverified | - | frame/triggers; partial; _time starts at 0 in the port (absolute base — only differences matter, so SEQ_AND is unaffected); live [L ee_start_05.bin@0x270190] every entry holds 108150... |
| 0x134698 | control_add_object_to_list | implicit (sort + pickups first) | unverified | - | frame/triggers; partial; See the walk row. |
| 0x135440 | Door_SetupSwing | - | unverified | - | level load; ported; the swing-speed 0.5/(int)(0.5/2s). |
| 0x135738 | Door_Interp | - | unverified | - | level load; ported; door::interp (Catmull-Rom / slerp over the 28-byte keys). |
| 0x135980 | Door_ReactToHit | - | unverified | - | level load; partial; the anti-crush (door::Door::react_to_hit); the proxy-list path is standin (U-043). |
| 0x135be0 | Door_Activate | - | unverified | - | level load; ported; door::Door::activate (the swing/slide state machine). |
| 0x135d40 | Door_Link2Portal | - | unverified | - | level load; not-started; the door→portal link (G-016, block 0x21). |
| 0x135fe0 | Doors_Calc | - | unverified | - | level load; ported; door::Doors::calc (group sync). |
| 0x136100 | Door_Create | - | unverified | - | level load; ported; door::Door::new (the runtime record: key count/rate, the anim, channels). |
| 0x1363c8 | Door_Update | - | unverified | - | level load; ported; door::Doors::update. |
| 0x136be0 | Drone_Create | drone.rs::Drone::new | unverified | - | drone AI; partial; Placement → drone fields; the obj_tag allocation is not modelled. |
| 0x136ce8 | Drone_Delete | drone.rs::Drones::tick (drop) | unverified | - | drone AI; partial; Drones flagged gone are dropped from the list. |
| 0x1376c0 | Drone_AIVolume_Create | aivolume::build | unverified | - | drone AI; ported; Placements 0xF5 / 0xF8 → AIBoxList. The half-extents, kind, factor, and the 07000005/param-1 rotation flip + position lift are ported. |
| 0x1378d0 | DroneSpawner_Create | spawners::Spawner::new | unverified | - | drone AI; ported; Placement → Spawner; modes 2/3, group, max_alive, waves, start/stop/done/kill channels, min_dist. |
| 0x137aa0 | DroneSpawner_Init | spawners::Spawner::new | unverified | - | drone AI; ported; The init from the placement parameters. |
| 0x137cb0 | DroneSpawner_SpawnDrone | spawners::Spawner::update (Action::Spawn) | unverified | - | drone AI; ported; The spawn action — picks an unused template; the spot-OK test (min_dist from the player, not visible). |
| 0x137e68 | DroneSpawner_Control | spawners::Spawner::update | unverified | - | drone AI; ported; The per-tick control: start/stop channels, waves, max-alive, kill mode. |
| 0x1383e8 | Drone_EnableAll | drone.rs::Drones::tick | unverified | - | drone AI; partial; The enable / disable from switch channels. |
| 0x1386d8 | Drone_IsCoverNodeUsable | cover.rs::CoverNode::usable | unverified | - | drone AI; ported; Switch gates; thresholds (low 30°/45°, corner 0.66·A/A+5°; +5° occupied); ray from the target's head to node + (0,1.5,0) (1.15 on 0x07000011); the +0x90 per-... |
| 0x138e78 | Drone_ProcessCoverNodes | drone.rs::Drones::tick | unverified | - | drone AI; ported; Every 5 frames from the first; shared cursor; stops after the first LOS ray. |
| 0x138f68 | Drone_GetOpponentInfo | drone.rs::opponent_info | unverified | - | drone AI; partial; +0x1A0 distance; +0x1C4 = ATAN2_APPROX(dx, dz). |
| 0x1390c0 | Drone_ProcessOpponents | drone.rs::Drones::tick | unverified | - | drone AI; partial; Bond's visibility factor from the AI volumes holding him (× 0.75 crouching — *deviation*: Bond does not crouch in the port); each drone's opponent info. |
| 0x139268 | Drone_Control | drone.rs::Drones::tick | unverified | - | drone AI; partial; The per-drone control: PreDroneControl → ControlSTANDARD → SnapRotate → animation → CallHandler. |
| 0x139520 | Drone_InitComms | drone.rs:3713-3748 (opponent info, light, corpse culling) + the hearing/alarm blocks 3810-3855 | unverified | - | frame/triggers; partial; No Drone_PauseNoDraw/NPCGlobals equivalent (the port's paused flag covers the freeze, see §7 item 3); the +0x154 four-player distance array is folded into op... |
| 0x139f50 | Drone_ModBulletDamage | — | unverified | - | drone AI; stub; The per-weapon bullet damage mod — folded into damage::hit_damage. |
| 0x13a0a0 | Drone_DCVfromOBJ | drone.rs (implicit field access) | unverified | - | drone AI; partial; The DCVars scratch is not modelled (the port works on Drone directly). |
| 0x13a630 | Drone_SM_SendMsg | drone.rs::Drone::send | unverified | - | drone AI; partial; The send path. |
| 0x13a9f8 | Drone_BulletHit | drone.rs::Drones::update_player_bullets | unverified | - | drone AI; partial; Bond's bullet → drone hit; the Collide_PickObj/Collide_Jointy path. |
| 0x13aab8 | Drone_ExplosiveHit | drone.rs::handle_death(*, msg::EXPLOSIVE) | unverified | - | drone AI; partial; Msg 9 → Drone_ExplosiveHit → HitDamage with the explosive damage; dead → Death_Anim. |
| 0x13ab58 | DroneAnim_CallAnim | drone.rs::call_anim / call_anim_variant | unverified | - | drone AI; ported; (blend, state, variant, next AI state, completion message) — the variant path picks consecutive danims of a record (e.g. 0x4B × 3). |
| 0x13add0 | DroneAnim_CallHandler | drone.rs::call_handler | unverified | - | drone AI; ported; Every frame after the anim tick; once a danim of the requested state started it dispatches the completion message. |
| 0x13aee8 | DroneAnim_SetDAnimInternal | drone.rs::set_danim | unverified | - | drone AI; ported; The danim's state, class and call bookkeeping; then SetScript for the danim's script. |
| 0x13b1c8 | DroneAnim_SetHTAnim | drone.rs::begin_blend | unverified | - | drone AI; partial; The hand-track script's blend switch (no blend / drone not animating → set DAnim; else SetHTAnim's blend). |
| 0x13b490 | DroneAnim_PreTransCallback | drone.rs::root_motion | unverified | - | drone AI; partial; Root motion clipping at boundaries; the free_root path (SpecialDeath) keeps the whole root motion and adds it to +0xBE0. The +0x4C4 anim yaw offset is not po... |
| 0x13b750 | DroneAnim_EventFunc | drone.rs::event_func | unverified | - | drone AI; partial; Events 3/4 holster swap, 7 +0x579=0, 0xF +0x20, 0x12 +0x20/+0x21, 0x10 clears, 0/4/5 footsteps, 1 the grenade-throw release → DroneWeap_ThrowGrenade. |
| 0x13b9e0 | DroneAnim_Can*Lean/StepOut (4 funcs at 0x13b9e0, 0x13bb38, 0x13bc60, 0x13bde8) | drone.rs::cover_types | unverified | - | drone AI; partial; The four cover anim predicates (crouch lean, crouch step, stand lean, stand step) are folded into one cover_types helper. |
| 0x13bf40 | DroneAnim_ValidateCoverAnim | drone.rs::cover_anim (the can_do/fallback path) | unverified | - | drone AI; standin; The 5 → 7 (or 10) fallback for an anim the set lacks is not ported. The port picks 0xA when 5 is missing (a one-step approximation). |
| 0x13c078 | DroneAnim_CoverAnim | drone.rs::cover_anim | unverified | - | drone AI; partial; The CCModes (0x270768) table is ported (cover::cc_anim). Mode 6 picks the type; mode 7 is the maybe-change. The crouch/stand lean/step predicates are ported ... |
| 0x13c780 | DroneAnim_LocationDeathAnim | drone.rs::location_death_anim | unverified | - | drone AI; partial; Bond aiming and part 4/5 → 0x42 (variant 1 when shot from behind, else 0); otherwise +0x3A=0, 0x40 variant Rand(4) if the danim's info+0x10 byte is set. |
| 0x13c8b8 | DroneAnim_LocationImpactAnim | drone.rs::location_impact_anim | unverified | - | drone AI; partial; CallAnim(0x4C, variant, next = +0x118); variant 5 + Rand(8) unless Bond aims, then by part: 4/5 → 5, 0x12–0x15 → 8, 0x1E/0x1F/0x20/0x23 → 9, 0x31–0x33 → 13, ... |
| 0x13ca68 | DroneAnim_SetStandIdleAnim | drone.rs::set_stand_idle | unverified | - | drone AI; partial; (SM id + 1) % 3 + 0x37 for patrol; sets 0x37 when the set lacks it. |
| 0x13cc70 | DroneAnim_SetWalkIdleAnim | drone.rs::walk_anim | unverified | - | drone AI; partial; The walk idle anim from the anim set. |
| 0x13ce00 | DroneAnim_GetMoveQuad | drone.rs::move_quad | unverified | - | drone AI; ported; 0 forward, 1/3 sideways, 2 backward, with hysteresis on previous. |
| 0x13cfb0 | DroneAnim_SetCombatMoveAnim | drone.rs::set_combat_move_anim | unverified | - | drone AI; partial; a = diff(yaw to move target, yaw to opponent) → quad with hysteresis; quad 0/2/1/3 → anim 8/0xF/0x1E/0xC/0xD; rolls with prop 0x2E; steps with prop 0x42. +0x... |
| 0x13da48 | DroneAnim_PlayFiringAnim | — | unverified | - | drone AI; stub; The +0x20/+0x21 flag-driven firing anim — not ported. |
| 0x13dae8 | DroneAnim_CheckForAnimComplete | drone.rs::update_anim | unverified | - | drone AI; partial; +0x57A set and +0x579 cleared by event 7 → SetEndAnim now. |
| 0x13dc90 | DroneAnim_ZeroAnimCall | drone.rs::call_anim | unverified | - | drone AI; partial; The pending-call zero on a playing-state request is ported. |
| 0x13dd00 | DroneAnim_SetEndAIState | drone.rs::handle(*, msg::ANIM_DONE) | unverified | - | drone AI; partial; Sets +0x118 (resume). |
| 0x13df38 | DroneAnim_SetEndAnim | drone.rs::handle(*, msg::ANIM_DONE) | unverified | - | drone AI; partial; Sets the end-anim flag. |
| 0x13dfc8 | DroneAnim_CanDoAnimState | drone.rs::can_do | unverified | - | drone AI; partial; The anim state's own danim has a script in our anim set (or set 0). |
| 0x13e088 | DroneAnim_SetScript | drone.rs::play_script | unverified | - | drone AI; ported; (script, state, follow-on, …, next AI state, message, flags). |
| 0x13e198 | DroneAnim_SnapRotate | drone.rs::Drones::tick | unverified | - | drone AI; partial; Applied to the yaw and the desired yaw; info flags 0x100 → π, 0x80 → π/2, 0x200 → −π/2. |
| 0x13e238 | DroneAnim_OnStartAnim | drone.rs::set_danim | unverified | - | drone AI; partial; Class 6 without a big hit (+0x1C) is cut to 15 frames, end blend 5. |
| 0x13e2c0 | DroneAnim_OnEndAnim | drone.rs::event_func | unverified | - | drone AI; partial; The +0x54C turn left by a finished turning danim; events are dispatched in event_func. |
| 0x13e3b0 | DroneAnim_InTransition | drone.rs::in_transition | unverified | - | drone AI; ported; The playing danim's class is above 3 and not 8. |
| 0x13e3f8 | DroneAnim_ForceCrouchCover | drone.rs::cover_types | unverified | - | drone AI; partial; Prop 0x10 or a corner without 8 → forced crouch. |
| 0x13e450 | DroneAnim_CanUseCornerAsLowCover | drone.rs::as_low | unverified | - | drone AI; partial; Low node or crouch-only corner. |
| 0x13e988 | NDrone2_PatrolTalk | drone.rs::patrol_talk | unverified | - | drone AI; partial; Every patrol frame while the global cooldown is open: Rand(2) ≠ 0 → cooldown set; type-3 drones speak. |
| 0x13eac0 | NDrone2_AttackTalk | drone.rs::attack_talk | unverified | - | drone AI; partial; Type 2 picks 0x37 + Rand(9) (07000005–08). |
| 0x13eeb8 | NDrone2_PainTalk | drone.rs::pain_talk | unverified | - | drone AI; partial; Forced; skipped only when the same line is the current one. |
| 0x13eff8 | NDrone2_DeathTalk | drone.rs::death_talk | unverified | - | drone AI; partial; Forced; the second call of a death sees the first's line and is skipped. |
| 0x13f1d0 | NDrone2_SeenPlayerTalk | drone.rs::seen_player_talk | unverified | - | drone AI; partial; 1 in 3, then the level's set (05–08: type 2 0x49 + Rand(6), else 0x21). |
| 0x13f688 | NDrone2_HandleTalking | drone.rs::speech | unverified | - | drone AI; partial; SpeechSFX(idx, force, 0, 0) → PlaySFX(2.0, …); the first-30-frame close, the repeat-delay gate, the facial-anim refusal, the line stamp. |
| 0x13fff0 | NDrone2_DealWithObjHit | drone.rs::update_player_bullets | unverified | - | drone AI; standin (case 5 partial); The per-hit dispatch (water ripples, doors, breakables, explosions, vehicle riding) is not ported; the explosion hit path is reached separately from Explode_... |
| 0x13fff0 | NDrone2_DealWithObjHit case 5 | drone.rs::update_player_bullets | unverified | - | drone AI; partial; The player-shooter arm only: the weapon picks the message — 1/0x4B/0x4D → msg 6 (punch), 0x4A/0x4C → msg 7 (taser, not ported), else Drone_BulletHit msg 8 [M... |
| 0x140660 | Drone_FeetOnPoint | drone.rs::Drone::feet_on_point | unverified | - | drone AI; standin; The down-ray pos → pos + up·−(hip+0.6) with mask 0xC and the on-surface threshold (floor ≥ feet_y − 0.2) are ported; the bVar2 extension that reads 0x3037dc ... |
| 0x140968 | Drone_CollisionHandler | drone.rs::Drone::collision_handler | unverified | - | drone AI; standin; The driver is ported (in-air flag clear, down-ray, on-surface snap, gravity accumulation, pos += v·REC_FRAME_RATE, obj+0xFA |
| 0x140e90 | NDrone2_Collision | drone.rs::Drone::collision_handler + nav::Network::push_out | unverified | - | drone AI; partial; The feet-sphere push-out of AI-network boundary walls (GetBoundsPushVectorForSphere, r 0.4) runs via Network::push_out in Drones::tick; the per-hit walkable/... |
| 0x141848 | NDrone2_PlaySFX | - | unverified | - | audio; not-started; drone foley (Drone_SfxInfo, the 16-entry ring). G-032. |
| 0x141b00 | NDrone2_SpeechSFX | - | unverified | - | audio; not-started; drone speech (the facial-anim path). G-032. |
| 0x141e58 | NDrone2_DoGravity | drone.rs::Drone::collision_handler (inlined predicate) | unverified | - | drone AI; partial; Same predicate pattern as DoCollision, with & 0x80. The port's bounded flag covers both & 0x20 and & 0x80. |
| 0x141f28 | DroneFunc_AllocateTargetID | drone.rs::target | unverified | - | drone AI; partial; The player target id 0. |
| 0x1422d0 | NDrone2_SetOpponent | drone.rs::opponent_info | unverified | - | drone AI; partial; Single player → the player. |
| 0x1424e0 | NDrone2_FindOpponent | - | unverified | - | level load; ported; the opponent pointer (Bond) is set for all drones in SP. |
| 0x1431c0 | DroneFunc_ConsiderExplosive | drone.rs::consider_explosive | unverified | - | drone AI; partial; Stores +0x330 (pos) / +0x350 (12 frag / 5 stun / 10 mine); routes to 0x86. The flee route itself is the STANDIN above. |
| 0x145150 | NDrone2_SeenAndAttacking | - | unverified | - | audio; ported; AttackGate::seen_and_attacking (the count/floor/window/distance vars, the latch). |
| 0x1453a8 | DroneFunc_CheckAlarmRaised | drone.rs::Drones::tick (music) | unverified | - | drone AI; partial; Channel 0xA4 (07000005/06: 0x37 / 0x61) raises the alarm-music gate; the per-level channel set is ported. |
| 0x1458e0 | NDrone2_DoHitEffects | drone.rs::do_hit_effects | unverified | - | drone AI; partial; Damage > 0.5 → sound 0x413 at the hit (0x601 when HITDATA+0x52 = 5 — *not modelled*). |
| 0x145bd0 | DroneFunc_OnInitDeath | drone.rs::handle_death(*, ENTER) | unverified | - | drone AI; stub; +0xCD8 = 1 flag set; the per-mode +0x4F8 mask writes are not all ported. |
| 0x145f28 | NDrone2_PunchImpact | drone.rs::punch_impact | unverified | - | drone AI; ported; The resume stamped into +0x118 before the damage; health ≤ 0 → KnockedOut_Anim 0x42 (mode 0x1E → 0xF2, bots not ported); alive: PainTalk, then the 0x51 stagg... |
| 0x1461c8 | NDrone2_BulletImpact | drone.rs::bullet_impact | unverified | - | drone AI; partial; +0x50C = 1; alert status 2; DoHitEffects → HitDamage; dead → 0x44; alive with allow → flinch when ≥ 2 s since the drone's / global's last flinch. |
| 0x1469b8 | DroneFunc_HandleImpact | drone.rs::on_bullet/on_punch/bullet_impact/punch_impact | unverified | - | drone AI; partial; The per-state impact table (Idle/Alert/Patrol/AlertToPosition/SearchArea/HeardNoise*/RunForCover/UnderCoverInit/LeaveNow → 0x56; UnderCover Idle/Aim/Fire/Ret... |
| 0x147c48 | DroneFunc_FirstSightState | drone.rs::handle(ATTACK, ENTER) (inlined) | unverified | - | drone AI; partial; The prop 0x1B → 50 % combat move / prop 0x19 path is ported. |
| 0x148010 | DroneFunc_FirstAttack | drone.rs::handle(ATTACK, ENTER) | unverified | - | drone AI; partial; CheckSurrender → alertness 1, +0x4F8 |
| 0x148338 | DroneFunc_CombatState | drone.rs::combat_state | unverified | - | drone AI; partial; Every call routes to the opponent; from 0x58 the 0x59/0x5A/0x5C/0x5F branches; from 0x68 reload/NoSight/combat move; from 0x70 2-s timer. |
| 0x148a30 | DroneFunc_HandleSoundAlerts | drone.rs::Drones::tick (sound loop) | unverified | - | drone AI; partial; share = 0.01·loudness; loudest sound → msg 0x14 at most once a second. The id 0x248 weight-10 override and the 0x11C/0x11D 09/0A skip are ported. |
| 0x148c60 | DroneFunc_DoForcedAttack | drone.rs::handle(*, msg::FORCED_ATTACK) | unverified | - | drone AI; stub; Always → Attack. |
| 0x148db0 | NDrone2_PreDroneControl | drone.rs::Drones::tick | unverified | - | drone AI; partial; The timer → message dispatch; the second-switch (+0x135) with a tag (+0x13A) → 0xC deletes. |
| 0x1490f0 | NDrone2_ControlSTANDARD | drone.rs::Drones::tick | unverified | - | drone AI; partial; Alertness decay; msg 3; find opponent; tracking; move; collision; DroneWeap_HandleFiring. |
| 0x1497e8 | DroneFunc_ChooseSFX | - | unverified | - | audio; ported; Rng::rand(b-a+1) (the random line index). |
| 0x1499d0 | DroneFunc_ClampPosToCollisionCylinder | — | unverified | - | drone AI; not started; The clamp-into-cylinder adjustment for slide moves is not ported. |
| 0x149dd0 | NDrone2_Enable | drone.rs::Drone::enabled | unverified | - | drone AI; partial; obj+0x30 &= ~0x30; +0x4F8 |
| 0x149ec8 | NDrone2_DoCollision | drone.rs::Drone::collision_handler (inlined predicate) | unverified | - | drone AI; partial; The "alive, & 0x20, not in death +0xD8 ≠ 0xC/0x13, no +0x538 blend, drawn, & 0x570" predicate is folded into bounded/free_root/corpse. |
| 0x149f88 | NDrone2_DoAnimation | drone.rs::update_anim | unverified | - | drone AI; partial; A drone that was not drawn (obj+0xFC = 0) playing a danim with info flag 4 and no blend slides along its yaw by Drone_AnimInfo +4 per frame. The port approxi... |
| 0x14a060 | NDrone2_CheckSurrender | drone.rs::check_surrender | unverified | - | drone AI; partial; Prop 0x43, alertness < 0.66, Bond within the range. |
| 0x14a168 | NDrone2_CheckUnsurrender | drone.rs::check_unsurrender | unverified | - | drone AI; partial; Farther than 12, or beyond 2 with Bond unarmed, or Bond not facing the drone. |
| 0x14a218 | NDrone2_FadeOut | drone.rs::handle_death(FADE/FADE_FAST, …) | unverified | - | drone AI; partial; Alpha decrement. |
| 0x14a298 | DroneFunc_DeadDrone | drone.rs::Drones::tick (corpse cull) | unverified | - | drone AI; partial; Counts 0x0F/0x47/0x48(+300)/0xBF; the oldest goes FadeFast beyond 3 (DAT_00298F5C). |
| 0x14a2f0 | NDrone2_SetAsDead | drone.rs::handle_death(DEAD, ENTER) | unverified | - | drone AI; partial; obj+0xF0 |
| 0x14a5f0 | DroneFunc_Set1stBehaviour | drone.rs::handle(*, msg::FIRST_SIGHT) | unverified | - | drone AI; partial; +0x4D8 → +0x4DC. |
| 0x14a618 | DroneFunc_Set2ndBehaviour | drone.rs::handle(ATTACK, ENTER) | unverified | - | drone AI; partial; +0x4D8 → +0x4E8. |
| 0x14a718 | DroneFunc_AnyoneNear | drone.rs::Drones::tick (bunched) | unverified | - | drone AI; partial; Another drone within 3 units. |
| 0x14ac60 | DroneFunc_ReactionTime | drone.rs::reaction_time | unverified | - | drone AI; partial; (1 − alert)·0.3·(200 − stat_b8) on 0x7000002/3/5/9/A, else 10. [L] 07000005: 60 ticks. |
| 0x14af80 | DroneFunc_AttackNoWeapon | drone.rs::handle(ATTACK, ENTER) | unverified | - | drone AI; stub; → DrawWeapon when unarmed. |
| 0x14b158 | DroneFunc_DropGrenade | drone.rs::drop_grenades | unverified | - | drone AI; partial; Captain death drops weapon 0x35 at its matrix. |
| 0x14b190 | DroneFunc_NearPlayerCollision | — | unverified | - | drone AI; stub; Always returns false in the port (no player-proximity short-circuit on the full sphere test). |
| 0x14b1e8 | DroneInit_Collision | — | unverified | - | drone AI; stub; The create-time floor snap is not ported; the port stands drones on the rendered floor at level load. |
| 0x14b300 | NDrone2_DefaultInit | drone.rs::Drone::new | unverified | - | drone AI; partial; +0xAC health 10.0; +0x14 damageable 1; +0x14C 1.0; +0x100 1.0; placement parameters 3/6/8/12/13; captain +0x19. |
| 0x14c938 | NDrone2_GetDTYPENEW | drone.rs::dtype_of | unverified | - | drone AI; partial; Mode for type ty and a block's mode block (0 = the base); types 10–16 special; block 2 → 2 (sniper) or 0x19 at alertness 1; etc. |
| 0x14cbd8 | NDrone2_GetDroneTypeAttackTypeFriend | - | unverified | - | level load; ported; drone.rs:609 (the team field: dtype 10/0xb/0xd/0xe → 3, 0xc → 2, else 1). |
| 0x14eba8 | NDrone2_Steer | drone.rs::face_and_aim | unverified | - | drone AI; partial; yaw += Vec_AngleDifference(yaw, desired) · 0.1. |
| 0x1514d8 | NDrone2_UpdatePatrolRoute | drone.rs::update_patrol_route | unverified | - | drone AI; partial; FollowRoute; result 0 → walk anim (0x35 when prop 0x25 and alert ≥ 0.66), 7 → stand idle, else walk / 0x21. |
| 0x151618 | NDrone2_MoveToObject | drone.rs::move_to_object | unverified | - | drone AI; partial; Verify / compute the route; 3 within 2.0; follow. |
| 0x1517d8 | NDrone2_MoveToGoalPosition | drone.rs::move_to_goal | unverified | - | drone AI; partial; Routes only when the combat route is invalid; results 0 moving, 3 arrived, 0xC blocked, 10 door, 4/5/6/8 failed. |
| 0x151900 | NDrone2_SetAngleToCover | drone.rs::set_angle_to_cover | unverified | - | drone AI; partial; Low / as-low → opponent; side 1 → node yaw − π/2; side 2 → node yaw. |
| 0x151d68 | NDrone2_CoverNodeOK | drone.rs::cover_node_ok | unverified | - | drone AI; partial; Bits & 0xE; with needLos & 0x70; distance ≥ 2.0 (and ≤ 1.5× combat range with prop 0x44); one cover anim possible; no other drone within 1.5. |
| 0x152028 | NDrone2_GetPosForCornerCover | cover.rs::CoverNode::position | unverified | - | drone AI; ported; Side 1: x = 0.1 (approach −0.7), z = −depth. Side 2: x = offset, z = −0.1 (approach +0.7). Low: x = 0, z = −0.3 (approach −1.1). |
| 0x152548 | NDrone2_FindCover | drone.rs::find_cover | unverified | - | drone AI; partial; The unoccupied node on the drone's network with the smallest path-distance d ≤ distance to opponent; claims owner/side/approach. |
| 0x152958 | NDrone2_FindAmbushCover | drone.rs::find_ambush_cover | unverified | - | drone AI; partial; FindCover restricted to nodes within 2.0 units of the drone. |
| 0x153720 | NDrone2_InitAIPath | nav.rs::Route::for_path | unverified | - | drone AI; partial; Mode 2 (loop) for kind 4, else 1; kind 0x10 sets route bit 0x20. The first-node-of-a-kind-4-path-without-0x10 second-link rule is ported. |
| 0x154190 | NDrone2_AssignAIPath | drone.rs::assign_path | unverified | - | drone AI; partial; The nearest node (2D, < 0.3) of a path with kind & mask → InitAIPath. |
| 0x1543f8 | NDrone2_SetGoalToDroneWithLosToOpponent | drone.rs::set_goal_to_drone_with_los | unverified | - | drone AI; partial; The first other drone (list order) that sees the opponent and wins Rand(4) == 0 and MoveToGoalPosition gives 0 or 2. |
| 0x154bd0 | NDrone2_SlideToPosition | drone.rs::slide_to / slide_free | unverified | - | drone AI; partial; x/z toward the point (snapping within 0.01); the 3D variant (the special death) is slide_free. |
| 0x154d58 | NDrone2_SetAngleToObj | drone.rs::combat_state (inlined) | unverified | - | drone AI; partial; Face the opponent; aim / look-about. |
| 0x1551c0 | NDrone2_MoveTest (vector) | nav.rs::Network::move_test | unverified | - | drone AI; partial; length² ≤ 900; |
| 0x155858 | NDrone2_DisownCover | drone.rs::disown_cover | unverified | - | drone AI; partial; Clears owner + flag 0x10 + +0x4F8 &= ~0x1800. |
| 0x155a38 | NDrone2_CoverAvailable | drone.rs::cover_available | unverified | - | drone AI; partial; Prop 0x30 = 0 → no; already claimed → yes; opponent visible and < 4 → no; else FindCover. |
| 0x155be0 | NDrone2_FollowRoute | nav.rs::Route::follow | unverified | - | drone AI; partial; The creep; target → +0x670; desired yaw +0x694 = ATAN2_APPROX to it. |
| 0x155cd8 | NDrone2_NavNodeCache | drone.rs::Drones::tick (target cache) | unverified | - | drone AI; partial; The player's goal nodes per navigation path, rebuilt when he has moved ≥ 2.0 (search radius 6 = min(engage, 6)). |
| 0x155e10 | NDrone2_SetGoalToCloserPosition | drone.rs::set_goal_to_closer_position | unverified | - | drone AI; partial; Where the straight drone → opponent line first meets an AI-network wall. |
| 0x156760 | AINetwork_OptimiseRoute | nav.rs::Route::to_position (drop node 0 / last) | unverified | - | drone AI; partial; Drop node 0 if MoveTestToNode(start → node[1], near) = 1 and the pair is not both door / both kick; drop the last likewise. |
| 0x156ba8 | AINetwork_CalcRoute | nav.rs::Route::to_position | unverified | - | drone AI; partial; MoveTest(start, dest, near 1) = 1 → direct route (status 2); else A*. |
| 0x156e18 | AINetwork_NodeSearchCel | nav.rs::Network::candidates | unverified | - | drone AI; partial; Box ±25 from the start; cel lists not decoded (one cel). |
| 0x1571b0 | AINetwork_DoAStarPath | nav.rs::Network::astar | unverified | - | drone AI; partial; The A* with cost = g + len × (used + 1); the four decompiler bugs (no re-sort of open nodes, the open bit 0x40000000) are kept. |
| 0x157780 | AINetwork_HitBoundry | nav.rs::intersect_adjust | unverified | - | drone AI; partial; The 2D segment intersection. |
| 0x157990 | AINetwork_LineIntersectsBoundry | nav.rs::intersect_adjust | unverified | - | drone AI; partial; The wall y-range test (yWall − 2.4 ≤ yLine ≤ yWall + hWall). |
| 0x157bd0 | AINetwork_GetBoundsPushVectorForSphere | nav.rs::Network::push_out | unverified | - | drone AI; partial; The push-out vector for the feet sphere (r 0.4) out of boundary walls (no flags 0x1C00). The push itself is *suggested*. |
| 0x157d80 | AINetwork_BoundsTest | nav.rs::Network::bounds_test | unverified | - | drone AI; partial; The portal connectivity part (Collide_StraddleCels) is *deviation*: a collision-world line-of-sight at +1 stands in. |
| 0x158140 | AINetwork_BoundsNodeTest | nav.rs::Network::bounds_node_test | unverified | - | drone AI; partial; The post-within-√thr test (thr = 0.5·r² when the post is within √1.3·r of A or B, else r²). |
| 0x158488 | AINetwork_NavPathForPosition | nav.rs::Network::nav_path_for | unverified | - | drone AI; ported; The nearest node (2D) of a path with kind & 0xD == 0 in the drone's cel. |
| 0x1586a8 | AINetwork_NodesForPosition | nav.rs::Network::target_nodes | unverified | - | drone AI; ported; The cel's nodes in a box (±√r² or ±15 in x/z, ±2 in y), nearest first (2D); the first with MoveTestFromNode(node → pos) = 1 is "found". |
| 0x158cf8 | AINetwork_EmitPath | nav.rs::Network::emit | unverified | - | drone AI; ported; Per nav node a path-distance byte (0xFF = unreachable). |
| 0x159668 | AINetwork_SetupNextNode | nav.rs::Route::next_node | unverified | - | drone AI; partial; Mode 2 wraps; mode 0 moves to the dest. |
| 0x15a490 | AINetwork_UpdateLinksUsedCount | nav.rs::Route::mark_used | unverified | - | drone AI; partial; Adds 1 to every remaining link of a followed route. |
| 0x15a5c8 | AINetwork_InitPassableBoundries | nav.rs::Network::init_walls | unverified | - | drone AI; ported; Crossed by any path link → 0x400 passable. |
| 0x15a8d8 | AINetwork_FurthestPosition | nav.rs::Network::first_wall_hit | unverified | - | drone AI; partial; Every BLINK, no filters, nearest intersection to the drone; y = post a's y. |
| 0x15aa88 | AINetwork_NodeSearch | nav.rs::Network::node_from | unverified | - | drone AI; partial; The nearest node (2D, < 0.3) of a path with kind & mask. |
| 0x15b110 | AINetwork_FollowRoute | nav.rs::Route::follow | unverified | - | drone AI; ported; Remaining distance; arrival radius; the link creep. |
| 0x15b498 | AINetwork_ClearLinkFlags | drone.rs::Drones::tick (zero used) | unverified | - | drone AI; partial; ClearLinkFlags(0x310) in Drone_InitComms — the per-frame used-link zero. |
| 0x15b830 | NDrone2_DSTATE_Global | drone.rs::handle(GLOBAL, …) | unverified | - | drone AI; partial; Switch-channel → WaitSwitch; start state +0x5A2. Modes 0x1E/0x17 and PlayScript (3) are not ported. |
| 0x15b960 | NDrone2_DSTATE_WaitSwitch | drone.rs::handle(WAIT_SWITCH, …) | unverified | - | drone AI; partial; Enable(0) (obj+0x30 |
| 0x15bf18 | NDrone2_DSTATE_Idle | drone.rs::handle(IDLE, …) | unverified | - | drone AI; partial; Stand/alert idle anim; prop 0x1D → AssignAIPath(8) (EnemyMission, none in 07000005); msg 4 idle timeout (prop 0x46). |
| 0x15c248 | NDrone2_DSTATE_Alert | drone.rs::handle(ALERT, …) | unverified | - | drone AI; partial; Anim 0x21; msg 4 → 0x22 (look around). |
| 0x15c498 | NDrone2_DSTATE_Patrol | drone.rs::handle(PATROL, …) | unverified | - | drone AI; partial; PatrolTalk, UpdatePatrolRoute, look-for-opponent; wait nodes (+0xBAE) not modelled. |
| 0x15fab0 | NDrone2_DSTATE_SniperIdle | drone.rs::handle(SNIPER_IDLE, …) | unverified | - | drone AI; partial; Anim 0x0E; stationary; look for Bond. |
| 0x15fc18 | NDrone2_DSTATE_SniperAim | drone.rs::handle(SNIPER_AIM, …) | unverified | - | drone AI; partial; Anim 0x0A; sight and combat range 1000; Bond seen → SniperFire. |
| 0x15fd88 | NDrone2_DSTATE_SniperFire | drone.rs::handle(SNIPER_FIRE, …) | unverified | - | drone AI; partial; 60-tick timer then DroneWeap_Fire; empty clip → SniperReload; 16 ticks unseen → SniperAim. |
| 0x15ff80 | NDrone2_DSTATE_SniperReload | drone.rs::handle(SNIPER_RELOAD, …) | unverified | - | drone AI; partial; 20 ticks, then reload anim 0x52 → SniperFire; clip refills on exit. |
| 0x161560 | NDrone2_DSTATE_AmbushWait | drone.rs::handle_cover(AMBUSH_WAIT, …) | unverified | - | drone AI; partial; Crouched; on opponent visible or within 3 → 0x56. |
| 0x1626a0 | NDrone2_DSTATE_Surrender_Anim | drone.rs::handle(SURRENDER_ANIM, …) | unverified | - | drone AI; partial; Drop the gun, unarmed (ChangeWeapon(1)), hands up. |
| 0x162950 | NDrone2_DSTATE_Surrendered | drone.rs::handle(SURRENDERED, …) | unverified | - | drone AI; partial; Faces Bond once; hands up while covered (CheckUnsurrender). |
| 0x162a70 | NDrone2_DSTATE_Unsurrender_Anim | drone.rs::handle(UNSURRENDER_ANIM, …) | unverified | - | drone AI; partial; Face Bond, aim-stand anim → Attack. |
| 0x1631c0 | NDrone2_DSTATE_StunGrenadeLoop | — | unverified | - | drone AI; stub; Stun-grenade receiving state — not ported. |
| 0x163350 | NDrone2_DSTATE_StunGrenadeRecover | — | unverified | - | drone AI; stub; Stun-grenade recovery — not ported. |
| 0x1637f8 | NDrone2_DSTATE_Death_Anim | drone.rs::handle_death(DEATH_ANIM, …) | unverified | - | drone AI; partial; obj+0xF0 |
| 0x163a80 | NDrone2_DSTATE_DeathByExplosion | — | unverified | - | drone AI; not started; Reached from a fatal explosion; the port routes a fatal Drone_ExplosiveHit straight to Death_Anim. |
| 0x163c68 | NDrone2_DSTATE_SpecialDeath_Anim | drone.rs::handle_death(SPECIAL_DEATH, …) | unverified | - | drone AI; partial; At its spot +0xBE0: OnInitDeath, death channel, & ~0x20/& ~0x80/& ~0x80000000, +0x1A = 1, the script as anim 0x40; else the normal death. free_root keeps the... |
| 0x163fe8 | NDrone2_DSTATE_Dead | drone.rs::handle_death(DEAD, …) | unverified | - | drone AI; partial; obj+0xF0 |
| 0x164210 | NDrone2_DSTATE_Attack | drone.rs::handle(ATTACK, …) | unverified | - | drone AI; partial; FirstAttack once, CombatState route, SeenAndAttacking music gate. |
| 0x1643e8 | NDrone2_DSTATE_CombatNoMove | drone.rs::handle(COMBAT_NO_MOVE, …) | unverified | - | drone AI; partial; A 0x58 variant with no move; not in 07000005. |
| 0x164690 | NDrone2_DSTATE_CombatOutOfRange | drone.rs::handle(COMBAT_OUT_OF_RANGE, …) | unverified | - | drone AI; partial; MoveToObject(min(range,6)), prop 0x40 → 0x59, cover → 0x8C, in range → 0x58. |
| 0x164838 | NDrone2_DSTATE_CombatNoSight | drone.rs::handle(COMBAT_NO_SIGHT, …) | unverified | - | drone AI; partial; Visible → 0x5B; unseen > 15 ticks → move toward opponent. |
| 0x164c08 | NDrone2_DSTATE_CombatNoRoute | drone.rs::handle(COMBAT_NO_ROUTE, …) | unverified | - | drone AI; partial; Anim 0x21, 1 s timer, fire; the timer's SetGoalToCloserPosition / SetGoalToDroneWithLosToOpponent alternation is ported but not the +0x31C "drone ahead" 0x5E... |
| 0x1650a0 | NDrone2_DSTATE_NoOpponent | drone.rs::handle(NO_OPPONENT, …) | unverified | - | drone AI; partial; Returns home via +0x5B0/+0x5A0. |
| 0x165270 | NDrone2_DSTATE_AlertToPosition | drone.rs::handle(ALERT_TO_POSITION, …) | unverified | - | drone AI; partial; Walk 0x35 / run 0x1E to the goal; arrived → +0x5A2; 6 → 0x5F. |
| 0x165640 | NDrone2_DSTATE_GoToGoalPosition | drone.rs::handle(GO_TO_GOAL, …) | unverified | - | drone AI; partial; MoveToGoalPosition; +0x4F8 |
| 0x165bf0 | NDrone2_DSTATE_SearchArea | drone.rs::handle(SEARCH_AREA, …) | unverified | - | drone AI; partial; Look 0x22, 3 s, turn 0x61, anim 0xB; give-up → home. |
| 0x165ef0 | NDrone2_DSTATE_DroneStuck | drone.rs::handle(DRONE_STUCK, …) | unverified | - | drone AI; stub; → 0x5F on enter; msg 3 → 0x56 (the DAT_00299024 test is always 0 in this build). |
| 0x166000 | NDrone2_DSTATE_DrawWeapon | drone.rs::handle(DRAW_WEAPON, …) | unverified | - | drone AI; partial; Anim 0x46 → +0x5A2. |
| 0x166178 | NDrone2_DSTATE_AimStand | drone.rs::handle(AIM_STAND, …) | unverified | - | drone AI; partial; Anim 0xA; reload / combat-move transitions via combat_state. |
| 0x166328 | NDrone2_DSTATE_AimStandFire | drone.rs::handle(AIM_STAND_FIRE, …) | unverified | - | drone AI; partial; HandleFiring, NextBulletTime, clip-empty → reload, no-sight, grenades, combat moves. |
| 0x1666d8 | NDrone2_DSTATE_AimStandReload | drone.rs::handle(AIM_STAND_RELOAD, …) | unverified | - | drone AI; partial; Reload anim 0x52; clip refill; back to 0x68. |
| 0x166b98 | NDrone2_DSTATE_AimBackoff | drone.rs::handle(AIM_BACKOFF, …) | unverified | - | drone AI; partial; Anim 4 (back off, offset π); 2-s timer. |
| 0x166f28 | NDrone2_DSTATE_AimCrouch | drone.rs::handle(AIM_CROUCH, …) | unverified | - | drone AI; partial; Anim 0xC; reload / fire transitions. |
| 0x167060 | NDrone2_DSTATE_AimCrouchFire | drone.rs::handle(AIM_CROUCH_FIRE, …) | unverified | - | drone AI; partial; As 0x68 with crouch. |
| 0x167370 | NDrone2_DSTATE_AimCrouchReload | drone.rs::handle(AIM_CROUCH_RELOAD, …) | unverified | - | drone AI; partial; Reload anim 0x52 crouched. |
| 0x167590 | NDrone2_DSTATE_StepAimLeft | drone.rs::handle(STEP_AIM_LEFT, …) | unverified | - | drone AI; partial; Root-motion step; ends with the anim. |
| 0x1676a0 | NDrone2_DSTATE_StepAimRight | drone.rs::handle(STEP_AIM_RIGHT, …) | unverified | - | drone AI; partial; As left. |
| 0x1677b0 | NDrone2_DSTATE_StrafeAimLeft | drone.rs::handle(STRAFE_AIM_LEFT, …) | unverified | - | drone AI; partial; Strafe 0xC, fires while moving, 2–5 s timer. |
| 0x167980 | NDrone2_DSTATE_StrafeAimRight | drone.rs::handle(STRAFE_AIM_RIGHT, …) | unverified | - | drone AI; partial; As left. |
| 0x167b50 | NDrone2_DSTATE_StrafeDodgeLeft | drone.rs::handle(STRAFE_DODGE_LEFT, …) | unverified | - | drone AI; partial; Dodge 0xC, 2–3 s timer. |
| 0x167cf0 | NDrone2_DSTATE_StrafeDodgeRight | drone.rs::handle(STRAFE_DODGE_RIGHT, …) | unverified | - | drone AI; partial; As left. |
| 0x167e90 | NDrone2_DSTATE_RollLeftCrouch | drone.rs::handle(ROLL_LEFT_CROUCH, …) | unverified | - | drone AI; partial; Roll; ends with the anim. |
| 0x167f80 | NDrone2_DSTATE_RollRightCrouch | drone.rs::handle(ROLL_RIGHT_CROUCH, …) | unverified | - | drone AI; partial; As left. |
| 0x169150 | NDrone2_DSTATE_RunForCover | drone.rs::handle_cover(RUN_FOR_COVER, …) | unverified | - | drone AI; partial; MoveToGoalPosition to the approach point; sliding 10 %/tick to the in-cover point; arrived → 0x93; blocked → face/aim/0x21; exit (not to 0x93) → disown. |
| 0x169cc0 | NDrone2_DSTATE_UnderCoverInit | drone.rs::handle_cover(UNDER_COVER_INIT, …) | unverified | - | drone AI; partial; CoverAnim(6) → 0x94; slide 8 %/tick; face the cover angle. |
| 0x169e58 | NDrone2_DSTATE_UnderCoverIdle | drone.rs::handle_cover(UNDER_COVER_IDLE, …) | unverified | - | drone AI; partial; CoverAnim(1); refill clip; timer C FRand(150); ambushers wait until visible / within 3. |
| 0x16a2d0 | NDrone2_DSTATE_UnderCoverAim | drone.rs::handle_cover(UNDER_COVER_AIM, …) | unverified | - | drone AI; partial; CoverAnim(2) (lean/step-out) → 0x96. |
| 0x16a3e0 | NDrone2_DSTATE_UnderCoverFire | drone.rs::handle_cover(UNDER_COVER_FIRE, …) | unverified | - | drone AI; partial; Fire; the ENTER sniper branch to 0x97 is ported; node-not-OK transitions to 0x99/0x9C. |
| 0x16a610 | NDrone2_DSTATE_UnderCoverSniperFire | drone.rs::handle_cover(UNDER_COVER_SNIPER_FIRE, …) | unverified | - | drone AI; partial; Timer C (1 s) fires, timer D (6 s) → 0x99; clip empty → 0x98; OpponentIsAimingAtMe(7.5) + BondIsFacingMe(45) + Rand(3) == 0 → 0x99. |
| 0x16a8b0 | NDrone2_DSTATE_UnderCoverSniperReload | drone.rs::handle_cover(UNDER_COVER_SNIPER_RELOAD, …) | unverified | - | drone AI; partial; Reload anim 0x52; refill +0xBBC = +0xBBE; back to 0x97. |
| 0x16a9c0 | NDrone2_DSTATE_UnderCoverReturn | drone.rs::handle_cover(UNDER_COVER_RETURN, …) | unverified | - | drone AI; partial; CoverAnim(4) → 0x9A; fail → 0x9B; node-not-OK → leave. |
| 0x16ab38 | NDrone2_DSTATE_UnderCoverLeaveNow | drone.rs::handle_cover(UNDER_COVER_LEAVE_NOW, …) | unverified | - | drone AI; partial; Disown; anim 0xA → 0x56. |
| 0x16af20 | NDrone2_DSTATE_HeardNoiseAware | drone.rs::handle(HEARD_NOISE_AWARE, …) | unverified | - | drone AI; partial; Share > 0.1 → Suspect; the look-around 0x22 variant 1 is ported. |
| 0x16b0e0 | NDrone2_DSTATE_HeardNoiseSuspect | drone.rs::handle(HEARD_NOISE_SUSPECT, …) | unverified | - | drone AI; partial; Route to the sound; turn (0x61) if > 135°; the prop 0x21 AlertToPosition/SearchArea branch is ported. |
| 0x16b540 | NDrone2_DSTATE_HeardNoiseAlert | drone.rs::handle(HEARD_NOISE_ALERT, …) | unverified | - | drone AI; partial; Face the opponent → Attack. |
| 0x171df8 | Drone_SM_RouteMsgDCV | drone.rs::Drone::send | unverified | - | drone AI; partial; Exit-then-enter repetition is ported; the "msg 1 → msg 0 if state changes during enter" loop is preserved. |
| 0x172960 | NDrone2_DSTATE_AmbushInit | drone.rs::find_ambush_cover + handle_cover(AMBUSH_INIT, …) | unverified | - | drone AI; partial; Within 2.0 → 0x93; else crouch 0x11 → 0x38. |
| 0x172a30 | NDrone2_DSTATE_KnockedOut_Anim | drone.rs::handle_knockout | unverified | - | drone AI; ported; ENTER: corpse flag, health 0, CallAnim(0x50 → 0x47), KO flag +0x1000000, DropWeapon, PlrStat_LogEnemyDisabled. EXIT: the death channel except on {0x07000009,... |
| 0x172b48 | NDrone2_DSTATE_Knocked_Out | drone.rs::handle_knockout | unverified | - | drone AI; ported; Update/impacts → 0x44. Never set by C code (state table only). |
| 0x172bb0 | NDrone2_DSTATE_StunGrenadeImpact | — | unverified | - | drone AI; stub; Stun-grenade impact entry — not ported. |
| 0x172cd8 | NDrone2_DSTATE_Fade | drone.rs::handle_death(FADE, …) | unverified | - | drone AI; partial; Alpha from +0x106 down 0.6/tick; 0 or not drawn → disabled. |
| 0x172dd8 | NDrone2_DSTATE_FadeFast | drone.rs::handle_death(FADE_FAST, …) | unverified | - | drone AI; partial; As Fade with 8.0/tick. |
| 0x172ee0 | NDrone2_DSTATE_PunchImpact | drone.rs::handle_knockout | unverified | - | drone AI; ported; ENTER replays the punch with allow = 1 (the stagger); update faces the opponent; msg 6/8 → HandleImpact(0, 0). |
| 0x173098 | NDrone2_DSTATE_BulletImpact | drone.rs::handle(BULLET_IMPACT, …) | unverified | - | drone AI; partial; ENTER turns vision on; the flinch call ends it. |
| 0x173480 | NDrone2_DSTATE_GrenadeThrow | drone.rs::handle_grenade_throw(GRENADE_THROW, …) | unverified | - | drone AI; partial; ENTER +0x4F8 |
| 0x173658 | NDrone2_DSTATE_RecoverFromScaryObject | drone.rs::consider_explosive | unverified | - | drone AI; standin; Sets state 0x86 and stores +0x330/+0x350; the flee route update (run away from the explosive) is not ported. |
| 0x1739b8 | NDrone2_DSTATE_UnderCoverTypeChange | drone.rs::handle_cover(UNDER_COVER_TYPE_CHANGE, …) | unverified | - | drone AI; partial; Leave flag → 0x9C; node-not-OK and other cover → 0x8C; CoverAnim(7) → 0x94. |
| 0x173ac0 | NDrone2_DSTATE_UnderCoverLeave | drone.rs::handle_cover(UNDER_COVER_LEAVE, …) | unverified | - | drone AI; partial; → 0x9C. |
| 0x175778 | NDrone2_ProcessStateMachine | drone.rs::Drone::handle + set_state | unverified | - | drone AI; partial; The handle() match is the dispatch; set_state records the next state, applied by send (RouteMsgDCV). Messages 0xC/0xD timers are dispatched in Drones::tick. |
| 0x1757d0 | Drone_SM_Init | drone.rs::Drone::new | unverified | - | drone AI; stub; The state-machine info table is not built; per-drone state is a single state field. |
| 0x175838 | Drone_SM_InitObject | drone.rs::Drone::new | unverified | - | drone AI; stub; Numbers state machines in creation order — not modelled (one machine per drone). |
| 0x1758c0 | Drone_SM_SetState | drone.rs::Drone::set_state | unverified | - | drone AI; ported; Records +0xC next; one-tick delayed application by RouteMsgDCV. |
| 0x1759a0 | Drone_SM_SendDelayedMsgs | — | unverified | - | drone AI; not started; The delayed-message queue (+0xCC4 / +0xCD0 timers) is approximated by the timer_c/timer_d fields dispatched in Drones::tick. |
| 0x175a38 | Drone_SM_SendMsgSelf | drone.rs::Drone::send | unverified | - | drone AI; stub; Self-messages are inlined at the call site; no separate queue. |
| 0x175aa8 | Drone_SM_BroadcastMsg | Drones::tick (alarm, sound alerts) | unverified | - | drone AI; partial; The alarm broadcast and the sound broadcast run in Drones::tick; the generalised broadcast loop is not modelled. |
| 0x175af0 | DroneVision_OpponentVisibility | drone.rs::Drone::visibility | unverified | - | drone AI; partial; R = e8·(2·alert+1); vis = 0.7 − (dist−R)/R, × sin(margin/e4·π/2), × AI-volume factor +0x22C unless alert bits +0x4FC and alert ≥ 1. |
| 0x175cb8 | DroneVision_SeekOpponentLOS | drone.rs::Drones::tick (round-robin) | unverified | - | drone AI; partial; One drone per tick, round robin, only if vis ≥ 0.7; head-to-target bone cycle (2,3,5,…) not ported — port uses the head position. |
| 0x175ef0 | DroneVision_HaveOpponentSight | drone.rs::Drones::tick | unverified | - | drone AI; partial; +0x228 & 4 from vis ≥ 0.7; counters +0x270/+0x274. |
| 0x176250 | DroneVision_ConsiderAlerted | drone.rs::Drones::tick (alert pairs) | unverified | - | drone AI; partial; Source attacking ≥ 1 s with visible alert bits; observer watching (Idle/Alert/Patrol); props accept bits; within 50 units; in view; head-to-head LOS. At most... |
| 0x1767d0 | DroneVision_FindAlertedDrones | drone.rs::Drones::tick (alert pairs) | unverified | - | drone AI; partial; As above. |
| 0x176a28 | DroneVision_ProcessDroneSight | drone.rs::Drones::tick | unverified | - | drone AI; partial; Visibility every tick for vision-on drones; LOS round-robin. |
| 0x1770f0 | NDrone2_ReactToOpponentSighted | - | unverified | - | level load; partial; the team == 3 gate is ported in look_for_opponent; the rest of the react path is in the state machine. |
| 0x177800 | DroneVision_AlertSound | - | unverified | - | audio; not-started; the alert sound routing. G-032. |
| 0x177a30 | DroneVision_ShoutFromOtherDrone | — | unverified | - | drone AI; stub; Returns 0 in this build (dead code). The port skips the shout broadcast entirely. |
| 0x177c18 | DroneVision_EnemyAlerts | drone.rs::enemy_alerts | unverified | - | drone AI; partial; Msg 0x14 → HeardNoise chain; msg 0x1E → Attack (alarm); the dead ShoutFromOtherDrone cases return 0. |
| 0x177ed0 | DroneVision_EnemyLookForOpponent | - | unverified | - | level load; ported; drone.rs::look_for_opponent (the team == 3 gate, 0078). |
| 0x178520 | DroneVision_GunThroughWall | — | unverified | - | drone AI; not started; The "shoot through thin walls" check — not ported. |
| 0x178678 | NDrone2_NearArmedBond | drone.rs::Drones::tick | unverified | - | drone AI; partial; Bond-armed scan (2..0x47 / 0x6A..0x71) — used for NearArmedDrone. |
| 0x178768 | NDrone2_ObjectIsArmed | drone.rs::Drones::tick | unverified | - | drone AI; partial; Weapon byte check (used in the alert pairs). |
| 0x1789c0 | DroneWeap_CloseCombatImpact | — | unverified | - | drone AI; not started; Close-combat hit (punch) — not ported. (The PLAYER punch is ported: it is a bullet, not this function — NDrone2_DealWithObjHit case 5 routes weapon 1 to msg 6.) |
| 0x178c48 | DroneWeap_Ready2Fire | drone.rs::ready_to_fire | unverified | - | drone AI; partial; Aiming class, close enough, or the gun within 20° of the target. |
| 0x178f68 | DroneWeap_FireWeapon | drone.rs::fire_weapon | unverified | - | drone AI; partial; Sound_Play3D(weapon_data+0x60, drone) (skipped for 0x5EF); DoBulletAccuracy; Bullet_init. |
| 0x179558 | DroneWeap_ThrowGrenade | drone.rs::throw_grenade | unverified | - | drone AI; partial; Decrements +0xBCE; ±π/4 yaw lob by distance; mode 0xD sniper 1/3 chance of stun (0x36) over frag (0x35); bone 0x15 origin. |
| 0x179a48 | DroneWeap_DoOpponentTargetting | drone.rs::Drones::tick | unverified | - | drone AI; partial; Moving / "first moved" for 2 s after Bond starts. |
| 0x179ba8 | DroneWeap_BurstDelay | drone.rs::burst_delay | unverified | - | drone AI; partial; max(15, trunc(45·k·(0.5 + FRand 0.5))), ≤ 60, = 15 under 4 units, = 60 beyond 30; k from aggression +0xB5 (1.66/1.33/1.0/0.66/0.33). |
| 0x179d40 | DroneWeap_NextBulletTime | drone.rs::next_bullet_time | unverified | - | drone AI; partial; Burst of weapon_data+0x2C (SG5 3, L96A1 1), +0x40 apart (SG5 4, L96A1 120); between bursts D + interval. |
| 0x17a080 | DroneWeap_DoFiring | drone.rs::handle_firing | unverified | - | drone AI; partial; FireWeapon while the burst continues; clip-empty → reload. |
| 0x17a180 | DroneWeap_HandleFiring | drone.rs::handle_firing | unverified | - | drone AI; partial; The per-tick driver; clip / burst / reload decisions. |
| 0x17a2a0 | DroneWeap_DropWeapon | drone.rs::drop_weapon | unverified | - | drone AI; partial; Once — the weapon byte (anim+0x62) is cleared. At its matrix. |
| 0x17a500 | DroneWeap_OpponentIsAimingAtMe | drone.rs::handle_cover | unverified | - | drone AI; partial; Used by the sniper 0x97 duck (7.5 range). |
| 0x17a5a0 | DroneWeap_GetWeaponInfo | drone.rs::muzzle/fire_weapon | unverified | - | drone AI; partial; Muzzle bone 0x15; weapon damage / speed / range from weapon_data. |
| 0x17a6e0 | DroneWeap_ChangeWeapon | drone.rs::handle(SURRENDER_ANIM, ENTER) | unverified | - | drone AI; stub; The surrender draws the sidearm (weapon 1, anim set 0xC). Other weapon swaps not ported. |
| 0x17a798 | DroneWeap_CanThrowGrenade | drone.rs::can_throw_grenade | unverified | - | drone AI; partial; +0xBCC/+0xBCE ≠ 0; Rand(2) == 0; 8 ≤ dist ≤ 20; CanDoAnimState(0x49, 0). |
| 0x17a870 | DroneWeap_ManageBurstCount | drone.rs::next_bullet_time | unverified | - | drone AI; partial; Burst count from weapon_data+0x2C; burst counter +0xBBC. |
| 0x17a8c0 | DroneWeap_Fire | drone.rs::fire | unverified | - | drone AI; partial; Bullet_init with the muzzle direction; the per-shot Rand calls run. |
| 0x17b968 | Env_Create | - | unverified | - | level load; ported; env::Drops (snow). |
| 0x17bc88 | Env_Update | drone.rs:3706-3712 (Drops::update, env.rs:58) | unverified | - | frame/triggers; ported; The port runs it inside Drones::tick before the paused gate — the game runs it before GS_IsPaused too, so snow keeps falling while paused [M order match]. Th... |
| 0x17cbd0 | Explode_Create | drone.rs::Drones::tick (Explosion struct) | unverified | - | drone AI; stub; The Explode object's create is approximated by an Explosion struct in the Drones list. |
| 0x17d178 | GS_PauseGame | none | unverified | - | frame/triggers; not started; The LOAD_LEVEL caller's pause is folded into the port's synchronous level swap. |
| 0x17d1e8 | GS_IsPaused | none (App.paused + Drones::paused stand in) | unverified | - | frame/triggers; standin; The port has no pause byte: nothing distinguishes "pause menu" from "cut-scene" — divergence 3 in §7. |
| 0x17d340 | Copter_Create | - | unverified | - | level load; not-started; the helicopter. |
| 0x18bee0 | MPSound_Play | - | unverified | - | audio; not-started; MP sound (not ported). |
| 0x18c580 | UpdateDrops | env.rs:58 | unverified | - | frame/triggers; ported; Velocity table seeds are not the game's (env.rs:41 notes the seed deviation). |
| 0x18d840 | RainBox_Create | - | unverified | - | level load; ported; env::RainBox. |
| 0x18da00 | Corona_Create | - | unverified | - | level load; ported; corona::placements (the 13 static glow quads). |
| 0x1902c8 | Player_HandlePain | bullet.rs::PlayerHealth::pain | unverified | - | drone AI; partial; Armour absorbs first; health floors at 0; flash += min(d·21.333, 128). |
| 0x1a16c0 | Player_WeaponHasAmmo | - | unverified | - | weapons; not started; Whether the player has ammo for a weapon (clip + carried). |
| 0x1a24c8 | Player_Weapon | - | unverified | - | weapons; partial; The per-tick weapon manager. Reads the d-pad/gadget actions and calls Player_WeaponChange. Ported: the trigger, reload, aim-mode toggle, zoom. Not ported: th... |
| 0x1a2cc0 | Player_WeaponSelect | - | unverified | - | weapons; partial; Starts the switch. Ported: checks +0xC4 (holster) to decide state 1 vs 3. Not ported: the variant-swap path (same base weapon → plays +0xC8 at 1.3x, state 8)... |
| 0x1a2e38 | Player_SetWeaponAnimObj | - | unverified | - | weapons; partial; The view-script state machine. Ported: state 0 (idle → holster), state 1 (holstering → plays +0xC4), state 3 (draw → plays +0xC0), state 4 (draw done → idle)... |
| 0x1a5310 | Player_Activate (door part) | drone.rs::Drones::player_use_doors | unverified | - | drone AI; partial; The use key at Bond's head, one unit ahead; Door_Activate for every door the sphere meets. |
| 0x1a5460 | Player_WeaponFiring | - | unverified | - | weapons; ported; The trigger. Gun::update_with. |
| 0x1a6eb0 | Player_IsBetterWeapon | - | unverified | - | weapons; not started; The BestWeapon table for auto-switch on pickup. |
| 0x1a6f80 | Player_WeaponChange | - | unverified | - | weapons; partial; The next/prev cycle. Ported: Gun::next_weapon iterates owned, selectable, has-ammo weapons. Not ported: the weapon/gadget split (mode=0 → weapons 1..0x47, mo... |
| 0x1a73f8 | Check_AutoAim | drone.rs::Drones::auto_aim | unverified | - | drone AI; partial; Drawn, alive enemy drones within 25; cone 0.12·k yaw, 0.22·k pitch, k = weapon+0x14/100 · (1−d/25) · difficulty; ×1.4 lock-on; target bone 2; LOS. The crossh... |
| 0x1a8a58 | Player_EquipWeapon | - | unverified | - | weapons; ported; Inventory::equip_weapon. |
| 0x1a8d18 | Player_EquipAmmo | - | unverified | - | weapons; ported; Inventory::equip_ammo. |
| 0x1a9680 | Player_GetBestWeapon | - | unverified | - | weapons; not started; Finds the best owned weapon (for auto-switch). |
| 0x1a9858 | Player_WeaponNone | - | unverified | - | weapons; not started; Clears the weapon (when out of ammo). |
| 0x1a9c80 | Input_Update | lib.rs:520-600 (winit key events → Input) | unverified | - | frame/triggers; partial; Keyboard/mouse only; no pad, no Input_Action repeat/edge semantics (action 0x19 "skip cut-scene" missing — see SP_Update). |
| 0x1ad998 | SSys_Monitor | none (ambient handled in sound.rs init) | unverified | - | frame/triggers; not started; [M 0x1ad998:10] gated on ScriptCam == 0. |
| 0x1adea8 | Switch_Update / Switch_Activate (class 0x39, placement 0x29) | none | unverified | - | frame/triggers; not started; Switch objects on 07000005: none in the main map [D]; other levels use them (the FuseBox 0x31 family is separate). |
| 0x1b1510 | Text_Update / Text_Update2Line | none (HUD shows health/ammo strings) | unverified | - | frame/triggers; not started; Cut-scene subtitles missing. |
| 0x1b1720 | Trigger_Create | triggers.rs:141 (from_params), triggers.rs:269 (build) | unverified | - | frame/triggers; partial; Per the table: the port's mux family reads mask q(9), list q(1..8)\\{7}, lock q(10), hide q(11) where the game reads mask q(8), list q(1..7), lock q(9), hide... |
| 0x1b1900 | Trigger_Calc | triggers.rs:291 (calc), called drone.rs:3895 | unverified | - | frame/triggers; partial; (a) missing (stats only); (b) ported for the two latches (triggers.rs:292-293); the clear pass clears locked triggers' outputs too — matches the game (no loc... |
| 0x1b1a78 | Trigger_Update | = 0x30 when hide (+0x3A) ≠ 0 (0x1b1a78:50-57), return either way; path anim pose + clock advance (wrap past frames); Trigger_Hit; then per type — 1 TouchOnce: out on → delete (if obj+0xF0 & 0x10), else falls through into 2; 2 Touch: ch[out] \ | unverified | - | frame/triggers; = hit (+timestamp), channel 0x7d only on 0x07001500; 3 and: ch[out] = and(inputs) stopping at the first 0 (+timestamp, bond-moment 0x65-0x6e + obj+0xf0 & 0x10 → delete); 4 seq and: and with the window predicate time[prev_input] − 4 <= time[input] — no upper bound (an input any time after the previous passes, and so does one up to 4 frames *before* it) [m 0x1b1a78:135-136]; 5 out: copy ch[out] to up to 8 targets; 10 load_level: ch[+0x28] on → trigger_activate(this, glb_players); 0xc or: ch[out] = or(inputs); 0xd movieplayer: input on → clear that channel, gamestate._16_4_ = out, gamestate[0x14] = 0, gameflow_pushstate(0,6), gameflow_pushstate(30.0,7) (0x41f00000 [m 0x1b1a78:190-199]).; triggers.rs:233 (update), per-type arms; Touch run in calc (triggers.rs:291) |
| 0x1b1fc8 | Trigger_Hit | triggers.rs:185 (hit) | unverified | - | frame/triggers; partial; Contact bit 2 (class-5 objects touching) has no port touchers (touchers are Bond + enabled drones only, drone.rs:3892-3894); mask bit 1's channel gate is mis... |
| 0x1b2318 | Trigger_Activate | - | unverified | - | level load; ported; Trigger::update (the LOAD_LEVEL arm, the Music_Event(3,1), the GS_PauseGame). |
| 0x1b2628 | SoundTrigger_Create (0xF9) | none | unverified | - | frame/triggers; not started;  |
| 0x1b2790 | SoundTrigger_Update / MusicTrigger_Update / InverterTrigger_Update | none | unverified | - | frame/triggers; not started; 07000005 placements 0xF9/0xFB/0x41: none in the main map [D per the survey in levels.md §2], so nothing playtest-visible yet. |
| 0x1b2950 | Trigger_LoadLevelCreate | triggers.rs:149 (LOAD_LEVEL arm) | unverified | - | frame/triggers; ported; mask hard 0xE not copied (unused by case 10). |
| 0x1b2a08 | Trigger_MoviePlayer_Create (0xF4) | none | unverified | - | frame/triggers; not started; Found in the 0x1d04f0 dispatch; not in docs/systems/triggers.md's type table. |
| 0x1b2aa8 | Trigger_Touch / Trigger_TouchOnce | triggers.rs:143-144 | unverified | - | frame/triggers; ported; — |
| 0x1b2c68 | Trigger_MultiplexIn / MultiplexSIn / MultiplexOrIn / MultiplexOut (0xEC/0xED/0xEF/0xEE) | triggers.rs:150 (shared _ arm) | unverified | - | frame/triggers; partial; The list/mask/lock/hide slot shift (divergence 6). |
| 0x1b3040 | InverterTrigger_Create (0x41) | none | unverified | - | frame/triggers; not started;  |
| 0x1b30e0 | MusicTrigger_Create (0xFB) | none | unverified | - | frame/triggers; not started;  |
| 0x1c1b38 | AnimProcessScriptCmds | drone.rs::script_commands | unverified | - | drone AI; partial; The walk-script commands: events 0/4/5 footsteps; event 3 Sound_StopAllWithId(arg). Other commands (sounds, weapon changes, footprints) are partial. |
| 0x1c4ac8 | AnimGetCollData | damage::boxes | unverified | - | drone AI; ported; The HumanCollisionBones (0x2c6be8) → boxes; entry 0's centre is the bone plus (0, bone.y − other.y, 0); the others sit midway. Extents × skin scale. |
| 0x1c6ca8 | AnimScriptPeekSfx | - | unverified | - | audio; not-started; the op-1 sfx id peek for speech precache. G-032. |
| 0x1c9790 | Game_Run | nf-view/src/lib.rs:649-687 (RedrawRequested tick loop) + scene.rs:237 (step_bond) / scene.rs:409 (run) | unverified | - | frame/triggers; partial; No FreezeGame/switch_AllowFreeze/sloflag (no slow-mo, no freeze); no MenuManager_*/Mission_Update/MP_Update (mission end/fail screens, the 0x0700004x wrapper... |
| 0x1c98c8 | GameFlow_Main | - | unverified | - | level load; standin; the GameState stack machine is not ported; nf-view drives the single-level loop. STANDIN in main.rs. |
| 0x1c9e80 | GameFlow_GetState / PushState / PopState / Main | lib.rs:623-687 (the RedrawRequested block plays the role of state 4 + the swap) | unverified | - | frame/triggers; standin; No stack: pending_level + a synchronous rebuild. Anything the game does between PushState(3) and PopState (load screen fade, the movie, PS2_SFXWaitTillFinish... |
| 0x1c9eb0 | GameFlow_PopState | - | unverified | - | level load; not-started; same. |
| 0x1c9f38 | GameFlow_PushState | - | unverified | - | level load; not-started; the state stack (load screen, etc.). |
| 0x1d04f0 | parsemap_create_dynamic_objects | - | unverified | - | level load; partial; the object-creation dispatch (type → Door_Create, Drone_Create, etc.); the port builds the instances in Scene::build but the per-type creation order and the ... |
| 0x1d1b68 | parsemap_block_map_data_dynamic | - | unverified | - | level load; partial; Level::parse handles placements (0x1A), but not the door record (key count/rate at +0x40/+0x42, the anim at +0x44) — that's in door.rs's own parse. The runti... |
| 0x1d24d0 | parsemap_block_path_data | - | unverified | - | level load; ported; Level::path_blocks (the 0x19 key-list payloads). |
| 0x1d3800 | ResetMap_LevelToLoad | - | unverified | - | level load; ported; Trigger's LOAD_LEVEL arm sets Drones::level_exit to the next level's hashcode; nf-view/main.rs reloads. |
| 0x1d3800 | ResetMap_LevelToLoad(level, warm, c) | lib.rs:623-645 (pending_level → load) + drone.rs:4007-4017 / nis.rs Request::LoadLevel | unverified | - | frame/triggers; partial; No GameFlow stack (the port swaps the scene at the end of the tick); the movie-code 0x71 prefix is dropped (the port loads {hash:08x}.bin directly — the on-d... |
| 0x1d3a80 | ReadTuningVars | nf_game::tuning::read_tuning_vars + nf_formats::tuning | unverified | - | drone AI; ported; The per-level tuning switch; checked against the R5900 interpreter for every level id 0x07000000–0x0700004F. |
| 0x1d5080 | ResetMap_GameInit | - | unverified | - | level load; partial; the level-start init (bank load, music start, drone creation); ported in Scene::build + build_sound, but the GameState stack machine is standin. |
| 0x1d5258 | ResetMap_LevelCode2Img | - | unverified | - | level load; ported; the level-code → archive-name switch (read from the decompile; the 36 on-foot levels). |
| 0x1d5de0 | ResetMap_GenLoadScreen | - | unverified | - | level load; not-started; the load screen (the port loads synchronously). |
| 0x1d6360 | ResetMap_Load | - | unverified | - | level load; ported; nf_view::load (the archive read + Level::parse). |
| 0x1d8088 | GetSplineWeights / Script_GetInterp | nis.rs:431 (keys) | unverified | - | frame/triggers; ported; The spline rotation slerp weight is *suggested* (U-040). |
| 0x1d8548 | Script_Run | nis.rs:161 (run) | unverified | - | frame/triggers; partial; Ops 0x13-0x1E other than 0x16/0x17 stop the stream in the port (nis.rs:262-266) instead of consuming their arguments — a script with a fade/sprite/light/subs... |
| 0x1d8d78 | Script_EntityStart / Script_CreateEntity | nis.rs:270 (entity_start) | unverified | - | frame/triggers; partial; The typed entities are stored as Obj::Entity{kind}; only 0xEA/0xEB gain trigger volumes and the generic ones gain collision (drone.rs:3900-3914) — Break/Ladd... |
| 0x1d8f40 | Script_AnimStart | nis.rs:285 (anim_start) + renderer's discrete0 | unverified | - | frame/triggers; ported; The root-translation path runs through World::discrete0 (the port reads the pose from the anim system); NDrone2_CreateNinjaEyes (skin 0x0500001E) not ported. |
| 0x1d9140 | Script_CameraStart / Script_CameraEnd | nis.rs:231-252 (op 0x0D/0x0F) | unverified | - | frame/triggers; partial; Input_ClearAllActions unported (queued inputs survive into the cut-scene); ScriptCam unported (§7 item 3). |
| 0x1d94c0 | Script_EventHandler | nis.rs:320 (event) | unverified | - | frame/triggers; partial; Events 5 (Drone_EnableAll), 8, 9, 10, 0xF fall into the catch-all Request::Event (nis.rs:377) — event 5's absence is divergence 3's other half: the port cann... |
| 0x1d9860 | Script_KillStream | nis.rs:297 (kill) | unverified | - | frame/triggers; ported; The camera pop posts Request::Camera{start:false} → Music_Event(0xC) [M 0x1d9860 vs nis.rs:303-305]. |
| 0x1d9a08 | Script_Interp | nis.rs:381 (interp) + drone.rs:3940-3974 (sound reposition) | unverified | - | frame/triggers; partial; Case 2's anim world is the renderer's (AnimObjectUpdate, footsteps, NDrone2_CreateNinjaEyes missing — footsteps during cut-scenes don't sound, U-0xx); case 5... |
| 0x1da880 | Script_Free | nis.rs:307 (free) | unverified | - | frame/triggers; partial; The drone re-enable and the ScriptCam clear have no port counterpart (the port's PlayerEnable request drives scene.rs:414-421 only). |
| 0x1da998 | Script_Update | nis.rs:138 (update) | unverified | - | frame/triggers; partial; The fade tail (camera fade-in/out at cut-scene ends; the two exemption sets above) is not ported — cut-scene fade is the renderer's lack, noted in script-pla... |
| 0x1db420 | Script_StreamEnd | nis.rs:173-197 | unverified | - | frame/triggers; ported; The port holds the stream on the end command while clock ≤ time && clock < length (game rewinds onto it — same observable state [S]). |
| 0x1db588 | Script_Play | = 1 (+8 when looping); if the script is a camera script (+0xAA8 & 0x40): ScriptCam = hash + Music_Event(6,1) for hashes {0x600098A, 0x6000952, 0x6000991}; Text_FlushAllSubtitles. | unverified | - | frame/triggers; nis.rs:128 (start); partial |
| 0x1db9a8 | SP_CreateScriptPlayer | nis.rs:490 (from_placement) | unverified | - | frame/triggers; partial; The port's formula mode==1 → 7; mask==0 → 6; else 0 misses the game's third branch: mask == 0 and mode ∉ {0,1} leaves the port in state 6 where the game load... |
| 0x1dbc08 | SP_LoadScript | nis.rs:512 (load) | unverified | - | frame/triggers; ported; The isLoadable streaming arm is ported (fix-cutscenes): the level's type-0xC records → pack 078/079/07A000NN.bin (the (flag+7) nibble) loaded in nf_view::loa... |
| 0x1dbda8 | SP_Update | 4 when +0x2C) & mask; states — 0: trigger → (streaming push \ | unverified | - | frame/triggers; ) load+play → 2 (gated on mission_status, health > 0 and the 0x6000994 music-event exception); 1: trigger → play, countdown 5, channel out = 1; 2: sp_unpause while mask 1 + channel on, action 0x19 fast-forwards (script_ffwd, death nis exempt), on done → swap/state 3 or stay → 5; 3/4 mirror 1/2 with out = 0; 5: delete; 6: trigger → delay 0, free+load other → 2; 7: load. tail: channel out = playing — gated on the trigger only (no mask-bit-1 condition [m 0x1dbda8:221-229]); hit-health unpause (sp_gethitdamage accumulates into +0x18, past +0x14 → sp_unpause − threshold).; nis.rs:528 (update) |
| 0x1dcad8 | SP_SwitchToScript | implicit (state 6 swap) | unverified | - | frame/triggers; partial; No direct port; the port swaps only via the state machine. |
| 0x1dcc30 | SP_Hit | none (only bit 4 via activated) | unverified | - | frame/triggers; not started; See above. |
| 0x1dcc68 | Sound_Play | - | unverified | - | audio; ported; Sounds::play (2-D sound). |
| 0x1dcd10 | Sound_Play3D | - | unverified | - | audio; ported; Sounds::play3d (3-D sound, custom radii for map ambients). |
| 0x1dce10 | Sound_UpdateListeners | - | unverified | - | audio; partial; MapSounds::update + the Sounds::update listener push. Not ported: the SFXSetEnvironment(room env, underwater) call (the cel/room value isn't available — G-01... |
| 0x1dce68 | Sound_UpdateSounds | - | unverified | - | audio; ported; Sounds::update (deferred start, grace period, stop, position update). |
| 0x1dceb8 | Sound_Ready | - | unverified | - | audio; partial; sound::level_bank (the bank-slot table); the SFXLoadSoundBank call and the Music_Event(0,level)/(1,1) hooks are in nf-view/main.rs. Not ported: the precache ... |
| 0x1dcf20 | Sound_Stop | - | unverified | - | audio; ported; Sounds::stop. |
| 0x1dcf48 | Sound_StopAllWithId | - | unverified | - | audio; ported; Sounds::stop_all_with_id. |
| 0x1dcf88 | Sound_SetPosition | - | unverified | - | audio; ported; Sounds::set_position. |
| 0x1dcfb8 | Sound_IsFinnished | - | unverified | - | audio; ported; Sounds::finished. |
| 0x1dd118 | HandleMapSoundAllocation | - | unverified | - | audio; ported; MapSounds::update (the radius in/out hysteresis). |
| 0x1dd548 | Sound_Play3D | drone.rs::Drones::play_sound_handle | unverified | - | drone AI; partial; The 3D sound play at a position; the hearing list entry. |
| 0x1ddb90 | SFXIsSFXPlaying | - | unverified | - | audio; ported; Driver::is_playing (the START_GRACE 5-VBlank window). |
| 0x1ddc48 | Sound_Alertness | drone.rs::Drones::tick (sound loop) | unverified | - | drone AI; partial; Per sound within its outer radius, weight (inside the inner radius) or weight·(outer−d)/(outer−inner); the sum and the loudest. PS2 float arithmetic (add_nog... |
| 0x1de328 | Music_Event | - | unverified | - | audio; ported; Music::event (the 64-entry event board). |
| 0x1de3a0 | (the music command queue) | - | unverified | - | audio; ported; Music::out → Cmd::Start/Jump/FadeDown. |
| 0x1df7e0 | vecutil_calculate_closest_point_on_line | - | unverified | - | audio; ported; closest_point_on_segment. |
| 0x1e0f10 | Ang_RotateToBearing | trig::rotate_to_bearing | unverified | - | drone AI; ported; The per-tick yaw rotation toward a bearing. |
| 0x1e2b78 | Vec_AngleDifference | trig::angle_difference | unverified | - | drone AI; ported; The wrap-to-π angle difference. |
| 0x1e2d08 | Camera_UpdateAll | scene.rs:420 (script_camera) + lib.rs:711-714 | unverified | - | frame/triggers; partial; Only camera 4 (script) and the Bond-follow camera; no Camera_CalcViewAngles game-path, no camera stack pops on Script_CameraEnd beyond the object delete. |
| 0x1e4528 | ATAN2_APPROX | trig::atan2_approx | unverified | - | drone AI; ported; The PS2's atan2 approximation. |
| 0x1e5168 | View_RotTransMatrix | aivolume::matrix | unverified | - | drone AI; ported; Rx·Rz·Ry row vectors; PS2 float arithmetic. |
| 0x1e62d8 | View_AddObjects | - | unverified | - | level load; partial; the object draw dispatch; the port's Scene::build + add_instance mirrors it but the cel linking is standin. |
| 0x1e6ba0 | View_AddCels | - | unverified | - | level load; partial; the draw-list assignment (glist flags → list 0/1/2); the cel visibility system (G-016) is standin. |
| 0x1EA398 | Collide_Update | world.rs capsule world + drone.rs:3900-3914 (set_dynamic) | unverified | - | frame/triggers; standin; The port's moving-set collision replaces the proxy/pair system; Door_ReactToHit anti-crush does not run from it (A-11b removed the stand-in call; U-041). |
| 0x1eae80 | Collide_PickObj | - | unverified | - | level load; standin; the cel-linked collision pick; the port uses a flat triangle list (CollisionWorld). STANDIN in world.rs. |
| 0x1ec180 | Collide_Jointy | damage::first_box_hit | unverified | - | drone AI; partial; The first box in list order the segment hits (not the nearest). The full Collide_Jointy (sphere/box/capsule) is not ported — only the ray-box path. |
| 0x3037dc | (per-material bounce table) | grenade.rs::BOUNCE_RESTITUTION (= 0.4) | unverified | - | drone AI; standin; The game reads 0x3037dc + material·0x88 for the bounce coefficient; the port uses a constant 0.4 (the feet-sphere radius and a reasonable floor bounce). |
