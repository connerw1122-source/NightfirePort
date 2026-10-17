# Hurt volumes (placement 0xE4, class 0x27) — the spec, from the decompile

Written for the port task that implements them. Everything below cites the decompile; nothing is
asserted from memory. 07000006 carries exactly one: **#373 "Hurt_A"**, the well shaft's kill volume
(docs/levels/07000006.md §5.10).

## `Hurt_Create` 0x17e870 [M]

`control_create_object(0x1c, pos, rot, 0)`; `ext = obj+0xE0` (the level-tag block). Fields:

| ext offset | source | meaning |
|---|---|---|
| +0x00 (f32) | 0 | the anim clock `t` |
| +0x04 (f32) | `count × REC_FRAME_RATE × FRAME_RATE / (u16)tag+4` | the clock increment per tick (`count` = `(short)tag+2 − 1`, the placement's object index − 1... `tag+2` reads the u16 at level-tag +2) |
| +0x08 (ptr) | `tag+8` | the framelist (`Spline_Interp`'s first arg) — the hurt object ANIMATES |
| +0x0C (u16) | `(short)tag+2 − 1` (0 when tag+2 = 0) | the key count / anim duration (the spline's time cap) |
| +0x10 (f32) | `tag+0x30` = **param 1** | the damage (`Hurt_GetDamage` 0x17ea18 returns it as float) |
| +0x14 (u8) | `tag+0x2c` = **param 0** | the damage type (`Hurt_GetType` 0x17ea50; 6 = fall in the port's damage table, types 1..7 with drowning 7) |
| +0x15 (u8) | `tag+0x40` = **param 5** | the one-shot flag (1 → state 2 after the first pass) |
| +0x16 (u16) | `tag+0x3c` = **param 4** | the "who" mask (ANDs the contact class bits: 1 = always, 2 = class 5, 8 = class 3, 0x10 = class 2/the player) |
| +0x18 (u16) | `tag+0x34` = **param 2** | the gate channel (0 = none; nonzero → the channel VALUE is the who-mask) |
| +0x1a (u16) | `tag+0x38` = **param 3** (0xffff when 0) | the activation sound id (`Sound_Play3D(100.0, 1.0, 1.0, id, obj+0x30, …)`) |

`obj+0xFF = 0x27` (the class; `control_funcs[0x27] = Hurt_Update`), `obj+0xFA |= 0x44`,
`Control_SetGList(obj, glist)` — **the volume's geometry is the glist object's collision** (the same
source the trigger volumes build from), `build_LinkToRoom(obj, 0, glb_world)`. If the gate channel
is set and LOW at create: `obj+0xF0 |= 0x20` (inactive until it rises).

#747's neighbour **#373**: params [6, 1000, 0, 0xffff, 1, 0] → type 6 (fall), 1000 damage, no gate,
no activation sound, who-mask 1, one-shot 0 (re-arms), anim duration = object index 373 − 1 = 372
keys [D #373; the framelist lives at `tag+8` — the placement block's +8 word, i.e. the HASHCODE
slot... **live check for the main agent**: dump `tag+8` for #373 on 07000006 and read the framelist
it points at — the decompiler's `tag` here is the level-tag the creator receives, whose +0x2C+4k
carries the params (the same layout SoundTrigger_Create uses), so +8 is BEFORE the params: it is
the tag's own +8 word, not a param].

## `Hurt_Update` 0x17e5d0 [M]

A state machine on `obj+0xF4`:

- **state 0 (armed)**: compute the who-mask `uVar5`:
  - param4 (+0x16) == 0 → `uVar5 = 1`;
  - param4 != 0 with a gate channel → `uVar5 = switch_channels[gate]` (the channel byte IS the mask);
  - param4 != 0 without a gate → walk the object's `obj+0xD0` HITDATA chain, OR-ing class bits
    (class 3 → 8, class 2 → 0x10, class 5 → 2), then `uVar5 = param4 & collected`.
  `uVar5 != 0` → state = 1 (the anim starts).
- **state 1 (animating)**: at `t == 0` play the activation sound (id +0x1a) at `obj+0x30`;
  `Spline_Interp(framelist, 0, key_count, t, obj+0x90, 0, 2)` — the pose animates;
  `t += inc`; `t > key_count` → `t = 0`, and param5 == 1 → state = 2.
- **state 2**: `obj+0xFF = 1` (the object retires).
- The gate arm: the gate channel RISING clears `obj+0xF0` bit 0x20 (re-arms a hidden volume).

**No damage is applied here** — the damage rides the COLLIDE system: a body intersecting the
volume's glist geometry gets the contact, and the victim's damage pass calls `Hurt_GetDamage`
0x17ea18 (+0x10) / `Hurt_GetType` 0x17ea50 (+0x14). The port's hook is wherever it resolves
damage volumes today (`bullet.rs`'s PlayerHealth damage types 1..7 — falls 6, drowning 7, hurt
volumes).

## Port plan (the next task)

1. `crates/nf-game/src/hurt.rs`: create from the 0xE4 placements (the same param layout as
   SoundTrigger: `tag+0x2C+4k` = param k), the volume from the placement's glist tris — reuse
   `triggers::build`'s glist-volume path.
2. Update: the state machine above (clock, spline pose via the existing door/SS interpolation,
   the sound), then the inside-test: the standing capsule vs the volume tris (the trigger
   walk's test).
3. Damage: `Player_Hurt(damage, …)`-shaped application through `health.pain_generic` with the
   type byte (the fusebox's live-wire hurt is the precedent: `player_hits` marker + `pain_generic`).
4. Tests: #373's row pinned (type 6, 1000.0, mask 1, one-shot 0, keys 372 [D + M]); a real-path
   test drops Bond into the well shaft (the well at (31.82, 39.79, 17.26) [D #65]) and asserts the
   1000-damage death; the sister stands at the well lip and takes nothing.
5. Evidence gap: the who-mask contact bits and the collide-pass reader are [M]-read above but the
   volume GEOMETRY resolution (which map object's tris) needs the disc dump — the census probe's
   `classify` path can print #373's glist object; a live PCSX2 check of `Hurt_GetDamage`'s caller
   settles the application point.
