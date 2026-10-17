# Breakables — the glass panes and smashable scenery (placement 0x20)

The game's breakable objects: 443 placements on the disc [D census, `probe_breakables`], types
0 (×171), 1 (×13) and 2 (×259 — the glass window panes). The 07000008 control room carries three
`Glass_Controlroom*` panes (kind 2, health 0, no filter), 07000006 one `Window_Securityroom`
(kind 2, health 0, filter param 0xf8) [D placements]; 07000005 has none [D census]. The port:
`crates/nf-game/src/breakable.rs` (`Breakables`), created per level in `Drones::attach_level`
alongside `crate::props`.

## Object model

`parsemap_create_dynamic_objects` 0x1d04f0 case 0x20 → `Break_Create` 0x12c230: a class-0x20
object (`obj+0xFF = 0x20` [M 0x12c230:17]) with the instance at `obj+0xE0`:

- `+0x00` kind — placement param 0 [M 0x12c230:21]; the parsemap case reads the same word for
  `psiCreateBreakableData`'s kind-2 gate [M 0x1d04f0:70-73];
- `+0x04` health — placement param 1's raw bits as a float [M 0x12c230:22-28]; the disc's panes
  carry 0 [D], so one bullet kills;
- `+0x08` bullet reject mask — `~(u16)(placement param 2)`, 0 kept as 0 [M 0x12c230:29-35].

Kind-2 panes also get their shatter data built at load (`psiCreateBreakableData` 0x117418 walks
the draw model's GIF stream into a tri list — the shard geometry `ShatterPolyRecurse` 0x12b270
splits at kill time; `psiFinishedObjectShatter` 0x11ca88 is an empty stub [M]).

## Damage and death

The collision pass mirrors every hit into the victim's `obj+0xD0` list with the hitter at `+0x58`
[M `Collide_Update` 0x1ea398, docs/fields/hitdata-spawner-ally.md §1]. `Break_Update` 0x12bfd8
filters that list with `Collide_FilterBullets(list, filter | 0x106)` [M 0x12bfd8:15] — which drops
every class-5 (a bullet, `Bullet_init` 0x12c5a0:87) hitter whose `weapon_definition+0x10` word
ANDs the mask [M 0x1ece30:24-33; disc words: guns 0x0008, Laser 0x0002, watch 0x0100 [L dump]] —
and subtracts each survivor's `HITDATA+8` damage from the health [M 0x12bfd8:36-37;
`Break_ApplyDamage` 0x12c128]. At health < 0 `Break_Kill` 0x12b970 runs: `obj+0xFE |= 1` (the
delete flag — the pane leaves the world and registers nothing further, the "collision goes away")
[M 0x12b970:60], the break sound — kind 2 always 0x2e (46); every other kind rolls
`Rand_Rand(100) < 0x29` → 0x5f2 (1522) else 0x5f3 (1523) [M 0x12b970:65-76; the roll is the only
RNG draw, taken only when a non-glass pane dies] — then the kind-2 shatter polys (`psiGetTriList`
0x11c9d8 + `ShatterPolyRecurse`) or the debris loop (`Drone_BrokenObject` 0x138588 is guard barks
on 07000001 only; `Debris_Create_Loop` 0x1ed548 spawns one debris object per collision triangle,
EffectInfo[hit material] supplies the gfx — the hit's material defaults to 0x10 when unset [M
0x12b970:134-141]).

## Bullets never stop on a pane

The panes' collision triangles all carry surface byte 0x10 (glass) [D census of the
`Glass_Controlroom*` / `Window_Securityroom` meshes] and their glist flags (0x201 / 0x1201) have
neither 0x40 nor 0x60 [D]. `Bullet_CollisionHandler` 0x12ce80's glass rule — material 0x10 pierces
unless `weapon_data+0x6c & 0x800` [M disasm 0x12cf44..0x12cf64, ported for world hits by
fix-rocket-glass-08] — therefore passes the round through an INTACT pane while the mirrored hit
still damages it. A shot shatters the pane and flies on; a second shot through the same spot
breaks nothing (the pane is gone). Non-breakable glass (the world-geometry material-0x10 surfaces,
e.g. the 08 corridor window) is not a 0x20 object: it never breaks [M + D].

## Port shape

- `Breakables::create(level)` — placement order over the main map, bounds per `crate::props`'s
  glist-bounds rule ([*port choice*] the game contacts the placed glist's collision triangles;
  the port checks the bullet's flight segment against the transformed bounds box grown 0.1).
- `Breakables::bullet_hit(a, b, weapon_mask, damage, rng)` — the per-tick contact, called from
  `Drones::update_player_bullets` next to `props.bullet_hit`; returns the `BreakEvent`s (position,
  kind, sound) and plays the sound through `Sounds::play3d` (`Sound_Play3D(100.0, −1.0, −1.0, id,
  obj+0x80, custom=0)` [M 0x12b970:69,75]).
- `Breakables::broken_at(placement)` / the public `list` — the state the draw layer reads to skip
  a broken pane's instance (`scene_instances` appends the 0x20 placements in the same order as
  `Breakables::list`).

## Not ported

- The kind-2 shard render stream and the debris (`ShardList` 0x316dd8, `UpdateAllShards`
  0x12b808 / `DrawAllShards` 0x12bef8, `Debris_Create_Loop` 0x1ed548) — presentation; the draw
  layer can build on `BreakEvent` / `broken_at`.
- The guards-smash-glass arm: `Break_Update`'s class-2 branch damages a kind-2 pane by
  `SqDist(drone+0x30, drone+0x40) × 20` when level 0x7000001's guards brush it [M 0x12bfd8:23-34].
- `Break_DoBreak` 0x12c168 — the script event 0x0a (NIS "break at the current entity", one
  occurrence on the disc, script 0600005b of 07000001 [D script census]): finds the first
  class-0x20 object inside the point's own sphere and kills it [M 0x12c168:39-61].
- `Bullet_CollisionHandler`'s class-0x28 arm (sensor volumes pass only with
  `weapon_data+0x70 & 0x100` [M 0x12ce80:129-134]) — the sensor-volumes task's ground.
