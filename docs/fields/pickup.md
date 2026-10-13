# PICKUPINFO (the pickup instance, 0x40 bytes at obj+0x110; class 0x2F)

Proof levels: see [README.md](README.md). Instance size 0x40 from `control_create_object(0x40, …)` inside
`Pickup_Create` 0x18dda0 [M 0018dda0_Pickup_Create…c:108]; class byte `obj+0xFF = 0x2F` [M :163]. Level:
ee_start_05 (07000005 at start) unless noted; a second snapshot pass on ee_start_07000001 is marked [L-01].

The whole struct is memset 0 by `control_create_object`; `MP_UnregisterPickup` 0x18a058:20 re-memsets it
(0x40) and restores `+0x30 = 0xFFFF` when an MP pickup is unregistered — the only whole-struct writers.

## Layout

| Offset | Type | Meaning | Proof |
|---|---|---|---|
| +0x00 | f32[3] | fall velocity per tick (vec3; +0x0C rides in the 16-byte stores). Zeroed at create, integrated in state 0, zeroed on landing | w `Vec_Zero` [M 0x18dda0:133], `+= WldGravity·REC_FRAME_RATE²` (REC_FRAME_RATE = 1/60) + store [M 0x18e148:89-100]; zeroed on landing [M 0x18e148:169-171]; r same lines. **[L-read obj 0x16d2760 → 0x16d2870+0x00 = (0,0,0)]** (resting) |
| +0x10 | f32[3] | sphere offset = matrix translation (obj+0xC0) − world-sphere centre (obj+0x80), written once at the end of `Pickup_Create`; the landing adds it back so the glist centre rests on the hit plane. 16-byte store; 4th word uninitialised noise | w [M 0x18dda0:190-195 (`_vsub(obj+0xC0, obj+0x80)` → `_sqc2` → `*(iVar1+0x10)`)], r [M 0x18e148:163-165 (`pauVar2[1]`)]. **[L-read 0x16d2870+0x10 = (−0.00557, −0.05523, 0.00770)]** (ammo pile); [L-01 armour pickups read (0,0,0) — glist centre on the object origin] |
| +0x20 | u16 | state: 0 falling, 1 resting, 2 taken/respawn-wait (hidden by obj+0xF0&0x10) | w create rest arg: 0 (dropped) / 1 (rest) [M 0x18dda0:185,188]; → 2 on take [M 0x18e650:346]; → 1 when the respawn wait ends [M 0x18e148:188]; → 1 on landing [M 0x18e148:173] (+ `MP_RegisterPickup` there [M 0x18e148:174]). r state gate [M 0x18e148:88 (`pauVar2[2]` → +0x20), :182]; **[L-read: 4/4 pickups on 05 and 4/4 on 01 read 1 (resting)]** |
| +0x22 | u16 | kind (0 weapon, 1 ammo, 2 key, 3 armour, 4 label, 5 message/clear +0x888, 6 Bond bonus; 7–11 intercepted → `Pickup_CreateFromSet`) | w create arg 5 [M 0x18dda0:122]; r `Pickup_Handler` switch [M 0x18e650:33,58,166], MP register kind-0 debug [M 0x189d80:24], bot goal scoring [M 00126b88_BOTSTATE_pickGoal…c:422,425]. **[L-read: 05 = 1,1,3,3; 01 = 3,1,1,3]** |
| +0x24 | u16 (s16) | weapon id (−1 refused at create); the key index for kind 2; the raw id for the take message. s16 only via the `param_6 == -1` check | w create arg 6 [M 0x18dda0:119, −1 check :89, 0x2C→4 on 07000008 :116-118]; r owned-check/equip [M 0x18e650:38-39,141,218], message name [M :246,252,266], bot goals [M 0x126b88:432,434,438,461]. **[L-read: 6 (P2K/ammo), 0x47 (body armour)]** |
| +0x26 | u16 | rounds for kinds 0/1 (the amount `Player_EquipWeapon/Ammo` gets); the bot armour addend on the drone path | w create arg 7 [M 0x18dda0:120]; r [M 0x18e650:65,76,101,110,128 (bot armour `+0x769 += +0x26`),192,207,211]. **[L-read: 30 (ammo), 50 (armour)]** |
| +0x28 | u16 | switch channel set to 1 (+ timestamp) when taken; 0 = none | w create arg 8 [M 0x18dda0:121]; r [M 0x18e650:330-333 (`switch_channels[+0x28] = 1`)]. **[L-read: 0,0,0,0 on 05; 0,0,0,13 on 01 (the armour on ch 13)]** |
| +0x2A | u16 | take sound; 0 stored as 0xFFFF at create; overwritten at take time per kind (0xF5 weapon/ammo msg path, 0x141 armour, 0x14C bonus) before `Sound_Play3D` | w create arg 9, 0 → 0xFFFF [M 0x18dda0:123-128]; take-time writes [M 0x18e650:290,300-301,319-321]; r `Sound_Play3D(..., +0x2A, obj+0xC0, …)` [M 0x18e650:336-337]. **[L-read: 0x13F (05 ammo), 0xFFFF (05 armour), 0xF5 (01 ammo), 0x141 (01 armour)]** |
| +0x2C | u16 (s16) | respawn: 0 = delete when taken; 0xFFFF = stays takeable forever; else hidden for `FRAME_RATE_INT·(+0x2C)·10` ticks (s16 compares at 0x18e650:338/344) | w create arg 11 [M 0x18dda0:129]; r take consumption [M 0x18e650:338-348], respawn wait [M 0x18e148:183-184 (`*(ushort*)(pauVar2[2] + 0xc)`)]. **[L-read: 0 on all 8 pickups (05+01) — delete on take]** |
| +0x2E | u16 (s16) | lifetime in ticks; 0 = forever; decremented every survived tick, 0 → `obj+0xFE \|= 1` + MP unregister | w create arg 12 [M 0x18dda0:131]; r/w decrement+expire [M 0x18e148:254-258 (`*(short*)(pauVar2[2] + 0xe)`)], r MP eviction pick (smallest lifetime) [M 0x189d80:59-60]. **[L-read: 0 on all 8 (no placed-pickup expiry on 05/01)]** |
| +0x30 | u16 | MP registry slot (0xFFFF = unregistered); written by `MP_RegisterPickup`, reset by `MP_UnregisterPickup`. NOT the label — the 16-byte +0x30 store in some Ghidra prints is the vec store below | w 0xFFFF at create [M 0x18dda0:130,198 — the second write + `MP_RegisterPickup` run when MP ∧ state ≠ 0 [M 0x18dda0:197-199]]; = slot on take-landing registration too [M 0x189d80:98 via `Pickup_Update` 0x18e148:174]; reset [M 0x18a058:21]; r registry match [M 0x18a058:11-13]. **[L-read: 0xFFFF ×8 (SP levels, never registered)]** |
| +0x34 | u32 | `Txt_BindLabel` hashcode for the kind-4 message | w create arg 13 [M 0x18dda0:132]; r kind-4 message [M 0x18e650:304-305]. **[L-read: 0 ×8]** |
| +0x38 | u16 | spin: 1 = rotate about +Y by REC_FRAME_RATE rad every tick (`RotMatrix((0,1/60,0))` concatenated into obj+0x90). Set for kind 6 or any MP pickup | w [M 0x18dda0:138-144]; r spin tick [M 0x18e148:179,193-201 (`*(short *)(pauVar2[3] + 8)` = +0x38)]. **[L-read: 0 ×8 (no spinning pickup on 05/01)]** |
| +0x3A..+0x3F | — | pad (inside the 0x40 block) | no reader/writer found [M —count 0] |

