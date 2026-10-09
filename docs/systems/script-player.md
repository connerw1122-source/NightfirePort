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

`nf-game::nis`: the player states, `Script_Update` / `Run` / `Interp` (linear and spline keys), commands 04–0C, 12
(events 3, 0xC, 0xD, 0xE, 0x11; others recorded), 16 / 17 (sound requests), entity and animation streams. Not yet:
camera, fade, sprite, light, sub-script, text commands (the stream stops on them), `SP_Hit` bullet / object flags,
the entity behaviours (triggers, AI volumes, pickups) — the objects are created and moved, nothing reacts to them yet.
