# The gas/smoke puffs (`Gas_Init` 0x1aacc8, `gases_smoke` 0x1ab2d0)

The game's particle puff: a shared `smoke_1_dust` quad (mesh hash 0x0200039 unless the caller
passes its own) instanced with a scale ramp, an rgba fade and a spin. Port: `crates/nf-game/src/gas.rs`
(`GasPool`), draw side `nf-view` scene.rs (the billboard slot per puff).

## `Gas_Init(cel, pos, quat, vel, scale1, scale2, mesh, kind, rgba_start, rgba_end, life)`

The ABI the decompile hides: the two scale floats ride f12/f13, the next four integer arguments
a0–a3 (cel, position, quaternion, velocity pointer), the next four **t0–t3** (mesh, kind, rgba
start, rgba end) and the life ushort on the caller's stack ([M prologue 0x1aacc8: `move $s5,$t3`,
`move $s3,$t1`, `move $s2,$t2`, `sw $t0,($sp)`, the life read back `lhu 0xc0($sp)` 0x1aad24]). The
decompiler drops the life at every call site — read the disassembly for it.

Body ([M 0x1aacc8]):
- entry gate: `cel != 0 && life != 0` — a 0-life call creates no object and **draws nothing** (:31).
- obj = `control_create_object(0x40 or 0x60)` (0x60 only for kinds 9/10 — :33-36).
- data+0x34 = (scale2 − scale1) · 1/life — the reciprocal form (:51); data+0x38 = life;
  data+0x3C byte = kind; obj+0xFE = 0xE (the gases_smoke control class).
- rgba: data+0x20..0x2C start from rgba_start's bytes as floats; data+0x10..0x1C rates =
  (end byte − start byte) · 1/life per channel (:56-71). The callers pass the END as an integer
  whose FLOAT bits are ~0 (usually the literal 1), so the fade target is (0,0,0,1).