obj_tag header fields this class drives (not instance): `obj+0xD8` = the celglist [M 0x18dda0:134];
`obj+0xF0` = `(glist+0x44 & ~0x2000) | 0x840`, kind 6/MP `&= ~0x800`, kind 3 `&= ~0x800 \| 0x80000`
[M 0x18dda0:135-146], `&= ~0x10` on respawn end [M 0x18e148:187], `\|= 0x10` on hide [M 0x18e650:345];
`obj+0xFA \|= 0x44` [M 0x18dda0:136], `\|= 0x20` while falling/interp [M 0x18e148:106];
`obj+0xEC` = the take timestamp [M 0x18e650:347], read by the respawn wait [M 0x18e148:183] — but until a take
it holds the create frame (`GameState[52]`, `control_init_object` 0x134630); **[L] all four 05 pickups read
0x1A676 = 108150**, one tick before the first drone's 0x1A677;
`obj+0xFE \|= 1` on take-with-respawn-0 and on lifetime 0 [M 0x18e650:339, 0x18e148:256].

## Readers per offset (complete list found)

Ghidra bases the instance pointer as `undefined1 (*)[16]` in `Pickup_Update` — `pauVar2[k]` = byte 0x10·k;
`pauVar2[2] + 0xc` = +0x2C, `pauVar2[2] + 0xe` = +0x2E, `pauVar2[3] + 8` = +0x38.

