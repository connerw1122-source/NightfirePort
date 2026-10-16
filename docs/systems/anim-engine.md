# Animation engine: scripts, blending, events, root motion

Build: ACTION.ELF `e7ffdcb3…`. Tags: [M] read in our decomp export, [S] suggested. Read 2026-10-09 (this session):
`AnimObjectUpdate` 0x1befd8, `AnimScriptNew` 0x1c1398, `AnimScriptInit` 0x1c1600, `AnimScriptAdd` 0x1c6d40,
`AnimScriptAppend` 0x1c7000, `AnimScriptAppendBlend` 0x1c17c0, `AnimScriptEnd` 0x1c1930, `AnimScriptTick` 0x1c2630,
`AnimProcessScriptCmds` 0x1c1b38, `AnimListBuild` 0x1c0fc0, `AnimFrameResolve` 0x1c35f8, `AnimFrameBlend` 0x1c2b80,
`AnimFrameSet` 0x1c75a0, `AnimSeqSet` 0x1c7460, `AnimScriptSetAnimRotation` 0x1c7208, `DroneAnim_SetHTAnim`
0x13b1c8. Builds on docs/formats/animation.md (sequences, poses) and docs/systems/drone-animation.md (the drone
call layer).

## The animation object [M]

`obj+0xDC` = sAnimObject; `+0x9C` the list of playing **scripts** (sAnimScript, 0xC0 bytes, linked by `+0x44` prev /
`+0x48` next; at most 32). A script:

| Offset | Field |
|---|---|
| +0x50 / +0x54 | its sequence states (body; facial when the skin has facial bones) |
| +0x5C / +0x74 | the script data / its hashcode |
| +0x78 | flags: 0x20000000 just started (commands of the start frame run), 0x80000000 / sign: looping (header +6 < 0), 0x10000000 finished, 0x40000000 wrapped this tick, 0x800000 ticked, 0x100000 blending out, 0x400000 do-not-blend-out, 0x200000 faded out, 0x20000 anim rotation, 0x8000 rotation reference, 0x40000 (facial sequence) |
| +0x84 | creation stamp (`AnimScriptTimeStamp`, +1 per script): the blend order |
| +0x90 / +0x94 | time (frames, float, from 1) / last tick's |
| +0x98 | speed (frames per tick; −1 when started reversed) |
| +0x9C | blend weight = `+0xA8` / `+0xAC` |
| +0xA8 / +0xAC | blend clock / blend length (both 1.0 at init) |
| +0xB0 | length (header +4) |
| +0xB2 / +0xB3 | header +6 low byte / (header +6 >> 9) & 0x3F |
| +0xB4 | speed mode (0 normal × `FRAME_RATE_MUL`; 1 distance table; 2 synced to another script; 3 phase) |
| +0xB5 | end mode (0 / 3 stop, 1 loop, 2 ping-pong; set to 1 when looping) |
| +0xB6 | fade: 1 in, 0xFF (−1) out, 0 none |
| +0x80 | facial marker: 0 (nothing), 0xe0000000 idle / 0xe0000001 blink / 0xe0000002 talk — `AnimFacialBlend` picks slots by it |
| +0x60 / +0x6C / +0x70 | done flag pointer / on-stop / on-event callbacks |

## Adding scripts [M]

- `AnimScriptAdd(obj, hash)`: deletes every script, then one new script (`AnimScriptNew` + `Init`).
- `AnimScriptAppendBlend(obj, hash)`: every playing script without 0x400000 gets fade = −1, flags |= 0x500000, blend
  clock = blend length = 8.0; then a new script is appended (if fewer than 32).
- `AnimScriptInit`: time = speed = 1 (reversed: speed −1, time = length), fade +1, blend clock = length = 1.0 (full
  weight at once), stamp.
- `DroneAnim_SetHTAnim(speed, script, flags, start, drone, blend)`: blend = 0 (or not animating) → `AnimScriptAdd`;
  else `AppendBlend` with the old script's blend clock = length = `blend` frames, and — when the old script had not
  reached its end and drone `+0x574 & 0x100` — the old script gets anim rotation (blend 30) and the reference yaw
  `+0x54C`. Start frame 999 → the script's last frame, speed 0; flags bit 0 clear → one-shot (`+0x57A`, `+0x579`, on
  stop `AnimEndCallback`), set → looping (flag 0x80000000).

