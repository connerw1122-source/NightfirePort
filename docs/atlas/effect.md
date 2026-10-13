# The bullet-impact effects — decals, shell casings, debris, gas puffs

The sim side ported in fix-impacts (`crates/nf-game/src/effect.rs`, `crates/nf-game/src/casing.rs`),
the draw side in fix-impacts-render (`crates/nf-view/src/scene.rs` — the per-mesh Effect pools and
`frame_state`'s slot posing; the old flat-colour `effect_quads` is gone). G-041 / G-025.

## The game's path (ACTION.ELF)

`Bullet_CollisionHandler` 0x12ce80 walks the bullet's HITDATA and calls `Effect_Create` 0x1ce688:
weapon_data+0x70 & 2 → effect flags 0x61 (bullet + emitter + sound). GLASS (material & 0x3f == 0x10)
negates the normal for the debris basis [M 0x1ce728]. Debris: `Rand_Rand(EffectInfo+0x83)` + 1 pieces
→ `Debris_Create` 0x1aaa00 per particle (scale `0.4 + FRand(0.1)` 0x1aaad8, life `(b+0x81)/2 +
Rand((b+0x81)/2)`, 3× velocity over the `Vec_MakeOrthonormalBasis` 0x1e4b78 basis sum, `Rand_FRandHalf_Vec`
spin; mesh `EffectInfo+0x74`, obj+0xe8 scale, obj+0x106 alpha = 0xff). `Effect_Bullet` 0x1ce8a8: the
`Rand_Rand(2)` gas gate (skipped for class-3 shooters [M 0x1ce94c]), the three per-material `Gas_Init`
puffs (wall-offset puffs skipped on |normal.y| > 0.3 [M 0x1ce990]), then the decal: when
`EffectInfo[type&0x3f]+4 != 0`, a node from the 200-entry FIFO (`DList_Init(0x3496e0, 0x60, 200)`,
`Effect_InitLists` 0x1cf268; full pool evicts the OLDEST [M 0x1cea20]), `Mat_Align2Dir(node+0x20,
normal, (0,1,0) @0x2d6210, (1,0,0) @0x2d6220)` 0x1df2a8, `pos = hit + normal·0.01` [M 0x1ce9f0],
`hashtable_set_sub_object_to_entity_gfx(node, EffectInfo+4)` 0x1ce9c8. The node rides the host's
sub-object list (static hits the cel, dynamic the obj — matrix made object-local via
`Mat_Inverse(obj+0x90)` [M 0x1ce8a8:166-186]; the port keeps only the world-space result — no marks on
moving hosts yet). The ricochet: weapon flag +0x10, bounces < 3, `Effect_RicochetProb` 0x1cf290's
`Rand_Rand(N) & 8`, reflect by the raw-bits restitution at EffectInfo+0x1c [M 0x12d704..0x12d7b8].

Casings: `Bullet_InitCasingLists` 0x12fd48 pre-creates 32 class-0x40 objects (ShellCasings 0x26f498,
round-robin NextCasing 0x30ca92), parked `View_SetDrawInNoViews`. `Bullet_init_casing` 0x12fde0 arms
a slot (delay weapon_data+0x78, mesh `ammo_data[weapon_data+0x90]+4` via
`hashtable_set_object_to_entity_gfx` → `Control_SetGList` 0x134960). `Bullet_init_casing_ex` 0x12f678
ejects: velocity = the bone transform of weapon_data+0xf8/−0xfc/+0x100 + jitter, euler at obj+0x50
from `vecutil_cartesian_to_spherical_acc` of the eject dir (obj+0x58 = 0), RNG order per eject [M
0x12f860..0x12f8f0]: 3× `Rand_FRand_MVar2(0.1, 0.05)` spin, 3× `(0.01, 0.005)` velocity jitter, 1×
`Rand_FRand(0.5)` landing plane (shooter y − 1..1.5). `Bullet_casing_update` 0x12faa0: pos += vel,
vel += WldGravity·REC², euler += spin, landing sound by material [M 0x12fb48..0x12fbcc], despawn at
age > 240 ticks. `Debris_Update` 0x1aac00: life−1, pos += vel, vel += gravity·REC², euler += spin,
alpha byte −1/tick; dead at life ≤ 0.

## The draw (no dedicated function exists)

There is no `Effect_Draw` in ACTION.ELF. All three render through the generic pipeline:

- **Decals** are sub-object tree nodes: `View_DrawObjectGlist` 0x1e6780 / `View_DrawCelGlist`
  0x1e6538 draw `node+0x10`'s glist at `node+0x20` via `psiDrawObjectMatrix` 0x124708. No scale —
  the quad size lives in the mesh; the matrix is pure rotation+translation (psiDrawObjectMatrix pins
  the w row to 1.0). The GS blend/z state is baked in each decal glist's packets (level decal glists
  carry their own list-0 blend state); the draw pass is pass 0 (solid), depth-tested.
