# The helicopter (`Copter_Create` 0x17d340)

The helicopter actor of the single-player levels: a script-assembled multi-part Hind (`SP_Create`
0x1dc7e0 inside `Copter_Create`, nine parts registered by `Copter_Register` 0x17e530) that flies a
level-supplied spline framelist (`Copter_Move` 0x17d628), spots Bond inside a 45° cone with line of
sight (`Copter_Update` 0x17dbb8 case 1), attacks with a gun turret and rocket pods (cases 2 +
`Copter_FireGun` 0x17d828 / `Copter_FireMissile` 0x17d9f0), takes damage per part through the
static `Bits` table [L ELF 0x2a37e8], and is destroyed at `299 × DiffMod` damage — the destruction
writes the record's +0x72 channel, which on 07000008 is the level-end channel 0x4B.

## 1. Where the game's data comes from

A `0x2F` placement (`parsemap_create_dynamic_objects` 0x1d04f0 case 0x2f) calls
`Copter_Create(pos, rot, record)` — the disasm 0x1d0b08-0x1d0b14 (`a2 = s1` = the placement record)
— and `Copter_Create` reads a **level tag** the block walker builds on its stack
(`parsemap_block_map_data_dynamic` 0x1d1b68: the tag is memset 0xb0, the placement's params land at
`tag+0x2c + 4·k` — the store loop 0x1d1bf0-0x1d1c10, `sw v1,0(a3 + key*4)` with `a3 = sp+0x2c`), so
**`Copter_Create`'s `param_3+0x30+4k` reads = the placement's parameter k**:

| record field | placement source | meaning [M 0x17d340] |
|---|---|---|
| `tag+0x02` (node+0x10 count) | u16 @ record+0x40 (`extra[7]` low) | the framelist's key count |
| `tag+0x04` | u16 @ record+0x42 (`extra[7]` high) | the dt divisor **fallback** |
| `tag+0x08` (node+0x08) | u32 @ record+0x08 → the tag's copy | **the framelist pointer** — `table[u16 @ record+0x44]` = `extra[8]` low |
| `tag+0x2c` (node+0x18) | param 0 low | the node type (2 = the death/exit segment) |
| `tag+0x30` (node+0x12) | param 1 low | the route id (nodes with the same id form one route) |
| `tag+0x34` (node+0x16) | param 2 low | the node's index within the route |
| `tag+0x38` (node+0x14 cap) | param 3 | the wrap-back frame (`min(count, param 3)`) |
| `tag+0x3c` / `tag+0x40` | params 4 / 5 | the SP scripts (parts assembly / flight) |
| `tag+0x44..+0x50` | params 6..9 | +0x70 gate, +0x72 out, +0x74 countdown gate, +0x76 countdown |
| `tag+0x54` | param 10 | the dt divisor (0 → the fallback above) |

The framelist table is the level bin's `0x19` blocks: `parsemap_block_path_data` 0x1d24d0 stores
each block's payload pointer (`FileNextBlock + 4`) into the table; the index is the block's
**per-map-file ordinal** (07000008's #45 reads index 3 = the fourth 0x19 block of its map entry —
the 8606-key flight starting exactly at the placement's position [D probe
`probe_helicopter2.rs`]; a global ordinal would read a 3-key gun track). Every measured placement's
`extra[7]` low equals its framelist's key count [D probe]:

| level | placement | route id | framelist | keys (= count) | divisor (= ticks) | loop start (param 3) | gate | out |
|---|---|---|---|---|---|---|---|---|
| 07000005 (Mayhew A) | map50 #241 | 1 | 4 | 382 | 2000 | 0 | 0 (flies at load) | 75 (0x4B) |
| 07000005 (Mayhew A) | map50 #242 | 3 | 5 | 273 | 1000 | 0 | 66 (0x42) | 75 (0x4B) |
| 07000008 (gondola) | map51 #45 | 1 | 3 | 8670 (keys 8606 — the tail clamps) | 17000 | 684 | 5 | 75 (0x4B) |
| 0700000b (Tower C) | map50 #328 | 1 | 18 | 984 | 3500 | 0 | 42 (0x2A) | 102 (0x66) |
| 07000012 (Tower2 B) | map38 #707 | 1 | 23 | 673 | 3700 | 0 | 37 (0x25) | 101 (0x65) |