## Ticking (`AnimObjectUpdate` → `AnimScriptTick`, per tick) [M]

- For each script not finished: run the commands between last and this frame (start frame included on the first
  tick), advance: time += speed · `FRAME_RATE_MUL` (mode 0), then `AnimScriptEnd`: within [1, length] → keep; past it:
  end mode 0 / 3 → finished (0x10000000), clamped to the end (1 when reversed), the on-stop callback; 1 → wrap
  (± length), 0x40000000; 2 → reverse. Then the commands between the old and new time.
- Commands (`AnimProcessScriptCmds`; script +8): op 0 `[a, b, seq]` — the sequence (0x04000000 | seq) plays over
  frames a..b (frame = time − a + 1, `AnimSeqSet`); op 1 `[frame, sfx]` — a sound at the object (weapon-specific id
  swaps: 1 → 2 for weapons 7 / 9, 0x403 → 2 for 3 / 5, 0xFA → 0x1FB for 0x14, 0x106 → 0x284 for 0x1E–0x23, 0x1E1 →
  0x602 for 0x10, 4 → 0x604 for 0x16 / 0x17; pitch Rand(500) − 250); op 4 `[frame, event, arg]` — see the op-4
  dispatch below; a command fires when its frame lies in the passed interval (wrap-aware), latched by the per-command
  fired bits in the script's +0x7C (`iStack_c0` sets / clears them at 0x1c1b38:465-471), so each fires once per pass.
- Anim rotation (flag 0x20000; `AnimObjectUpdate`): the root bone's rotated forward → yaw; the change since last tick
  is added to the object yaw (`obj+0x54`) and removed from the root quaternion — the animation turns the object.
- Then `AnimSeqTick` per sequence (root motion, docs/formats/animation.md) and `AnimFrameResolve`.

## Blending (`AnimFrameResolve`, `AnimFrameBlend`) [M]

- Weights: fade +1 → blend clock += `FRAME_RATE_MUL` up to the length; fade −1 → −= until ≤ 0 (flag 0x200000);
  weight = clock / length.
- Pose: the scripts (not finished-and-faded) sorted by stamp; one → its pose; several → result = first, then for
  each next script: result = blend(result, next, 1 − weight(previous)). Root motion blends the same way.
- `AnimFrameBlend(t, a, b)`: per bone, translations a + (b − a)·t without overshoot, rotations `Quat_Slerp_Acc`
  (an accurate slerp; objects far away (LOD bytes ≥ 0xF1) take a cheaper path [S]).
- Within a sequence (`AnimFrameSet`): fractional frame f → blend(frame ⌊f⌋, frame ⌊f⌋ + 1 clamped to the last, f − ⌊f⌋).
- Afterwards scripts faded out (0x200000 with 0x100000) and those with fade 0 are deleted.
- Root motion is rotated by the object matrix — or by the saved rotation of the first script in anim-rotation mode —
  passed to the pre-translation callback (drones: `DroneAnim_PreTransCallback`) and added to the position.

## The facial weights (`AnimFrameResolve` → `AnimFacialBlend` 0x1be648) [M]

Per tick, for every object whose skin has facial morphs (`AnimSkin+0x2e` ≠ 0 — 22 on every face skin), resolve ends by
blending the morph weight array (`animData+0xb0`, the 22 floats `SkinIt` consumes). `AnimFacialBlend` reads three
SCRIPT slots through `AnimListBuild(obj, 0x200, marker)` — the predicate is the script's **+0x80 marker == marker**:

- `0xe0000000` **idle**: its +0x54 sequence's frame buffer copies straight over the weights.
- `0xe0000001` **blink** / `0xe0000002` **talk**: override a weight only when |w| > 0.01.
- The anim object's eye bytes then write indices 14/13 (from `+0x97`) and 11/12 (from `+0x98`) over the same threshold.
- **Under a script camera (`ScriptCam != 0`) only the talk slot applies, as a straight copy** — no threshold, no eye
  bytes, no idle/blink.

Who carries the markers:

- `AnimScriptInit` 0x1c1600:52-61: **every script started while `ScriptCam != 0` on a facial-bearing skin gets
  +0x80 = 0xe0000002** — a cut-scene's anim script IS the talk slot, so its facial window (the sequence flags bit 0x8,
  `AnimProcessScriptCmds` 0x1c1b38:165 → the script's +0x54 slot via `AnimSeqSet` 0x1c7460's `param_7` pick) feeds the
  weights directly: the 22-channel face track's channels are the morph weights, never bones. In gameplay new scripts
  get marker 0.
