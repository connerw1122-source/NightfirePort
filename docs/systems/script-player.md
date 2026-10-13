# Script players and level scripts

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] decomp / disassembly read here, [L] snapshot `work/ee_start_05.bin`, [D] disc
bytes, [S] suggested. Format: docs/formats/nis-script.md. Port: `crates/nf-game/src/nis.rs`.

## ScriptPlayer object (placement type 0xD9) [M]

`SP_CreateScriptPlayer` (0x1db9a8), created only when parameter 0 or 1 is set. Parameter k is at `level_tag+0x2C+4k`.

| Param | Field | Meaning |
|---|---|---|
| p0 / p1 | `+0x1C` / `+0x20` | scripts A / B (`+0x2A` selects the current one) |
| p2 / p3 | `+0x26` / `+0x28` | mode of A / B: 1 = play at once, 8 = play looping (state 6), else load, run one update, stop (state 1) |
| p4 | `+0x35` | ≠ 0: after the script ends go to state 5 (delete), else swap to the other script (state 3) |
| p5 | `+0x2E` | trigger mask over the `SP_Hit` flags (1 channel, 2 bullet, 4 activated, 8 / 0x10 objects) |
| p6 | `+0x30` | channel: its value is trigger bit 1; written back (= playing) on a trigger |
| p7 | `+0x14` | hit health (damage to unpause) |
| p8 | `+0x37` | bullet filter |
| p9 | `+0x32` | gate channel: no update while it is 0 |
| p10 | scale % | `SCRIPTINFO+0x94` (0 → 1.0) |

Start state: 6 if the mask is 0; 7 if mode A is 1 (loads and plays on the first update). `SP_Update` (0x1dbda8):
skip while `+0x36` or the gate channel is closed; `Script_Update`; trigger = (hit flags | 4 when `+0x2C` was set by
`SP_Activate`) & mask (the decomp loses this; disassembly 0x1dbe20–0x1dbe9c). States: 0 wait → load and play (2);
1 → play (2) on a trigger, channel out = 1; 2 playing → when done: state 5 if p4, else swap scripts and state 3;
3 → play (4) on a trigger, channel out = 0; 4 → when done swap, state 1; 5 → delete the object; 6 → on a trigger
swap to the other script and play (2); 7 → `SP_LoadScript`.

07000005 [D]: six players. #37 plays 0x06000A22 (the truck) from the first update (mode 1, mask 0); #38
(0x060008C8) waits on channel 0x5C, #71 (0x06000074, a loadable cut-scene) on 0x7D, #284 on 0xBE, #298 on 0x43, #320
(0x06000A23, mode 0x40) on 0x65.

## Script runtime [M]

`SCRIPTINFO` (0xAC0 bytes): `+0x00` matrix (`View_RotTransMatrix(player rotation, position)`), `+0x80..+0x88`
callback data, `+0x8C` keys, `+0x94` scale, `+0x98` hash, `+0x9C` delay ticks, `+0xA0` clock, `+0xA4` fade,
`+0xA8 + 0x28·i` streams, `+0xAA8` flags (0x10 keep objects, 0x40 camera), `+0xAAA` stream count, `+0xAAE` loop
start, `+0xAB0` length, `+0xAB2` play flags (1 playing, 2 done, 8 looping), `+0xAB3` current stream.

Stream (0x28): `+0x00` anim start clock, `+0x04` anim start frame, `+0x08` clock, `+0x0C` wait-until, `+0x10`
object, `+0x14` command pointer, `+0x18` first command, `+0x1C` / `+0x1E` keys, `+0x20` start tick / object stream,
`+0x22` flags (2 spline), `+0x24` kind (1 entity, 2 animation, 3 sprite, 4 camera, 5 sound, 6 light, 7 sub-script),
`+0x25` parent stream (−1 none).

- `Script_Play(loop)` (0x1db588): flags |= 1, loop sets 8 (else clears it). `Script_Set2Start`: every stream clock 0,
  command pointer to the start, clears flag 2. `Script_IsPlaying`: flag 1 unless done (2) and not looping.
- `Script_Update` (0x1da998), once per tick while playing and the delay is 0: `Script_Run` on every stream in order,
  then the script clock and every stream clock += `FRAME_RATE_MUL` (1.0 [L]).
- `Script_Run` (0x1d8548): while the stream clock ≤ the length, `Script_Interp` first; then run commands while the
  clock ≥ wait-until. `04` (`Script_StreamEnd`): wait-until = time; clock ≤ time and clock < length → stay on it;
  otherwise looping → restart the stream (clock = loop start − 1, script clock = loop start); else stop playing
  (flag 1 cleared); either way flag 2.