| Off | Readers | Writers |
|---|---|---|
| +0x00 | Pickup_Update :89-105 | Pickup_Create :133 (zero), Pickup_Update :100, :170 |
| +0x10 | Pickup_Update :163 | Pickup_Create :195 |
| +0x20 | Pickup_Update :88, :182 | Pickup_Create :185, :188; Pickup_Update :173, :188; Pickup_Handler :346 |
| +0x22 | Pickup_Handler :33, :58, :166; MP_RegisterPickup :24; BOTSTATE_pickGoal :422, :425 | Pickup_Create :122 |
| +0x24 | Pickup_Handler :38-39, :141, :218, :246, :252, :266; MP_RegisterPickup :25; BOTSTATE_pickGoal :432, :434, :438, :461 | Pickup_Create :119 |
| +0x26 | Pickup_Handler :65, :76, :101, :110, :128, :192, :207, :211 | Pickup_Create :120 |
| +0x28 | Pickup_Handler :330-332 | Pickup_Create :121 |
| +0x2A | Pickup_Handler :336-337 | Pickup_Create :123-128; Pickup_Handler :290, :300-301, :319-321 |
| +0x2C | Pickup_Handler :338, :344; Pickup_Update :184 | Pickup_Create :129 |
| +0x2E | Pickup_Update :254-255; MP_RegisterPickup :59-60 | Pickup_Create :131; Pickup_Update :255 |
| +0x30 | MP_UnregisterPickup :11-13 | Pickup_Create :130, :198; MP_RegisterPickup :98 (create-time and landing-time registration, 0x18dda0:199 / 0x18e148:174); MP_UnregisterPickup :21 |
| +0x34 | Pickup_Handler :304-305 | Pickup_Create :132 |
| +0x38 | Pickup_Update :179, :193 | Pickup_Create :139, :143 |

No other function touches a PICKUPINFO (the MP registry `MPpickups` @ 0x2a4b60 holds obj pointers +
per-pickup bot-visit floats at +0x70 — a different 0xa0-byte struct; `MP_Pickup_Process` 0x18a168 and
`MP_resetPickupBotVisitTimes` 0x18be50 touch only that table, never the instance).

## Param map (from `Pickup_Create` 0x18dda0)