- `AnimFacialUpdate` 0x1be310 (gameplay): the idle/blink state machine appends the tables' scripts
  (Male/Female_Facial_State_Anims, [D ACTION.ELF .data 0x2c6ea8/0x2c6ec0]) with markers 0xe0000000/0xe0000001 and its
  `Rand_*` blink cadence.
- `AnimFacialAppend` 0x1c6490: marks the appended script 0xe0000002 (the gameplay talk slot); its caller is
  `NDrone2_SpeechSFX` 0x141b00 — a guard speaking a voice line animates his mouth through it.
- A facial-bearing actor with **no** facial window still reads zeros: `AnimScriptNew` 0x1c1398 allocates the +0x54
  slot for every facial-bearing skin (`AnimSeqNew` 0x1c7248 zeroes the 22-float buffer) and the ScriptCam talk-read
  copies those zeros — the neutral face. (`AnimFacialBlend`'s gameplay callers are additionally gated on the draw
  distance, min of animData+0x110..0x113 < 13; ScriptCam skips the gate.)

The window classifier: the sequence record's flags bit 0x8 [M 0x1c1b38:165]. Census over all 18,244 kind-4 sequences
on the disc: flags {0x00 × 17,363, 0x01 × 10, 0x08 × 871} and every flags-0x08 one carries exactly 22 channels — a
body track is 3·bones + 3·popcount(mask) (≡ 0 mod 3), so the bit and the count can never disagree [D].

## The op-4 event kinds (`AnimProcessScriptCmds` 0x1c1b38 case 4) [M]

The command words are `[frame, kind, arg?]` (`puVar12[1..]`). After the facial-sequence gate
(`animobj+0x58 & 0x40000` skips everything) and the fired-bit check, the kind switch:

