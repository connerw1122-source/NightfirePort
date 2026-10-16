# flesh-hit-effects — the guard hit's blood puff

Playtest 1015-1738: bullet hits on people show "No red puffs".

## What was actually wrong (the evidence chain)

The task brief guessed the spawn was missing and pointed at `Effect_Create` / `Effect_Bullet` /
"the EffectInfo row for flesh". The decompile says otherwise, and the audit settled it:

1. **A character hit carries material 0.** `Collide_Jointy` 0x1ec180:125 and :173 write
   `HITDATA+0x50 = 0` (with the bone id at +0x52) [M]. `Effect_Create` 0x1ce688:63 returns unless
   `material & 0x3f != 0`, so **no `Effect_Bullet` runs on a person** — `Drone_CollisionHandler`
   0x140968 calls no effect function either. There is no flesh row to port: the whole
   `EffectInfo` table [L ACTION.ELF `EffectInfo` @0x3037c0, 23 rows × 0x88, dumped in full] has
   decal/gas/debris entries for stone/metal/glass/wood/water/snow — every row's puffs are grey or
   spark yellow; none is a blood row. (This confirms fix-flesh-impacts' earlier finding; the
   earlier task's modelling was right and was not the gap.)
2. **The blood puff is the victim's own reaction.** `NDrone2_DealWithObjHit` 0x13fff0's default
   arm (every weapon outside {0x2b, 0x34..0x37}, damage > 0) calls
   `Gas_Init(3.0, 6.0, obj+0x20, HITDATA+0x40, 0, 0, 0x20008bc, 5, 0xffffffff, 0xffffff00,
   life 0x3c)` and sets the gas obj's `+0xf0 |= 8` [M — the decomp drops args, so disassembled:
   site 0x14047c → `ori $t0, $t0, 0x8bc` (the mesh), kind 5, the 3.0/6.0 floats, rgba
   0xffffffff → 0xffffff00, life 0x3c; the 0x32/0x33 Phoenix-bolt arm at 0x140414 is
   0x200026e at 1.0/16.0]. The red comes from the 0x0200_08bc mesh's own texture drawn
   white-modulated. fix-flesh-impacts already ports this call into the gas pool (`victim_puff`,
   drone.rs) — I re-verified every constant against the disassembly: **the pool data is correct.**
3. **The real gap was the renderer.** The gas draw-slot list (scene.rs's Effect-pool builder)
   collected only the `EffectInfo` rows' gas meshes plus `SMOKE_QUAD` (0x0200_0039). The blood
   mesh 0x0200_08bc — a hardcoded `Gas_Init` site, not an `EffectInfo` row — got NO slots, so the
   puff sat in the pool with nothing to draw it through. Invisible puff = "no red puffs". The
   same hole covered two sibling hardcoded-site meshes: the projectile trail smoke 0x0200_001f
   [M `Bullet_trail_effect_smoke` 0x12cba8 disasm 0x12cc10-0x12cc74] and the bolt puff
   0x0200_026e.

## The fix (branch s/flesh-hit-effects)

- `crates/nf-game/src/gas.rs`: `pub const POOL_MESHES: &[u32]` — every mesh the pool's own spawn
  sites can carry (SMOKE_QUAD, 0x0200_001f, 0x0200_08bc, 0x0200_026e), documented per site with
  the addresses above. Data next to the spawners, so the next hardcoded `Gas_Init` site extends
  one list.
- `crates/nf-view/src/scene.rs`: the slot builder's `gas_meshes` extends from
  `nf_game::gas::POOL_MESHES` (was: a single `SMOKE_QUAD` push). One line; no shared file
  restructuring.

No sim/RNG changes at all (the puff's spawn was already bit-correct).

## Verification

- `crates/nf-view/tests/fix_flesh_hit_effects_live.rs`, opt-in `NF_ISO`:
  - `a_guard_hit_spawns_the_blood_puff_and_the_mesh_has_draw_slots` — loads 07000005, settles,
    moves the first live guard 5 units ahead, aims at the LIVE torso box centre, fires through the
    real input path (`step_bond(Input { fire: true, … })`), and asserts: the pool gains the
    0x0200_08bc puff with the game's exact data (kind 5, scale 3.0, life 60, rgba 255×4) near the
    guard, the guard's health actually dropped (the shot landed), and the scene now carries gas
    draw slots for the blood mesh AND the bolt puff AND the trail smoke. A slot is created only
    when `level.by_hash` resolves the hash, so the slot assertion doubles as the proof that the
    blood mesh exists in the level archive and will draw.
  - `a_world_hit_spawns_no_blood_puff` — the sister: shots straight into the floor spawn the
    material row's own puffs and never the blood mesh.
  - RED→GREEN: with the fix stashed the guard test fails at exactly "no gas draw slot for the
    blood puff 0x0200_08bc — the pool puff would still be invisible"; with the fix both pass.
- Ran: the new test 2/2 green on the final tree; `cargo check --release` clean for nf-game and
  nf-view (tests included). Full suite (`tools/test/suite.py`, the full env incl. the Vulkan
  loader) on the final tree merged with origin/master — the total line is in the driver's queue
  message; on this sandbox's slim ISO only the 3 `disc_golden` tests may fail.

## What's left

- Nothing blocking. Two [S] observations for a render pass, not defects with evidence:
  - The game's blood puff spawns at `obj+0x20` (the victim's torso origin) with `HITDATA+0x40`
    as the second vector; the port uses the hit point, within ~1 unit of the torso — visually
    equivalent, the main agent's render check owns the look.
  - The gas obj's `+0xf0 |= 8` draw flag (only the blood/bolt arms set it) is not modelled; the
    port's gas pool draws every puff uniformly. If the main agent's render shows the puff at the
    wrong layer/blend, that flag is the first suspect.

## Live checks for the main agent

1. **The render** (this is a visual fix — the pool assertions can't see it): on 07000005, stand
   before a guard and fire a few rounds.
   - `--bond` spot: at Bond's spawn facing the moved guard used by the test (guard placed 5 units
     ahead): expect the red blood puff at the torso on each hit — mesh 0x0200_08bc, growing
     3.0 → 6.0 over 60 ticks, fading white-modulated texture.
   - `--bond` spot: shoot a wall/floor: expect NO red puff (the material row's own grey/spark
     puffs only).
   - Bonus from the same fix: the AR-GL/MGL projectile trails (0x0200_001f) should now draw their
     smoke puffs.
2. **PCSX2 ground truth** (optional): `gdbwatch` on `Gas_Init` 0x1aacc8 during a guard hit —
   expect the call from `NDrone2_DealWithObjHit` (0x14047c) with `$t0 = 0x02008bc` (the mesh) and
   the returned obj's `+0xf0 |= 8`; then a screenshot a few ticks later showing the red puff.