- **Casings / debris** are ordinary objects: `Control_GetObjMatrix` 0x134210 builds the cached
  `obj+0x90` = `View_RotTransMatrix(euler@+0x50, pos@+0x30)` then `Mat_Scale3f(obj+0xe8)` once when
  ≠ 1 (uniform; z-only when +0xfa & 1); `View_DrawObjectGlist` copies it unscaled for non-billboards
  ([M 0x1e6780:47-49] — the +0xf0 & 8 billboard path and the & 4 vertex-jitter path don't apply to
  casings or debris). Both draw in pass 0. `View_DrawObjList` 0x1e69a0 buckets them by obj+0xf0
  flags; neither carries the 0x1 (alpha list) or 0x2000 (pass 2) bits.

Per-frame order (`Game_Draw` 0x1e6f50): `View_DrawSky(v,1)` → `psiSetUpColourBlend(2)` →
`View_DrawObjList(v,0)` (decals, casings, debris) → `psiPostSolid` → alpha list → shards/drops →
pass 2 (gas) → sprites.

## The port's mapping

`Scene::build` instances every distinct `EffectInfo+4` decal, `EffectInfo+0x74` debris and
`ammo_data+4` casing hash the level archive resolves as Effect-layer pools — 200 decal slots
(the FIFO size), `nf_game::casing::POOL` = 32 casing slots, 128 debris slots (unbounded in the game;
≤ 6 per impact, ≤ ~54 ticks). Each slot is one `Attachment::effect: Some((kind, slot, mesh))`
(kind 0 decal / 1 casing / 2 debris) over an identity-baked instance; `frame_state` poses the slot
per frame from the sim and collapses free slots far below the world:

- decal: `nf_game::searchlight::align2dir(normal, (0,1,0), (1,0,0))` (the same construction as
  `Mat_Align2Dir` 0x1df2a8) at the stored pos — the port's decal mesh keeps the leaf's own GS state
  (blend/atest/zwrite from the disc glist), depth-tested.
- casing: `casing_matrix` — euler = the eject direction's spherical angles [S: reconstructed from the
  pre-gravity velocity] + spin·age, at the integrated pos; scale 1 (the game writes no obj+0xe8 at
  eject); only state 0 draws.
- debris: euler = spin·age at pos, rows scaled by `obj+0xe8` (0.4 + FRand(0.1)); age = 255 − alpha
  (the alpha byte starts 0xff and steps 1/tick [M 0x1aabf8, 0x1aac00] — an exact tick count, no new
  sim field). The game applies no per-particle alpha at draw (the byte is bookkeeping only here).

The old flat-colour `effect_quads` (fix-impacts' placeholder, drawn in the snow pass) is removed —
the snow pass is back to snow + emitter particles only, matching the game's pass order.

## The port's [S] stand-ins (live-check list)

- The casing eject origin/basis: the game reads `AnimGetBoneWorldTrans(obj, 1|5, 0)` (view-model
  bone 1, or 5 when weapon_data+0x68 & 0x40000 and objdata+0x958 is clear); the port uses the muzzle
  + an aim-derived basis. The casing's initial euler comes from the weapon-local eject dir through
  that same matrix; the port eulers the world eject direction (gravity-compensated) — the tumble
  itself (spin·age) is the game's.
- The decal draw pass: the game draws in pass 0 (fully fogged, before the alpha list); the port
  draws the pools in the Effect layer (pass 5, after the fog pass, fog 0). Visually equal for the
  marks (depth-tested, own blend), off for heavy fog at distance.
- The debris scale: single application (0x134210:25-33); `View_DrawObjectGlist` re-scales only the
  & 4 jitter path, which debris don't take. Render check: debris should be roughly half the wall
  puff's size, tumbling.
- `Effect_Create`'s material-0x80 `Rand_Rand(0x3f)` penetration roll and the flags & 0x20 emitter
  arm are not ported (no RNG consumed); `Effect_Body` 0x1cf400 / `Effect_Player` 0x1cec48 /
  `Effect_Laser` 0x1cef38 and the bullet trails (`Bullet_trail_effect_smoke` 0x12cba8 /
  `Bullet_DoTrails` 0x12d890) are not ported.

Measured data [D disc / probe]: 07000005 resolves the decal hashes 0x02000122 (GlassBulletHole),
0x02000126 (DefaultBulletHole), 0x02000127 (MetalBulletHole), 0x02000128 (WoodBulletHole),
0x02000262, 0x020006fd, the debris hashes 0x0200012a-2d ("spark leaves/wood/plaster/glass"),
0x020001fe (spark concrete), 0x020001ff, 0x02000200, 0x02000220, and the casing meshes
0x0200043f/443/444/445/447 — but NOT the default shell mesh 0x02000548 (earlier story archives
carry it; the P2K's ammo 1 → casing 0x0200043f, resolved locally).