| kind | effect |
|---|---|
| 0 | `ScriptFootstepFoot` toggles, then the footstep-type set below |
| 1 | the player toggle: `*(*(obj+0x1c) + 0xe0) + 0x964` (the SMI's weapon-anim byte) flips [M :374-379; `Player_SetWeaponAnim` 0x1a9338 sets it from weapon_data+0x68 & 8] |
| 2 | the script's on-event callback `*(*(script+0x70))(obj, animobj, script, arg)` [M :382-385] — drones get `DroneAnim_EventFunc` (fix-anim-events); `AnimScriptInit` 0x1c1600:51 zeroes +0x70, so player-only scripts (the death script) have no handler |
| 3 | `Sound_StopAllWithId(arg)` [M :387-389] |
| 4 / 5 | `ScriptFootstepFoot` = false / true, then the footstep-type set: `ScriptFootstepType` = 1 when the script's hash is 0x6000099 / 0x60000ae, else 2 [M :391-403] |
| 6 | `Effect_Create(1.0, *(obj+0xdc), obj, 0xc0)` [M :406-410] — see below |
| 7 | fire a bullet from the weapon datum + the muzzle `AnimDatumSetEntity` flash + `Light_Create` + `Gas_Init` smoke [M :411-457] |

The footstep consumer is `AnimObjectUpdate` 0x1befd8 (per object, after the script tick):
`ScriptFootstepType` is cleared at :126; for a **class-3 object** (`obj+0xff == 3`, the living
player) the walk accumulator at `SMI+0x924` (`+= |animobj+0xd8|`, the stride 1.65 — 0.4125 when
`obj+0xf6 == 1`) drives the footstep instead and the script's byte is ignored [M :159-176]; the
class-3 arm plays the hardcoded footstep sound 0x286 + `Sound_SetAlertness(0, h)` [M :198-202].
For every other class (a drone, or the DEAD player — the death marker sets `obj+0xff = 0x12`
[M `Player_CheckForDeath` 0x190810:72]) the script's `ScriptFootstepType` sets the anim object's
datum flag bit 4 (`*(animobj+0xdc)+0x60 |= 4`) [M :184-186], and the bit-4 block calls
`Effect_Create(1.0, animobj, obj, 0x42 | foot)` — foot 8 while `ScriptFootstepFoot`, else 4
[M :189-199]. Everything is skipped while a script camera is up (`ScriptCam != 0`, set by
`Script_CameraStart` 0x1d9140 / cleared by `Script_Free` 0x1da880) [M :188-189].

`Effect_Create`'s "HITDATA" here is the **anim object itself**, whose +0x40..+0x58 shadow holds
the last feet contact: `Player_FeetOnPoint` 0x194188:183-191 (`Drone_FeetOnPoint` 0x140660:204
for drones) `amemcpy`s the feet ray's 0x60-byte hitdata into `obj+0xdc` while grounded (skipped
while airborne — the same +0x50 byte reads 0xd/0xe then, `Player_HandleJump` 0x1916b0:194; the
fields doc's animobject.md +0x50 row carries both readings). The shadow's +0x40 is the contact
point (the sound position), +0x50 & 0x3f the contact's material — `Effect_Create`'s guard
`material != 0` [M 0x1ce688] mutes everything when it is 0, and `Bullet_init_casing_ex`
0x12f678:186 reads the same byte as the casing's material. `Effect_Player` 0x1cec48's 0x40 arm
plays the surface's **footstep** sound (`EffectInfo + material·0x88 + 0x16`) at the contact with
`Sound_SetAlertness(0, h)` [M 0x1de210] (silent to the AI) and its 0xc arm lays the footprint
decal (not ported). Kind 6's flags 0xc0 → `Effect_Body` 0x1cf400: the surface's **body** sound
(`EffectInfo + material·0x88 + 0x18`) at the contact with `Sound_ModAlertness(0.5, h)` [M
0x1de220] — the node's alertness halves. The columns are `nf_formats::sfx::SurfaceSounds`'s
`footstep` / `body` (parsed from the `EffectInfo` symbol).

## The death script 0x06000151 (fix-death-anim-events) [M 0x190630 case 0xd]

`Player_ChangeSubState(obj, 0xd/0xe)` (the death substate) deletes the anim set and
`AnimScriptAdd(obj, 0x6000151)` — the 0x60008e0 arm plays only while the gravity global
`DAT_002d8894` is 0.0, which retail never is (−9.4 from `bootup` 0x1c7e78:73 / `Player_Init`
0x1910f8:153). The script (07000005.bin, `nf-extract script-info … 6000151` — **the tool prints
args as hex**): one sequence window (1..297, seq 0x040000a3) + four op-4 commands — frame 5 kind
5, frame 8 kind 4, frame 0x98=152 kind 4, frame 0x116=278 kind 6. The dead tick runs
`AnimObjectUpdate` then `Player_HandleDeath` and nothing else [M `Player_Update` 0x193600:317-319;
the MP pause flag `GS_PausePlayer` 0x17d280 writes is not read there], so the script plays out
behind the red fade. Its clock runs 2.0 frames per dead tick — speed 1.0 [M 0x1c1600] ×
`FRAME_RATE_MUL` 2.0 [M `GS_SetRefreshRate` 0x17d2b8:15 = 60/FRAME_RATE; the capture ran the
30 Hz arm] — [L] PCSX2 trace: +0x90 reads 12, 32, 52 at dead frames 8, 18, 28. The death tick
itself ticks the just-added script through the alive path's tail `AnimObjectUpdate`
[0x193600:358] — the just-started window (1, 1], empty for this script — so the events land on
dead ticks 3 (frame 5), 5 (frame 8), 77 (frame 152), 140 (frame 278) with the clock at 2·tick−1.
Port: `nf_game::death::death_script_events` (Death.script_time / events / footstep_right) runs
the same (from, to] windows over `d.scripts[0x06000151]` each dead tick BEFORE the pane/channel
logic, plays the footstep / body sounds through the same SurfaceSounds columns at the floor query
under Bond (the drone-footstep pattern), pushes the body sound's hearing entry at half weight,
and records every event in `Death.events` — fix_death_anim_events_live 2/2 (fail-first shown).

## Port

`nf_game::anim` (WORKLOG 0048): AnimObject / AnimScript with the add / append-blend / tick / end / weight / delete
rules and blended fractional poses; the drones' danims and the view model play through it. **fix-audio2: the op-1
(sound) commands fire** — the drones' `script_commands` had them (footsteps/weapon events ride op-4), and the NIS
animation objects now fire them from `Script_Interp` case 2 (0x1d9a08 overrides the anim script's time with the
stream's frame and runs `AnimObjectUpdate` → `AnimScriptTick` → `AnimProcessScriptCmds(old, new)`, so an op-1 fires
on the (previous frame, current frame] interval; the first interp's window starts at 0 — the start frame's commands
run then). `Sound_Play3D(sfx, obj+0x30, 100, −1, −1, pitch Rand(500)−250, ref 0, custom 0)`; the shared RNG's
`Rand(500)` is consumed per fire (the pitch itself is never forwarded downstream). This is where the cut-scene
characters' lip-synced voice lines live (op-1 → STREAMS.BIN entries through the bank's negative FileRef) —
fix_audio2_live drives 07000001's intro, 07000005's ride dialogue (ScriptPlayer #298, gate ch 0x43) and 07000007's
fly-through through it. Not yet: facial
sequences, distance-table speed modes 1–3, head tracking (`AnimObjectHeadTrack`), the far-object blend path.

**fix-anim-blend (2026-10-13, branch s/fix-anim-blend):** the frame-blend side is now exact and
the speed-mode machinery exists. `nf_game::anim::slerp` is `Quat_Slerp_Acc` (0x1e4188) verbatim:
dot = q0·q1, `neg` = dot < 0 with **w0** (the q0 coefficient) negated after the branch — not q1;
the slerp branch runs while `0.01 < 1 − dot` (the port's old 0.9995 gate was wrong), weights
w1 = PS2Sinf(ωt)·(1/PS2Sinf(ω)), w0 = PS2Sinf(ω−ωt)·(1/PS2Sinf(ω)) (one reciprocal, then
multiplies), and there is **no renormalisation** (0x1e4188:33-36 add the weights as is; a rounded
dot past 1.0 gives 1 − dot ≤ 0 < 0.01 and lands in the linear branch, so no clamp is needed either)
— `slerp_negates_w0_and_never_renormalises` pins all of it. The drones' root motion samples the
same f/f+1 blend the pose does: `anim::root_translation_at` is `AnimFrameSet` (0x1c75a0) + the
`AnimFrameBlend` (0x1c2b80) translation rule (gate |frac| ≤ 0.0002, ⌊f⌋/⌊f⌋+1 clamped-lerp) applied
to the root channels, fed by `Drone::script_frame`'s fractional frame (seq.frame = script.time −
start + 1.0, `AnimSeqTick` 0x1c28d0 — the script clock is +0x90, a float; the port's `ftime` is the
same number). At speed 1.0 the fraction is always 0 and every frame is the integer it was before;
at a fractional `Drone_AnimInfo +0x14` speed (lean-out 176 plays at 1.5, run 0xDC at 0.7 on
07000009/0A) the root now moves by the blended frame instead of stepping whole frames while the
bones already moved fractionally. The wrap tick samples through `ftime` synced to its forced
integer times, and the loop wrap keeps `ftime`'s fraction. Speed modes (`AnimScriptTick` 0x1c2630,
byte script+0xB4): `anim::DistanceTable::build` is `AnimDistanceTableCreate` (0x1c4060 —
cumulative |root Δ| per frame, components masked by the axis bits, a segment under 1e-05 becomes
0.01, looping = frames−1 segments, table = total / count / 0.0 / cum), `distance_to_frame` is
`AnimDistanceTableDistanceToFrame2` (0x1c4270 — walked wrapped into [0, total) and stored back,
segment = the estimate then the walk, frame = segment + distance-into/segment-length, speed
wrapped in the stride's direction), `speed_for_mode` carries modes 0 / 2 (synced to the master
+0x4C, wrap by the master speed's sign) / 3 ((len+1)·phase − time) verbatim. Not yet wired: no
writer sets +0xB4 in the port — the game's mode-1/2 pairs come from `AnimSetUpdate` (0x1c0938)
building the player's walk sets (master +0xB4=1, slave +0xB4=2, `sAnimSet` +0x24 = 0.37 idle
accumulator, +0x28 = 0.5 stride factor [L docs/fields/animobject.md]), which is the remaining
work; the guards play mode-0 scripts at `Drone_AnimInfo +0x14`, as in the game.