- **RNG draws** (the shared generator, in order):
  1. spin: `Rand_FRand(REC_FRAME_RATE · 1.5707964)` → data+0x30 (:72, jal 0x1ab060); kinds 0xb
     quarter it (:74-76);
  2. sign: `Rand_Rand(0x100)` — ≥ 0x80 negates the spin (:77-80, jal 0x1ab08c);
  3. velocity branch:
     - vel pointer non-NULL, kind 4: `Rand_FRand(0.05)` ×2 — vel = (given.x·REC + r, given.y·REC,
       given.z·REC + r) (:89-98, jal 0x1ab21c/0x1ab248);
     - vel pointer non-NULL, other kinds: vel = given · REC, **no draws** (:100-103);
     - vel NULL, kind 7: `Env_GetWindVel` (0x17c048 — the cel's wind radiator, 0 with none) then
       `Rand_FRand(0.2)` ×3 — vel = (wind.x + r − 0.1, wind.y + r, wind.z + r − 0.1) · REC
       (:105-116, jal 0x1ab160/0x1ab184/0x1ab1a4);
     - vel NULL, kind 9: the position-copy arm — vel = pos, obj+0xF0 |= 0x10 (LAB_001ab1e4);
     - vel NULL, kind 10: kind 10 first halves the life (:122) then the position-copy arm;
     - vel NULL, any other kind: vel = (0, REC_FRAME_RATE, 0) — the rising default (:118-128).

So a puff costs the shared stream **2 draws** (spin + sign) plus 3 more for kind 7 with a NULL
velocity, or 2 more for kind 4 with a given velocity. The CALLERS add their own draws before the
call (below).

## The ported call sites and their full draw counts

| Site | Caller draws | Gas_Init draws | Total | Port |
|---|---|---|---|---|
| Player muzzle smoke — `Player_MuzzleFlash` 0x1a4298 (the +0x68 & 0x2000 == 0 arm), vel NULL, kind 7, 0.05→0.1, rgba 0x48403080→1, life 0x80 [M disasm 0x1a4404-0x1a442c] | 1 (`Rand_FRand(2π)` 0x1a43ec, the quaternion z) | 2 + 3 wind | **6** | `init_muzzle_smoke` + the scene's 2π draw — real stream (fix-gas-rng) |
| Broken-searchlight smoke — `Searchlight_Update` 0x1cb900:373-385, vel NULL, kind 7, 0.05→0.25, life 0x80 | 1 (`Rand_FRand(2π)` 0x1cbca0) | 2 + 3 wind | **6** | `init_searchlight_smoke` — real stream (fix-searchlight-remainder) |
| Impact puffs — `Effect_Bullet` 0x1ce8a8 gas loop (≤3 entries per surface record), vel NULL | 0 (the owner gate `Rand_Rand(2)` 0x1ce94c is drawn for NPC shooters only — obj+0xff == 3 skips it, the player's impacts always puff) | 2 per entry, +3 for kind-7 entries | 2/5 per entry | `init_effect_puff` — real stream (fix-gas-rng); NPC impacts pay the coin draw first |
| Projectile smoke trail — `Bullet_trail_effect_smoke` 0x12cba8 fast arm, vel = a non-NULL zero vector, kind 7 | 1 (`Rand_FRand(6.25)` 0x12cc18) | 2 (the given-velocity arm, no wind) | **3** | `init_trail_smoke` — real stream |
| Rocket exhaust — `Bullet_DoTrails` 0x12d890 bit 0x20 arm, vel z = the draw, kind 0 | 1 (`Rand_FRand(6.25)` 0x12da2c) | 2 | **3** | `init_exhaust_puff` — real stream |
| Drone fire-script muzzle smoke — `AnimProcessScriptCmds` 0x1c1b38:445-453 (the fire script's flash op), vel NULL, kind 7, same args as the muzzle | 1 (`Rand_FRand(2π)` 0x1c1bfe-ish, the op's own) | 2 + 3 wind | **6** | NOT ported (the op itself is not run for drones yet) |
| Drone flash-sprite jitter — `DroneWeap_FireWeapon` 0x178f68:199-203 | 3 (`Rand_FRand(2π)` + `Rand_FRandHalf(0.2)` ×2, inside the sprite-create branch) | — (no Gas_Init) | 3 | deliberately not consumed — the port creates no sprite object; the branch's `control_create_object` gate is a live check (fix-guard-weapons3, docs/WORKLOG.md) |
| Ripples — `Ripples_Update` 0x1ac028 | the ripple system's own gate + jitter draws, then a `Gas_Init` kind 6 | 2 | site-specific | counter hash (world_fx.rs) — the ripple draws are not re-derived |
| The drone.rs kind-5 puffs (the swoosh stand-in 6227, the white/grey puffs 6361/6407) and the 0xb crate puff 8562 | their game sites are not re-derived (Copter_Update/GT_Update/RB_Update/Effect_Laser territory) | 2 (+branch) | — | counter hash (`init`/`init_ex`) until each site is re-based |

## `gases_smoke` 0x1ab2d0 (the update)

Per tick, by kind: 0 = vel damped ×0.95 + slight gravity, pos += vel; 4 = pos += vel +
gravity·0.01, vel damped; 7 = no own velocity, gravity·0.05 buoyancy drift; 9/10 = hidden
emitters (the game keeps them invisible); the tail fades rgba toward the end bytes, decrements
the life and kills the object at ≤ 0 (obj+0xFE |= 1). Port: `GasPool::update`.

## Determinism notes

- The draws ride the one shared generator (`Rng`, docs/WORKLOG.md 0025) — never add draws the
  game doesn't make, never skip ones it does. The pool-cap check sits AFTER the draws for the
  real-stream sites (the game's RNG has no notion of the port's cap).
- The fade rates use the game's reciprocal form ((end−start) · 1/life) — bit-exact against the
  EE's `div.s`/`mul.s` sequence via `nf_formats::ps2float`.
- The muzzle and searchlight puffs share every fixed arg except the scale end (0.1 vs 0.25) —
  the scale_rate field separates the two sites' puffs in tests.
