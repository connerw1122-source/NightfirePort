# Switch channels, triggers and spawners

Sources: ACTION.ELF decomp and disassembly (research pass, WORKLOG 0027), placements of 07000005, live RAM reads. [M] =
read in the code, bytes or RAM, [S] = inferred. Port: `crates/nf-game/src/triggers.rs`.

## Switch channels [M]

`switch_channels` u8[256] @0x26FC90 (also `_prev` @0x26FD90, `_hold` @0x26FE90 — snapshots taken after `Trigger_Calc`;
`_time` u32[256] @0x270190 — frame of the last set). `Init_SwitchChannels` (0x134310) zeroes everything at level
start; no channel defaults to on.

## Frame order [M]

`control_movement_object_handler` (0x1336F8): `Drone_InitComms` → `Doors_Calc` → `Trigger_Calc` (0x1B1900) → snapshot
prev/hold → every dynamic object's update in list order. `control_add_object_to_list` inserts at the head, so objects
update **newest first** = reverse placement order (runtime-spawned drones go to the front). Triggers (object type
0x1D) update in `Trigger_Update` (0x1B1A78), spawners (0x30) in `DroneSpawner_Control`.

`Trigger_Calc`: Bond moments (channels 0x65–0x6E); hard-coded latches — 07000005: `ch[0xA4]` on → `ch[0x6F] = 1`
(0x0700000B: 0x34 → 0x2D); clears the output channel of every Touch trigger, then runs those Touch triggers.

## Trigger objects [M]

| Placement | Trigger type | Behaviour per update |
|---|---|---|
| 0xEA TouchOnce | 1 | `ch[out] \|= hit`; once its channel is on at the start of its update (glist flags 0x10) it deletes itself |
| 0xEB Touch | 2 | `ch[out] = hit` (OR over the Touch triggers of a channel; cleared in `Trigger_Calc`) |
| 0xEC MultiplexIn | 3 | `ch[out] = AND(inputs)` every update (not latched) |
| 0xED MultiplexSIn | 4 | ordered AND, each input on within 4 frames of the previous |
| 0xEE MultiplexOut | 5 | copies `out` to up to 8 channels |
| 0xEF MultiplexOrIn | 0xC | `ch[out] = OR(inputs)` every update |
| 0xE8 LoadLevel | 10 | level change (p0 level, p1 unlock channel, p2 auto channel) |
| 0xDC Trigger | raw | type = p0 |

Parameters: TouchOnce out = p0, mask = p1, filter list = p2..p8, **lock = p7**, hide = p8; Touch out = p0, mask = p1,
list p2..p8, **lock = p6**, hide = p7; muxes out = p0, inputs p1..p6 (+ p8), mask / lock / hide = p9 / p10 / p11. A
trigger whose lock channel is on is inert. Channel 0x7D Touch triggers only work on level 0x07001500.

`Trigger_Hit` (0x1B1FC8): contact bits from the touching objects — 8 player, 0x10 drone, 2 object type 5. Mask 8 → the
player touches; mask 0x10 → a drone; mask 0x18 → player or drone with the filter list (0xB needs a drone, 0xE the
player, other values a drone skin-hash match in the low 16 bits).

**Volume**: the trigger model's collision triangles at the placement transform (no box/sphere). Contacts come from
the collision system: the player's capsule against the triangles. Player capsule (normal stance, `Player_Collision`
0x194DF0): from feet + 0.55 up to about feet + 1.86 (hip height 1.035 read live), radius 0.55. Muxes have glist flags
0x30 (no collision).

## Spawners (`DroneSpawner_Control` 0x137E68) [M]

Parameters: p0 mode (2: spawn while alive < max; 3: only when none alive), p1 group (drone parameter 4), p2 max alive
(0 or more than the templates → all), p3 waves (0 → 32000), p4 start channel, p5 stop channel, p6 done channel, p7 kill
channel, p8 minimum player distance (0 → 10). Init: the group's drones become templates and are removed; with p4 = 0 (or
on) and p5 off the spawner is "immediate" (no distance / visibility checks). Per frame: kill channel → kill flag (latch);
inactive until `ch[p4]` (then p4 = 0); `ch[p5]` → stop (deleted unless killing); kill → delete every live spawned drone
and the spawner; past the last wave wait for none alive, set `ch[p6]`, delete. At most one spawn per frame:
`Rand_Rand(templates)` picks the k-th unused template; unless immediate its spot must be ≥ p8 from the player and not
visible to him (`Drone_PositionVisible`).

