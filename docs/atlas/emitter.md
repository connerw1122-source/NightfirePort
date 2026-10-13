# The emitters (placement type 0xf2, class 0x32) — particles: smoke, steam, sparks

`parsemap_create_dynamic_objects` 0x1d04f0 case 0xf2 → `Emitter_CreatePlist` (0x17b848, 276 B)
→ `Emitter_Create` (0x17ae00, 640 B); `Script_CreateEntity` case 0xf2 (0x1d8968:163) creates
script-driven ones the same way. Update `Emitter_Update` (0x17b080, 1668 B), draw `Emitter_Draw`
(0x17b708, 344 B), kill stub `Emitter_Kill` (0x17b960). Ported in `crates/nf-game/src/world_fx.rs`
(fix-world-fx); the def parse is `nf_formats::level::EmitterDef::parse_block`.

## The defs — map block 0x30 (`parsemap_block_particles` 0x1d2060 → `Emitter_LoadDefs` 0x17ab40)

Block 0x30 of the per-level .bin (`%08X.bin`, parsemap_handle_block_id case 0x30 0x1d1020:159).
`u32 count`; per def: `u32 tag` (low byte must be 4 else the load aborts [M 0x17ab40:89-91];
tag>>8 = the blend flag def+0x3c), `u32 hash` (the lookup key = placement param 0), `u32 tex hash`
(−1 → 0), `u32 model hash` (−1 → 0), `u16 max` (def+0x38), `u16 batch` (def+0x3a), 10 floats in
FILE order → mem +0x30 (sprite size), +0x0c life base, +0x10 life var, +0x14 gravity, +0x18 speed
base, +0x1c speed var, +0x20 pitch base, +0x24 pitch var, +0x28 yaw base, +0x2c yaw var [M the
local-to-struct stack mapping], `u32 keycount`, then keycount × 5 floats stored (r·0.5, g·0.5,
b·0.5, a, t) [M 0x17ab40:136-149]. An unresolvable hash means NO object [M 0x17ae00 hashtable
0 → return 0] — the port skips the placement the same way.

## Placement params (level_tag+0x2c+4k) → behaviour

| param | meaning |
|---|---|
| 0 | def hash |
| 1 | rate: 0 → −1 (emit while enabled), 1 → 0 (one-shot), n → n/60 s countdown [M 0x17b848:20-32] |
| 2 | ON channel (emit while set) |
| 3 | OFF channel (emit while CLEAR — inverted) |
| 4 | OUT channel: driven 1 while any particle lives, 0 otherwise (+ time stamp) [M 0x17b080] |
| 5 | blink period in ticks: ON half of it, OFF all of it [M 0x17b080:105-120] |

## The sim (Emitter_Update 0x17b080)

- Slots: `max` of them, age starts 99999.0 (0x47c34f80, dead), life 0 [M 0x17ae00:133].
- Per tick per alive slot (age < life): colour = `ColourGetValue` 0x17a938 at age/life (last key
  with t ≤ x+0.001; lerp; the rgb land in the draw bytes at the keys' half range); vel.y += dt·g·−9.8;
  pos += vel·dt; age += dt. The OUT channel re-sets 1 (+ stamp) while anything lives.
- Spawn pass (enabled, blink on, batch > 0): dead slots (life < age) reset age 0 and fire until the
  batch cap; `expired` (rate counted out / param 1 = 1) lets only the never-spawned slots (age ≥
  99999) fire [M the `age < 99999` skip]. Spawn: 4 draws — yaw, pitch, speed, life — the direction
  from PS2Sinf2 0x11f968 (BOTH outputs are sines: the +π/2 idiom gives the cosine; RotMatrixX
  0x1e46a0 proves the convention) rotated by the object matrix; pos = the emitter's.
- Death: 3 all-dead ticks (dead_ticks data+0x49 counting while `expired` or already counting) set
  obj+0xFE bit 0 [M 0x17b080:300-320]. Continuous emitters (rate −1, not expired) never die.

## Draw (Emitter_Draw 0x17b708) and the port's handoff

Path A (model hash 0): `psiDrawParticleList` 0x122d50 — billboarded sprite quads, texture =
def+0x04 via `psiFont_SetTexture` [M 0x119910:19], blend def+0x3c ? additive : alpha, size =
def+0x30 (screen-space scalar `4·size/(PS2ParticleSize·PS2SnowSizeMult)`, 0x11e2e8:67). Path B
(model ≠ 0): one tinted spinning mesh per particle. The port exposes
`WorldFx::particle_sprites()` (pos, size, rgba, additive) and draws path-A sprites as camera-facing
quads (`Sim::particle_quads`, appended to the snow pass in nf-view/src/lib.rs `snow_for`).

**Deviations [S]**: the 4 spawn draws per particle come from the per-object counter hash, not the
shared stream (the gas.rs/searchlight discipline — the bit-exact stream must not gain draws); the
sprite texture (def+0x04) is not bound — the quad colour is the slot rgba alone; the screen-space
size formula is taken as the world-space half-extent. Path B is not rendered (no def on this disc
resolved a model hash — unverified [S]).

Levels with 0xf2 on this disc: 07000002, 07000014, 07000015 (docs/systems/levels.md §7.1).