- `Script_Interp` (0x1d9a08), by kind. 1 entity: the keys' interpolation (linear, or spline when `+0x22 & 2`) ×
  each parent stream's (an animation parent gives its discrete object 0's world matrix) × the script matrix (the
  last argument of `Script_GetInterp` is `parent == −1`, disassembly 0x1d9a7c–0x1d9a98); scale from the key.
  2 animation: frame = stream clock − start clock + start frame, clamped to [1, script length], integer frames.
- `Script_GetInterp` (0x1da158) linear: binary search on key time for ⌊clock⌋ + 0.001; weight
  (⌊clock⌋ − t0) / (t1 − t0); position lerp, `Quat_Slerp_Acc`, scale and +0x24 lerp; before the first key or after
  the last, that key. Spline (`GetSplineWeights` 0x1d8088): Catmull-Rom over keys i−1..i+2 (clamped).
- `Script_EntityStart`: params to `0x349948+0x2C` (a level_tag), `Script_CreateEntity` by object type (0x20 Break,
  0x22 Ladder, 0x29 Switch, 0xE1 Destroy, 0xEA TouchOnce, 0xEB Touch, 0xF0 Pickup, 0xF2 emitter, 0xF5 AI volume, else
  a plain object drawing the entity's glist); kind 1; parent = the byte when "has parent", else −1; spline flag.
- `Script_AnimStart`: a new object at the origin with an `AnimObject` of the skin (flag 0x8000), the animation script
  added (flag 0x10000: integer frames), start frame clamped to [1, length]; stream start clock = clock; the object
  position = the first frame's root translation (`AnimGetFirstTrans`), then each tick += the root delta
  (`AnimSeqTick` zeroes the root, `AnimFrameResolve` adds the rotated delta to `obj+0x30`): the object follows the
  sequence's root translation in world space; root rotation stays in the bones.
- `Script_KillStream`: entity / animation: delete the object unless flag 0x10 (keep: detach only); sound: stop.
- `Script_EventHandler`: 3 delay = p0 − 1; 4 disable the player; 5 `Drone_EnableAll(0)`; 0xC flag 0x10 (keep objects);
  0xD channel p0 = p1 (and its time when set); 0xE stream flag 1; 0x11 if channel p0 then channel p1 = p3 else
  channel p2 = p3; 0x12 load level p0; others camera / callbacks.
- `Script_SoundStart`: 2-D (or no object) `Sound_Play`, else `Sound_Play3D` at the object stream's object.

## The truck on 07000005 [D, M]

ScriptPlayer #37 plays 0x06000A22 from the first update. Stream 1 animates skin 0x05000005 (9 rigid parts on 9
bones) with anim script 0x06000092 frame = t + 1 for t = 0..5895; the event at 5890 sets "keep objects", so at 5895
the animation stream ends but the truck stays parked on the last frame. The headlight coronas follow discrete object
0; the touch triggers set channels 0x70 / 0x71 / 0x5C (0x5C starts ScriptPlayer #38).

## Collision [M, L]

`Collide_PickObj` (0x1eae80) tests every object linked into a cel whose collision data (`obj+0xD8`, `+8`) is set,
unless its `obj+0xF0` has a bit the test excludes (0x10 / 0x20 / 0x40 / 0x1000 by test flags). The truck's nine
discrete objects (type 0x3D) all have collision data, are linked into cels and carry flags 0x8880 (windows 0x8A81)
[L, snapshot 0x137C8A0…]: the whole truck is solid to Bond, bullets and sight lines. The port places the collision
meshes of script animations' parts and of drawn script entities in `CollisionWorld`'s moving set after the script
update each tick (WORKLOG 0060).

## Port status

`nf-game::nis`: the player states, `Script_Update` / `Run` / `Interp` (linear and spline keys), commands 04–1E —
including the G-031 remainder: **fade** (op 0x13 → `Camera_SetFade(-f, 4, 0xff, 0)` 0x1db3d0/0x1bbdd8; the port's
`Drones::view_fade` ticks 1/|f| per tick and nf-view draws the black overlay; `Script_Update`'s own length−20
fade-out (40) and the −22 handback fade-in are raised too), **letterbox** (`Script_CameraStart` 0x1d9140's two
512×64 border sprites [L ACTION.ELF 0x2c9468] — `Drones::letterbox` while a script camera is live; the music
events `Music_Event(0xB/0xC)` were already wired), **sprite** (op 0x14/0x15 → `Obj::Sprite` with the 0.8/0.9333
scaled rect + hash, killed by `Script_KillStream` case 3), **light** (op 0x18/0x1a → `Obj::Light`,
`Light_Create` 0x1cc3d8 at the origin vector 0x2d6200 [L (0,0,0)], kill keeps flag-0x10 scripts' lights),
**sub-script** (op 0x1b/0x1d → `Script_Load` + `Script_Play(sub, 1)` looping on `Player::subs`, ticked from
`Script_Interp` case 7 — the game ticks the sub there too, the `Script_Update` jal at 0x1d9e4c — and killed by
`SubScriptEnd`'s loop-persistence condition), and **text** (op 0x1e → `Text_AddMsg(-1, 0, 4,
Txt_BindLabel(label), 0, time)` 0x1db2b8; the raw label drains `Drones::script_msgs` → the HUD's message queue
bound through the level's Lang table). Every command is op + args + 1 trailing byte (`Script_Run` 0x1d8548:120)
— `nf_formats::nis`' decode lens for 0x13/0x14/0x18/0x1b/0x1e were fixed to 6/16/5/8/8 and typed (`Cmd::Fade/
Sprite/Light/Text`). Not yet: `SP_Hit` bullet / object flags (07000007's urn, mask 2), AI volumes and pickups
made by scripts, cut-scene skipping's view-side fade arm, `Corona_2`-class generic entity glist drawing.

**The loadable cut-scene packs are ported (fix-cutscenes)** [M]: `SP_LoadScript`'s `isLoadable` arm
(0x1dbc08 → 0x1d69a0 → 0x1d69e8 → `LoaderLoad` 0x1d4a00) streams the pack named with the level id's (flag+7)
nibble (flag 1 → `078000NN.bin`, 2 → `079000NN.bin`, 3 → `07A000NN.bin`) from the level archive's type-0xC
records (`LoaderProcess` 0x1d4880 case 0xC: u32 hash + u8 flag at +4). The port reads those records in
`nf_view::load_with_carry` and merges the pack's kind-7 scripts into the script lib and kind-4 sequences into
the animation set at level load (the game streams lazily on first use; the port loads eagerly — same content,
no RNG order impact). `docs/systems/levels.md` §5 lists every loadable cut-scene per level: the party's
Drake fly-through (0x06000072), the party hand-back (0x06000073) and the gondola intro (0x06000077) were the
user's two broken cut-scenes; live tests in `crates/nf-view/tests/fix_cutscenes_live.rs` drive Bond into the
real trigger volumes (`step_bond` → TouchOnce → ch → SP) and fail on the pre-fix code. Still unported: the
`SP_Hit` proxy bits (07000007's urn break, mask 2) and the `GameFlow_PushState(12.0, 0xD, 0xFF)` streaming
loading screen (`SP_Update` state 0's `isLoadable` arm — the port loads eagerly, no screen).

**Event 0x12 (LoadLevel)**: ported (`Request::LoadLevel` → `Drones::level_exit` → `main.rs::pending_level`). No NIS
script on 07000005 fires kind 0x12 — the truck script 0x06000A22 fires only 0xC (keep objects, tick 5890) and 0xD
(channel 0x73 = 1, tick 5412). The level transition on 07000005 is the placement-0xE8 trigger at the castle door, not
a script event. Set `NF_SCRIPTLOG=1` to log every script event fired (tick, frame, player #, hash, stream, kind, params).

## Cameras [M, L]

`Script_CameraStart` (0x1d9140): u8 spline, u16 first key, u16 last key (the camera uses first..last − 1), skipped
while SCRIPTINFO flag 0x20; pushes the camera states and enables camera 4 (the script's), borders on.
`Script_Interp` case 4 → `Script_UpdateCamera`: the keys' matrix is the camera's (rows right, up, forward, position:
+Z forward — key 0 of 0x060008C8 points from (83.2, 21.0, 96.2) toward the waiting truck [D]) and
`Camera_CalcViewAngles(key scale × viewer+0x114)`; `+0x114` = 4/3 [L], and the argument is the vertical field of view
in radians (gameplay 60° = fy 388.06 on 448 lines), so 0x060008C8's 0.32 is ~24.5°. `Script_CameraEnd` / the stream's
end pop back to the game camera.

## Player hand-off [M]

Event 4 `Player_Disable`. Event 6 sets flag 0x100 and copies the previous stream's object matrix (or the player's) to
`+0x40`, p0 to `+0xAB6`; `Script_Free` then calls `Player_Enable(players, +0x40, +0xAB6)`. 0x060008C8 (the gatehouse,
started by the script trigger on channel 0x5C — a 5 × 5 plate at y 27.05 that Bond touches standing on the wall top at
24.75 above the waiting truck): disable at once, Bond's jump into the truck (anim 0x06000A24 on skin 0x05000007) under
the script camera, and after 143 ticks Bond on the bed at (92.254, 19.424, 103.576).

## Riding [M]

`Player_FeetOnPoint` (0x194188) casts cc + 0.6 below the player; when the hit belongs to an object (not types 2, 3, 5,
0x11, 0x12, 0x1A) `Control_InheritVelocity(player, object)` (0x133e88) adds the object's translation since the last
tick (`+0x30 − +0x40`) and, when its heading changed, turns the player's offset about the object and his yaw by it.
The port: `CollisionWorld::ground_motion` (the moving set keeps each object's previous matrix by id) applied to Bond
before his movement (*suggested* order: the game does it in the collision handler).