## Drones and channels [M]

- Parameter 3 → `+0x134` wait channel (WaitSwitch until on; checked on msg 3, so a one-frame pulse is enough).
- Parameter 6 → `+0x135` second switch, with parameter 8 → `+0x13A` (0x64 for every drone here):
  `NDrone2_PreDroneControl` (0x148DB0, every frame): `ch[+0x135]` on and `+0x13A` ≠ 0 → (0xC: delete) else clear both,
  switch to the alternate type (`+0xC6`) and `SetState(+0x5A4)` (the alternate type's state, e.g. 0x56 Attack).
  `FirstAttack` clears `+0x135`.
- Parameter 12 → `+0x13C` death channel (`DroneFunc_SetDeathChannel` 0x13E660: `ch[p12] = 1` on death).

## 07000005 [M]

| Placement | Type | Out | Inputs / mask | Lock | Shape |
|---|---|---|---|---|---|
| 21 / 22 / 23 | TouchOnce | 0x23 / 0x24 / 0x25 | player | – | boxes by the gate (x 57–64, z 79–113) |
| 24 | TouchOnce | 0x21 | player | – | box (1.3–3.1, 33–38, 52.7–57.4) |
| 25 | Mux OR | **0x22** | 0x23 \| 0x24 \| 0x25 | | |
| 305 | TouchOnce | **0x22** | player | – | valley terrain patch (−2.9…15.4, 57.5…73.3) |
| 306 | TouchOnce | 0x44 | player | – | the same patch |
| 304 | Mux OR | 0xB3 | 0x44 | | |
| 301 | TouchOnce | **0x43** | player | 0xB3 | wall-hug plate (x 23.9–25.9, z 70.8–72.2, y 31–33.4) |
| 57/60, 56/59, 54/58 | Touch | 0x54/0x55, 0x56/0x57, 0x58/0x59 | inner: player or drone; ledge: player | | window plates x ≈ −32.1 / −35.2 |
| 63 / 62 / 61 | Mux AND | 0xA1 / 0xA2 / 0xA3 | the pairs above | | |
| 270 | Mux OR | **0xA4** | 0xA1 \| 0xA2 \| 0xA3 | | |
| 315 | TouchOnce | **0xA4** | player or drone | – | plate at (−23.15, 37.23, −27.11) |
| 311–314 | TouchOnce | 0x7F, 0x80, 0x81, 0x85 | player | 0xA4 / 0x6F | ledge plates |
| 80 | Mux AND | 0x69 (Bond moment) | 0x7F, 0x80, 0x81, 0x85 | | |

Effect (reverse placement update order): 0x22 is latched at the end of each frame once any of the gate boxes was
touched (mux 25 runs last), and pulses within a frame while Bond stands on the valley patch; 0xA4 = (an inner window
plate touched by Bond or a guard and its ledge plate by Bond) or plate 315 touched; 0x43 latches when Bond creeps
along the wall plate unless 0xB3 (valley patch, or a gate guard's death channel 0x44) is already on.

Drones: 3 / 4 gate guards (wait 0x22, group 2 of spawner 268, death 0x44); 7 / 267 (group 1 of spawner 266); 65 / 68
(wait 0x22, second switch 0xA4, death 0xA4); 66 / 67 / 271 / 272 (wait 0xA4); 302 / 303 (wait 0x43). Spawner 266:
immediate, spawns one of its two templates at level start. Spawner 268: start 0x22, kill 0x43, distance 4, max 2.

Unresolved: in a live snapshot spawner 268 had started while 0x22–0x25 and 0x44 were 0 (pulse semantics fit; 0x44
never latching does not — a floor contact winning the sort, or a reloaded session) (U-032).