The per-tick advance: `node+0x0c = count · REC_FRAME_RATE · (FRAME_RATE / divisor)` with
`REC_FRAME_RATE = 1/FRAME_RATE` [M GS_SetRefreshRate 0x17d2b8] = **count/divisor**, so the route
takes exactly `divisor` ticks and then wraps to the loop start (`t = node+0x14` — the single-node
search always re-selects itself [M 0x17d628's end-of-path arm]; a node whose +0x74 channel number
is 100 — 0700000b's — sets `obj+0xFE |= 1` at each wrap [M 0x17d740]).

The remaining 0x2F placements are the **part templates** the dynamic placements reference by hash
(all-zero params, count 0 — `Copter_Move` no-ops a zero-count path [M 0x17d66c]): 07000005 map7/map9
(the gun Hind: Rotor, TailRotor, Engine, glass, Pilot, Gun, MuzzleFlash, MissileLeft/Right, Beamhind
+ `Damaged_Hind` #11), 07000008 map9 (the same set), 0700000b map22 + map21, 07000012 map3 (Rotor,
TailRotor, glass, Gun_A, MuzzleFlash_A, RocketPod, NoCollision), 07000046 map36 (the MP map's
pad copter). Static `0x19`-kind helicopter props (heli pads, the 07000029 heliyard) are scenery,
not the actor.

## 2. The actor's machine [M `Copter_Update` 0x17dbb8]

- **Gate**: a non-zero +0x70 channel that is off returns at once — the copter does nothing until
  the gate opens; then the +0x76 countdown (when non-zero) pulses `switch_channels[+0x74]` at 0.
  The gate is a ONE-SHOT rising-edge start [M 0x17dbb8 head :99-105]: on the first tick the
  channel reads on, the head calls `SP_HideObj(record+0x2c, 0)` (unhide) and **zeroes record+0x70**
  — afterwards the actor behaves as if it never had a gate, even if the channel goes off again.
  (Port note: the port's `Copter::gate` field keeps the channel number and `update`/`visible`
  re-check the channel every frame — equivalent while the channel stays on; a channel that
  drops after the start would freeze/hide the port's copter where the game's would not. The
  zero-on-start is not ported yet — flagged in the helicopter-2 report.)
- **State 0** (once): counts the same-id route nodes into +0x78 (type-2 nodes stored to +0x58, the
  death segment). → 1.
- **State 1** (patrol): `Copter_Move` advances the spline; the whole spot check — the obj+0xF6
  timer increment, the every-`240/DiffMod` period test (`FRAME_RATE_INT << 2`), the cone check
  (the player within `1e6` square units, inside the `COPT_COS/COPT_SIN` cone [M 0x17e468 static
  ctor: `PS2Sinf(3π/4)`, `PS2Sinf(π/4)` — both 0.70710678, a 45° half-angle], `Collide_LineOfSight`
  clear), the state-2 flip, the `attackmode_159 = Rand_Rand(10)` draw and the recent-damage
  accumulator (+0x6c) reset — sits **inside the conjunct `record+0x74 != 100`** [M 0x17dbb8:140,
  the `Rand_Rand(10)` at :184 inside it]. A copter whose +0x74 channel number is 100 — the
  ambient patrols (07000005's #241/#242, 0700000b's #328 [L the level params]) — never increments
  the timer, never spots, never draws from the shared RNG, never fires, never plays 0x5df and
  never requests combat music: it flies its loop forever. (This is the helicopter-2 merge-hold
  fix: the pre-fix port spotted the courtyard player at t160 and attacked for the rest of the
  window, which shifted the guards' sight-to-first-shot stream
  (`guard_reaction_live` path 3) and interrupted the weapon-sound sweep
  (`fix_weapon_sounds_live` 0x3a/0x41).)
- **State 2** (attack): `Music_Event(2, 1)` (the combat music) every tick; `Copter_Move`; obj+0xF6++
  and per tick one of:
  - `attackmode & 1 == 0` → **`Copter_FireMissile`**: the pod pick `Rand_Rand(0x86e6) & 1`
    (+0x50 left / +0x4c right) draws on EVERY call, BEFORE the `60·DiffMod` launch gate
    (`GameState time % (60·DiffMod) == 0`) [M 0x17d9f0:47 vs :58-64] — off-gate ticks still
    spend one RNG draw; only the launch is gated. On a launch tick the bullet
    = `weapon_data[0x6d]` (`0x62` on 07000008) from the pod's muzzle (`pod+0x80` sphere, the
    `pod->cel+0x38 · 1.5` offset along the aim dir); on 07000008 a `Rand_Rand(8/DiffMod) == 0`
    roll aims straight at Bond instead of the pod's matrix dir.
  - else **`Copter_FireGun`**: both turrets (+0x44/+0x48) `Mat_LookAt` Bond's eye (+0.5 y); the
    +0x44 turret's shot counter (+0xF4) increments and at `> 4` fires `weapon_data[0x71]`
    (`0x61` on 07000008) from `turret_pos + dir · (turret->cel+0x38)`, resets the counter, plays
    `Sound_Play3D(100.0, muzzle, -1.0, -1.0, 0x5df, 0, 0, 0)` [M 0x17d828:85] and readies the
    +0x48 turret's muzzle flash (its +0xF4 = 2, the +0xF0 bit 0x10 cleared — the flash the update's
    head ticks down).
  - The attack tick fires BEFORE the bail checks [M 0x17dbb8:191-198 → :199-242]: even the tick
    that loses the sight has already fired. Back to 1 (timer 0, +0x6c reset) when the player is
    gone, or the recent damage exceeds 100, or `5·60·DiffMod < obj+0xF6` — the timeout is STRICT
    and a keep tick does NOT reset the counter [M :200], so a stint ends mid-sight at
    `5·60·DiffMod`.
- **Damage** (every tick, LAB_0017e180): per part i (0..8) the part's `obj+0xD0` HITDATA chain sums
  `record.damage · Bits[i].mult` into the total (+0x64) and the recent (+0x6c). `Bits` [L ELF
  0x2a37e8, the static .data table — only `Copter_Update` references it]: mult
  `[2.0, 3.0, 1.0, 2.0, 100.0, 1.0, 0, 0, 0]` with Bits[4] filter 16 (the rest 0); part 2 (the
  body, +0x38 = `Copter_GetBody` 0x17e4b8's return) takes ×1, part 4 (filter 16 — the explosive
  category) takes ×100, parts 6-8 take nothing.
- **Destroyed** (`1 < state-4` unsigned — states 0-3 — and `+0x64 ≥ +0x68`): state **6** on
  07000012/0700000b — `Explode_Create(10, 10, 67.4, obj, pos, 0x2d6210, 0x6000052, 0xff, 0xff, 100,
  0)` — else state **4**: the next `Copter_Move` completes, the path switches to the death segment
  (+0x58), `t = 0`, and **`switch_channels[+0x72] = 1`**; state 5 writes +0x72 again and sets
  `obj+0xFE |= 1` (done — the record freezes at its last matrix). The +0x72 channel is the
  destruction output: on 07000008 it is **0x4B, the level-end channel** ("Escape with Zoe in
  gondola" — the Hind must be shot down), on 0700000b 0x66, on 07000012 0x65.

## 3. The helicopters' roles (all five)

- **07000005 Mayhew A** — two copters over the castle approach: #241 (route 1, gate 0) flies its
  382-key route from load — the ambient patrol; #242 (route 3, gate 0x42) starts with the
  `SWITCH_Helicopterstart(SC3)` #269 chain (param 0 = 66) and flies its 273-key route; both write
  0x4B on destruction. Neither attacks on paper — but the machine is the attack Hind (the map9
  template carries Gun/MuzzleFlash/Missile parts), gated only by the cone check.
- **07000008 gondola** — #45, the extraction Hind: gate ch 5 (the ride's end cinematic
  0x06000078 writes it [D fix-objects2]), flies the 8606-key route looping back to key 684,
  attacks Bond (the cone check, the 0x08 gun/missile weapon ids), and its destruction writes
  0x4B — the mission's completion [M Copter_Update cases 4/5/6; the level doc's mux #276 → ch 100].
- **0700000b Tower C** — #328 hovers on the roof deck (y ≈ 58 [D drone-props]) with the parachute
  pickup #280 riding it; gate 0x2A, destruction (or the level's design) writes 0x66; its +0x74 = 100
  sets the wrap flag each route lap.
- **07000012 Tower2 B** — #707, the attack Hind over the lower offices: gate 0x25, out 0x65, the
  RocketPod/Gun_A/MuzzleFlash_A parts; on this level the destruction takes the case-6
  `Explode_Create` arm.

## 4. The port

`crates/nf-game/src/copter.rs` holds the actor; `Drones` owns `copters: Vec<Copter>` (create-time,
main map's 0x2F placements) and ticks them in the update walk. The channel machine (gate,
countdown, out) was fix-objects2's; this task adds the real spline flight (the framelist keys from
the level bin's 0x19 blocks through `nf_formats`'s parse, `door::interp`'s ported
`Spline_Interp`/`KeyFrame_Interp`), the combat machine, the Bits damage, and the destruction.

