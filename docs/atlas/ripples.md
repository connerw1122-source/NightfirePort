# The ripples (placement type 0x21, class 0x34) — water rings

`Ripples_Create` (0x1acc90, 188 B) / `Ripples_Update` (0x1ac028, 300 B). Ported in
`crates/nf-game/src/world_fx.rs` (fix-world-fx) — the rings ride the existing `GasPool`
(crates/nf-game/src/gas.rs), the game's own `Gas_Init` path [M 0x1ac028:44-52].

## Create (0x1acc90)

`control_create_object(4, …)`; data+0 = param 0 (s16 spread), data+2 = param 1 (s16 period);
both 0 → spread 1, period 0x1e [M 0x1acc90:20-30]. class 0x34, `obj+0xf0 = glist+0x44` flags,
LinkToRoom. On 07000001/02/04/0b (docs/systems/levels.md §7.1).

## Update (0x1ac028) — per tick

- Gate: `Rand_Rand(period / FRAME_RATE_MUL) == 0` → spawn [M 0x1ac028:28-31].
- Spawn: two `Rand_FRand(2·spread) − spread` draws jitter x/z around obj+0x30..0x38; one
  `Rand_FRand(0.05) + 0.05` sets the start scale; `Gas_Init(s, 2s, cel, pos, …, mesh 0x02000129,
  …, rgba 0x7f7f7fff, …)` [M 0x1ac028:34-52] — a white ring on the water mesh 0x02000129 (the
  0x0200xxxx mesh namespace, gas.rs's SMOKE_QUAD is 0x02000039).
- The port: the gate and the three draws come from the per-object counter hash (the shared stream
  must not gain draws — the gas.rs discipline [S]); the puff's life is taken as the period [S] (the
  decomp drops the life argument), so a ring survives until roughly the next one spawns. Kind 6 is
  not in gas.rs's update table → the default arm (grow + fade in place) — the visible behaviour of
  an expanding ring [S].

Render: the pool's puffs draw as billboarded quads already (the muzzle-smoke path); the ring mesh
0x02000129 is not bound — the quad carries the puff's rgba — a listed follow-up.