**fix-anim-events (2026-10-13, branch s/fix-anim-events, G-030):** the disc-wide census (all 99
FILES.BIN archives, 18,231 scripts, 50,279 commands, 0 parse failures) shows only opcodes 0 / 1 /
4 exist; op-4 carries kinds 0 (34) / 1 (52, player toggle) / 2 (12,947 — `DroneAnim_EventFunc`) /
3 (53, stop sfx 18 / 61 / 1479) / 4 (6,925) / 5 (6,976, footsteps) / 6 (1,294, `Effect_Create`)
/ 7 (4, fire bullet — one script, 07000016/06000858), and kind-2's args are 0–16 and 18 with the
fire flags 0xF/0x10/0x12 (args 15/16/18) at 86% of the traffic. The port's `Drone::event_func`
now carries the missing `DroneAnim_EventFunc` (0x13b750) cases, all gated on `0.0 < drone+0xAC`
as the game does (the port's `script_commands` only runs the current script, so the game's
`script == drone+0x530` gate holds structurally): 2 → `DroneWeap_ChangeWeapon(drone+0x45)`
(disasm 0x13b7dc: `a1 = lb 0x45(a2)` in the dispatch's shared delay slot — the drone's setup byte,
NOT the event arg; `DroneWeap_ChangeWeapon` 0x17a6e0 is a 55-entry per-weapon jump table at
0x2fb740, of which the port carries the two SP-verified entries 1 and 6), 5 / 10 / 0xB →
`DroneWeap_CloseCombatImpact` (0x1789c0: `Bullet_init(-1, obj, obj, record, obj+0x30, facing,
0xffff)` — record 99 at 2.0 for the weapon-class-7 kick, else 100 / 0x65 / 0x66 at 1.75 with the
`|+0x1D4| ≤ 60°` facing gate; no accuracy roll, no muzzle light), 9 → `DroneWeap_PlaceTertiary`
(0x17a848 is a tail call to `DroneWeap_ThrowGrenade`), and the health gates on 1 / 6. The
`+0x45` swap byte (`weapon_swap`) and the melee/swap outboxes (`pending_melee`,
`pending_weapon_swap`) drain in `Drones::tick`. `DroneAnim_PlayFiringAnim` (0x13da48) is ported
inside `handle_firing`: for the recoil weapons — `+0x3E`, set by `DefaultInit` (0x14b300:801-811)
when the anim datum's weapon byte is 0x1C / 0x1D; the port keys it on the drone's weapon id — a
due shot fires only when the anim's 0x12 event set `+0x21` (with the game's `+0x558 == 0` /
`+0x538 == 0` gates), then plays the 0x5C recoil danim, zeroes the pending call's completion
(+0x596 / +0x598) and sets +0x57B; non-recoil weapons fire unchanged. Tests per ported kind on
real disc scripts (crates/nf-game/tests/anim_events_live.rs): melee 10 via 07000005/060002ba
(the punch connects in range, nothing out of 1.75), weapon change 2 via 07000001/06000330 (entry
6 = weapon 6 / set 4 / 16 rounds; an unported entry changes nothing), the 0x12 fire flag via
07000005/0600029f sets both +0x20/+0x21 through the real script, tertiary 9 via
07000012/060007ac spawns the grenade and spends one. Still unported (censused, with the
decompile evidence): op-4 kind 1 (the player's SMI +0x964 toggle — `Player_SetWeaponAnim`
0x1a9338 sets it from weapon_data+0x68 & 8; the port's player anim objects don't run script
commands yet), kind 6 (`Effect_Create(1.0, *(obj+0xdc), obj, 0xc0)` 0x1c1b38:406-410 — the port
has no general effect system, only the bullet impacts), kind 7 (the anim-driven bullet + muzzle
`AnimDatumSetEntity` flash + `Light_Create` + `Gas_Init` smoke, 0x1c1b38:411-457 — 4 commands on
the whole disc), `DroneAnim_EventFunc` 0xE (`Drone_SM_SendMsg(0xB, 10, …)` to the +0x2B0
partner's SMI +0x108 — the port has no partner messaging), the other 53 `ChangeWeapon` table
entries, the footprint decals (`ScriptFootstepFoot`/`ScriptFootstepType` → `AnimObjectUpdate`'s
`Effect_Create(floor hit, 0x42 | foot)`; the footstep SOUNDS are ported), the setup-word read of
`+0x45` (0x14b300:266 `[0x17]`), and `DroneWeap_PlaceTertiary`'s second argument's effect (the
decomp shows the ThrowGrenade tail call only).