Getters for the draw code (the main agent renders; nothing here draws):

- `Copter::pos()` — the body's world position (matrix row 3; the game's `obj+0xC0`,
  what `Copter_Move`'s `SP_SetPosRot` drives).
- `Copter::matrix()` — the full spline pose (`obj+0x90`).
- `Copter::mesh_hash` — the placement's object hash (0x20003cc/0x200035d/0x20008c7/0x200062a),
  the draw's mesh lookup key.
- `Copter::visible()` — the gate/show state (`Copter_GetBody` 0x17e4b8's gate check).
- `Copter::rotor_phase` — **not a copter field**: the game's rotors are the flight script's keyed
  part animation (`Copter_Update` holds no rotor state [M — the whole function reads]); the
  spinning parts ride the port's script player exactly as the game's SP parts do. The viewer's
  rotor draw comes from the script entity's keyed matrix, not from this record.

[*deviation*] the port's copter bullets reuse `Drones.bullets` with `owner = usize::MAX` (the
game's `Bullet_init(0xffffffff, ...)` non-player slot [M 0x17d828:90]); the smoke-puff arm of the
damage scan (`Gas_Init` at >50% damage, 0x17e2xx) is not ported (visual, gas system out of scope
this task).

## 5. Tests

`crates/nf-view/tests/helicopter_live.rs` (ISO-gated):

- 07000005 #241 flies its route from load: the body's position follows the framelist spline
  (asserted against the ported `Spline_Interp` at the measured t; sister: the parked pre-tick
  position).
- 07000012 #707: the gate 0x25 → the cone spot → the gun fires at Bond (`weapon_data[0x71]`
  bullets + the 0x5df sound); sister: outside the cone, no fire.
- 0700000b #328: gate 0x2A shows it; the game's damage (a player bullet into a part, the Bits
  multiplier) destroys it → the case-6 `Explode_Create` arm + ch 0x66; sister: below the
  threshold, no channel.
- 07000008 #45 (the fix-objects2 gondola test, corrected): ch 0x4B is the **destruction** output
  [M Copter_Update cases 4/5/6 — the only +0x72 writers], not the flight script's end; the test
  drives the destruction through the game's damage.