Signature `(VECTOR* pos, VECTOR rot, MATRIX* M, celglist_tag* glist, ushort kind, short id, ushort rounds,
ushort switch, int sound, char rest, ushort respawn, ushort lifetime, u32 label)` — args 9-13 reach it on
the stack (decompiler drops their stores at the call sites; each field's write line above is the evidence).

| arg | Instance offset | note |
|---|---|---|
| pos/rot | obj transform | pos NULL → M's translation row [M 0x18dda0:96-103] |
| M | obj+0x90 | NULL from parsemap [M 0x1d04f0:459-462] |
| glist | obj+0xD8 | placement's object (parsemap) or `weapon_data[id]+0x80` hashcode→glist (`Pickup_CreateSimple` 0x18f348:45, `Pickup_CreateFromSet` 0x18f1b0:58) |
| 5 kind | +0x22 | 7-11 → `Pickup_CreateFromSet` first [M :104-105]; kind 6 without a gold medal → refused [M :93-95] |
| 6 id | +0x24 | −1 → refused [M :89] |
| 7 rounds | +0x26 | |
| 8 switch | +0x28 | |
| 9 sound | +0x2A | 0 → 0xFFFF |
| 10 rest | state +0x20 | 0 = dropped: `Mat_Align2Up` + `Collide_SphereIntersect` push-out + translation.y −= radius − \|glist centre y\| [M :164-186]; ≠ 0 = rest in place |
| 11 respawn | +0x2C | |
| 12 lifetime | +0x2E | |
| 13 label | +0x34 | |

Placement side (type 0xF0): params 0-3 = kind/id/rounds/switch from `level_tag+0x2C+4k` [M
0x1d04f0:457-462]; the trailing five stack args are decompiler-dropped there. `Pickup_CreateSimple(M, id,
rounds, kind)` (drops): refuses rounds 0 / ids 1, 0x45, 0x47, 0x50, 0x51, 0x53; 0x6A/0x6E → 0x33; id−1 > 0x70
refused [M 0x18f348:11-44]; the visible call passes 8 args, the drop-path stack args (sound 0xF5, rest 0,
respawn 0, lifetime 7200, label 0) are [S docs/systems/pickups.md] — the C here does not show them.

## Live checks

Instance = u32 at obj+0xE0. Walk: `DynamicObjList` 0x2705a0 → first at head+0x14 → next obj+0x14; filter
`obj+0xFF == 0x2F`. All four 07000005 pickups: obj 0x16d2760 / 0x16d28c0 (ammo, kind 1 id 6 rounds 30 snd
0x13F), 0x16d2a20 / 0x16d82e0 (armour, kind 3 rounds 50, obj+0xF0 = 0x80060).

1. **+0x2C respawn is the port's missing half-take mechanic.** Probe a placed pickup:
   `python -I tools/trace/fieldprobe.py watch <obj+0xE0 pointer>+0x2c` then take it with respawn forced:
   poke `+0x2C = 0x003C` (60) before touching → the pickup should vanish (obj+0xF0 |= 0x10, state 2,
   obj+0xEC = now) and reappear after 600 ticks (10 s); `+0x2C = 0xFFFF` → it should never vanish. If the
   game re-hides immediately or never, the respawn formula (`FRAME_RATE_INT·respawn·10`, 0x18e148:184) is wrong.
2. **+0x2E lifetime on a drop.** Drop a gun (kill a guard) and watch the new object's +0x2E count down from
   7200 (`watch`, 1/frame). Reaching 0 deletes the object (class 0x2F disappears from the walk).
3. **+0x38 spin.** The 07000005 gold-medal/MP path is dead in SP, so force it: poke a pickup's +0x38 = 1 →
   the model should rotate ~0.0167 rad/frame about Y (poke 0 → still). The game rewrites obj+0x90 every
   tick while +0x38 ≠ 0, so poke the field, not the matrix.
4. **+0x10 sphere offset** predicts the rest height: for the ammo pile, centre (obj+0x80) = translation
   (obj+0xC0) − (−0.0056, −0.0552, 0.0077) — check live with `fieldprobe.py dump` that obj+0x80 − obj+0xC0
   equals the stored +0x10 (it is fixed at create; a mismatch means another writer exists).

## Port mismatches (crates/nf-game/src/pickup.rs @ master; report only)

1. **No respawn / state 2 at all.** The port's `Pickup` has no +0x2C field and no hidden-restore: every
   taken pickup is deleted (`drone.rs:4481 p.gone = true;`) or expires by lifetime (`drone.rs:4499-4501`).
   The game hides for `FRAME_RATE_INT·respawn·10` ticks and restores (0x18e148:182-189) or never deletes
   (respawn 0xFFFF, 0x18e650:344). No live impact on 07000005/01 (all 8 pickups have +0x2C = 0 [L]), but
   any level with respawning armour/ammo would behave wrong.
2. **kind 5 unhandled — the game CONSUMES it.** `pickup.rs:381 _ => None,` — the port leaves a kind-5
   pickup on the floor forever. The game's `Pickup_Handler` case 5 (0x18e650:229-233) clears
   `BLData+0x888` for a class-3 taker and falls through to the shared tail: message 0x2000013, sound,
   switch channel, consume. (What +0x888 is — likely the taker's "has bonus/gadget" word — is not chased here.)
3. **`Taken::Label`/kind 4 drops the message and the +0x2A sound** — the game binds +0x34's label and shows
   it (`Text_AddMsg`, 180 s); the port consumes silently (documented in pickup.rs:361-364).
4. **Fall's dynamic-hit branch missing.** The game, when the landing ray first hits a non-static object
   (class ∉ {2,5,0x11,0x1A}), calls `Control_InheritVelocity(obj, 3.0)` and keeps falling that tick
   (0x18e148:118-149); the port's `Pickup::fall` (pickup.rs:172-191) only tests the static world, so a gun
   dropped onto a live drone/body rests differently (edge case).
5. **Landing formula — now [M], was [S].** `Pickup::fall`'s rest position `hit − ray_unit·|centre_y| + (t − c)`
   (pickup.rs:184) matches the decompile exactly: HITDATA+0x30 = the unit ray direction (normalised from
   the HITTEST at `Collide_Intersect` 0x1ebaa8:394 → `Vec_Normalise` 0x1e3000, dst first), HITDATA+0x40 =
   the hit position (+0x44 = its y, the "passed the hit" test 0x18e148:116), landing =
   `HIT+0x40 + HIT+0x30·(−|glist+0x24|) + info+0x10` (0x18e148:150-168). The port comment at :171
   ("*Suggested*: `HIT+0x30` taken as the unit ray") can be promoted.
6. Take-path channel write, per-kind sounds, armour-50 refusal, upgrade_weapon tables: **agree** with
   0x18e650 (drone.rs:4480-4495, pickup.rs:344-383).
7. **Create-time MP registration only.** The port never registers drops with an MP registry (no equivalent of
   `MP_RegisterPickup` at create 0x18dda0:197-199 or landing 0x18e148:174, no `+0x30` slot, no
   `MP_UnregisterPickup` on consumption 0x18e650:341 / expiry 0x18e148:257) — invisible in SP, structural for
   any future MP bot-goal work (`BOTSTATE_pickGoal` 0x126b88:422-461 reads kinds/ids through it).
