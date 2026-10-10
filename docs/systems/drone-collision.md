# Drone collision and gravity

Sources: ACTION.ELF decomp and disassembly (research pass, WORKLOG 0078). [M] = read in the code or bytes, [L] =
live snapshot, [S] = inferred. Port: `crates/nf-game/src/drone.rs::collision_handler`.

The on-foot engine's per-tick collision driver is `Drone_CollisionHandler` (0x140968), called from the control-list
pass after the state machine. The port ran a floor-probe stand-in (`floor_below` within 2.0 / 0.6 of the feet) inside
`move_by` / `slide_to`; this ports the game's own path so a guard dying at a ledge edge falls (WORKLOG 0041's gap).

## Fields [M]

| Drone offset | port field | meaning |
|---|---|---|
| `+0x480` | `Drone::velocity` (V3) | per-tick velocity; gravity accumulates here, `pos += velocity · REC_FRAME_RATE` |
| `+0x484` | — (cleared each tick) | the y-velocity damping scratch |
| `+0x4F8 & 0x20` | `Drone::bounded` (collision on) | gates the whole handler |
| `+0x4F8 & 0x80` | (gravity on) | gates gravity accumulation |
| `+0x4F8 & 0x4000` | (in-air) | cleared at the handler's top; set when the down-ray misses |
| `+0xA0` | hip height | the feet-sphere offset; default 1.0, `max(0.2, hip − 0.02)` for drawing |
| `obj+0x30` | `Drone::pos` | the object's world position (feet + 1.0 on the ground) |

## Constants [M]

| constant | value | source |
|---|---|---|
| `WldGravity` (VECTOR at 0x2d8890) | `(0, −9.8, 0)` | [L] docs/systems/pickups.md, player.md |
| `REC_FRAME_RATE` (0x30d0cc) | `1/60` | the per-tick velocity/position scale |
| Feet-sphere radius (default) | `0.4` | `NDrone2_Collision` line 91 |
| Feet-sphere radius (state `+0xD8 == 0xC`) | `0.55` | line 92–93 |
| Feet-sphere radius (crouch mode `+0xC5 == 0x17`) | `0.3` | line 124: `0x3e99999a` |
| Down-ray start offset | `pos + up · −(hip + 0.6)` | `Drone_FeetOnPoint` `fStack_c0 = -(*(float*)(+0xa0) + 0.6)` |
| On-surface threshold | `0.2` below the feet | `Drone_FeetOnPoint` `fVar10 < 0.2` |
| Ray mask | `0xC` (walkable + solid) | `Collide_RayIntersect` last arg |
| Velocity damping (`& 0x40`) | `× 0.98` (`0x3f7ae148`) | `Drone_CollisionHandler` line 140 |

## The handler (`Drone_CollisionHandler` 0x140968) [M]

```
clear +0x4F8 & 0x4000
if collision & gravity both off (state 0xC5 == 0x1D, or NPCGlobals / Drone_bDisableSystem):
    return  (the +0x464 scratch cleared)
else:
    NDrone2_DoCollision(dcv)          # predicate: alive, & 0x20, not in death/blend, drawn, not near player
    if DoCollision:
        Drone_FeetOnPoint              # down-ray; if hit within 0.2 → on-surface (+0x60 & 8), copy hit
        for each hit in the list: NDrone2_DealWithObjHit   # water, doors, breakables, explosions, players
        # the nearest walkable hit sets the on-surface flag; a hit above the head sets the "crush" flag
        NDrone2_DoGravity(dcv)         # predicate: & 0x80, same alive/blend/draw gates
        if DoGravity:
            if not on-surface (& 8 clear):  velocity += WldGravity · REC_FRAME_RATE
            pos += velocity · REC_FRAME_RATE
            obj+0xFA |= 0x20            # moved this tick
    else:
        NDrone2_DoGravity(dcv)
        if DoGravity:
            if not on-surface:          velocity += WldGravity · REC_FRAME_RATE
            pos += velocity · REC_FRAME_RATE
            obj+0xFA |= 0x20
    if & 0x40 (damping):  velocity *= 0.98
```

`Drone_FeetOnPoint` (0x140660) casts `Collide_RayIntersect(pos, pos + up·−(hip+0.6), mask 0xC)`. If the hit's
`hit.y` is within 0.2 below the feet (`pos.y − hip − hit.y < 0.2`), the drone is on the surface: `anim+0x60 & 8` is
set and the hit is copied into `anim+0x00` (the surface the renderer and footsteps read). A hit belonging to the
player inherits its velocity (`Control_InheritVelocity`). The hit list's nearest walkable entry is the floor.

`NDrone2_DoCollision` (0x149ec8) is the predicate: returns 1 when collision should run — alive (`& 0x200` clear or
level 0x700004a), `& 0x20` (collision on), the state-machine id `+0xD8 ≠ 0xC/0x13` (not dead), no `+0x538` blend,
`NearPlayerCollision` false, `obj+0xFC` drawn, `+0x570 & 2`. `NDrone2_DoGravity` (0x141e58) is the same with `& 0x80`.

`DroneFunc_NearPlayerCollision` (0x14b190) returns 1 if any of the four `+0x154` per-player distances is `< 2.0` —
the game skips the full sphere test when a guard is right on top of Bond (the AI-boundary push-out stands in).

## Port [M]

`Drone::collision_handler(d)` runs at the end of the per-drone update (after the state machine, before the
AI-boundary push-out). It mirrors the game: `feet_on_point` (the down-ray), the on-surface flag, gravity accumulation
into `velocity` when off the surface, `pos += velocity · REC_FRAME_RATE`. `move_by` / `slide_to` no longer probe the
floor themselves — the handler puts the drone back on the ground.

The `free_root` path (SpecialDeath) bypasses the handler entirely: the root motion moves the object in 3D and the
gravity/collision flags (`& 0x20 / 0x80`, `bounded`) are off (WORKLOG 0041).

## Unknowns

`+0x464` (cleared each tick, read by `DroneAnim_PreTransCallback` for the draw height), the per-material table at
`0x3037dc` (`Drone_FeetOnPoint`'s `bVar2` extension path reads the surface's `+0x88` coefficient — the same table
`Bullet_CollisionHandler` uses for bounce; not ported, the walkable/solid mask 0xC is enough for the floor).
