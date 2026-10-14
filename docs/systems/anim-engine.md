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
  0x602 for 0x10, 4 → 0x604 for 0x16 / 0x17; pitch Rand(500) − 250); op 4 `[frame, event, arg]` — 0 / 4 / 5
  footsteps, 1 player toggle, 2 the on-event callback (drones: `DroneAnim_EventFunc` with arg), 3 stop sound, 6
  effect, 7 fire a bullet from the weapon datum (+ light, smoke). A command fires when its frame lies in the passed
  interval (wrap-aware).
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
