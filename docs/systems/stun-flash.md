# The stun grenade's flash

What happens when a stun grenade (weapon record 0x35) pops: the flash factor's formula, the
player state it arms, the render shape, the guards it stuns, and what the port records so tests
can assert the flash against the geometry it actually used. Facts are tagged [M] decompile
(`work/decomp/ACTION.ELF/file:line`), [L] measured bytes, [D] disc, [S] suggested.

## The pop

The 0x35 record carries `+0x70 & 0x200` (tick fuse) and `& 0x2000` (the flash branch) [L
ee_start_05.bin: +0x70 = 0x2280], fuse 210 ticks [L; the `Bullet_init` table 0x12c5a0:196-229 —
210 default]. `Bullet_DoTrails` 0x12d890's fuse arm (0x12d890:262-285) decrements `BU+0xFC` per
tick; at expiry the `+0x2000` arm runs [M 0x12df58-0x12e130]:

- `Gas_Init(0.5, 1.68, …, fx 0x200001f, 0xB)` — the white puff;
- `Camera_Shake(2.0)`;
- `Sound_Play3D(0x16)` — the pop;
- `Light_Create(10.0, 5.0, 0xff,0xff,0xff, 0, 10, 1, 0)` — the white light.

No `Explode_Create`, no damage, no 0x6000052 script anywhere on this path — the light and the
puff are the visuals [M: the arm's body has neither call].

## The flash factor

Then a 30-unit sphere from the blast [M 0x12dfb0]: for the player, with `dist` = head → blast,

```
f = clamp01(1 − (dist − 5)·0.011764706)        // 0.011764706 = 1/85: full ≤ 5 u, gone ≥ 90 u
  × (blast at the player's xz ? 1.0
     : clamp01(1 − (|angleDiff| − 0.5235988)·0.81851107))   // full within 30°, zero past ~100°
  × (LOS head → blast clear ? 1.0 : 0.5)
```

- The facing angle is `atan2(blast.x − px, blast.z − pz)` against the player's body-matrix yaw
  `cel+0x54` [M the disasm 0x12e058-0x12e068; the port's stand-in is the crosshair ray's
  bearing — [*suggested*, both are the camera's yaw at the blast]].
- The disasm's zero-arm: a NEGATIVE facing factor zeroes the flash (the running best starts at
  0.0 before the first player) [M 0x12e058-0x12e068].
- The LOS leg is `Collide_LineOfSight` from the head to the blast [M 0x12e0a4 area].

`Player_SetFlashBang(FRAME_RATE·f·10, BL, (u8)(f·255))` [M 0x12e014-0x12e0b8 + 0x196040]: the
strength byte `BL+0x963` = f·255, and `BL+0x918` = `BL+0x91C` = 600·f ticks at 60 Hz (FRAME_RATE
= 60 [D docs/systems/input.md:126]) — the remaining count and the total, fixed at the pop.

## The render

`Player_Update` 0x193600:328-352 [M]: `+0x918` counts down 1.0 per tick; the whiteout draws at
the full `+0x963` alpha while more than half the total remains, then fades with the remaining
fraction; `GlobalBlur` rides the same byte.

## The guards

The broadcast gate [M 0x12e094]: in SP only a player thrower broadcasts. `Drone_MessageObjVicinity`
0x13a438 sends msg 0x18 to every class-2 object within 20.0 (the 30.0 sphere is the pre-filter)
whose head sees the blast (`NDrone2_CanSeeObject` → the AI-net ray + `Collide_LineOfSight`
0x176f18). `HandleImpact` case 0x18 [M 0x1469b8:250-312] → state 0x4D `StunGrenadeImpact` [M
0x172bb0], which arms `DroneFunc_RecoverTime`'s 0x35 duration `18·(200−stats[8])` = 3600 ticks at
the default stats and drops the held weapon (`DroneWeap_DropWeapon` + `ChangeWeapon(1)`) unless he
holds the unarmed 4.

## The release timing (why the pop geometry moves)

The hand grenades' release is NOT the fire script's playhead: `Player_WeaponFiring` 0x1a5460
puts the `+0x6C & 0x400` + `+0xB4` family into view state 0xb at 1.5×, case 0xb waits for the
fire script to END, and case 0xc's release check [M 0x1a2e38:505-519, disasm 0x1a3268-0x1a32c4]
compares the +0xB4 script's playhead (starts 1.0, +1.0/tick [M AnimScriptInit 0x1c1600:37,
AnimScriptTick 0x1c2630]) against `+0x44` — the frag releases at press+71, not press+17 (the
full audit: docs/reports/frag-throw-timing.md). The pop lands 210 ticks after the release, and
with the later release the grenade can still be rolling at the pop — anything reconstructing
the blast from a previously sampled position drifts.

## The port

- `crates/nf-game/src/drone.rs`: `flash_amount(head, blast, px, pz, yaw, has_los)` — the formula
  above with the disasm's own float constants; the flash arm in `Drones::tick`'s grenade event
  loop (the `0x2000` branch) calls it and arms the health state.
- `Drones::flash_probe: Option<FlashProbe>` — the exact inputs the last pop's flash consumed
  (blast, eye, px/pz, yaw, LOS branch, f), recorded at the call site. Port-side instrumentation
  with no game counterpart (the game computes and forgets): tests assert against it instead of a
  reconstruction.
- The facing yaw stand-in is `atan2(aim.x, aim.z)` [*suggested* — see the formula section].
- Tests: `crates/nf-view/tests/fix_grenade_types_live.rs` (the whiteout's time + alpha on the
  recorded geometry, the distance scaling, the guard stun), `frag_throw_timing_live.rs` (the
  release tick), `fix_grenade_release_live.rs` / `fix_grenade_throw_live.rs` /
  `remote_mine_detonate.rs` (the family's release windows).
