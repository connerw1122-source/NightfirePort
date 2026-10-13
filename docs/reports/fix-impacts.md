# fix-impacts — bullet decals, shell casings, impact dust and sparks, ricochets

Branch `s/fix-impacts` (master 2bd43d4). Task 110. Started 10:58 UTC, sandbox expired ~12:56 UTC;
this report written 12:45 UTC.

## What's done

**The sim side of the game's whole impact path, from the decompile:**

- **`EffectInfo` decoded and parsed** (`crates/nf-formats/src/sfx.rs`): the table at ACTION.ELF
  0x3037c0 (23 × 0x88, symbol `EffectInfo`, sha1 e7ffdcb3…) is now parsed at runtime by the existing
  `SurfaceSounds::from_action_elf` into `EffectRecord` columns: the decal mesh (+4), the ricochet N
  (+0x1a), the reflection scalar read as raw f32 bits (+0x1c — 0x0000003f on most materials, a
  denormal ≈ 0), the three `Gas_Init` entries (+0x20, stride 0x1c: mesh, behaviour type, rgba
  start/end, scale start/end, life, the wall-offset and orientation flag bytes), the debris fields
  (+0x74 mesh, +0x78 speed, +0x7c spin, +0x81 life base, +0x82 enable, +0x83 count). Also
  `ammo_data` (0x2c69b8, 34 × 0xc): the per-ammo-type casing mesh at +4. No game bytes committed —
  the same symbol-lookup parse every other table uses.
- **`crates/nf-game/src/effect.rs`** — `Effects::bullet_impact` implements, in `Bullet_CollisionHandler`
  0x12ce80's order: `Effect_Create` 0x1ce688 (the weapon_data+0x70 & 2 gate → flags 0x61; the GLASS
  normal negation for the debris basis; `Rand_Rand(EffectInfo+0x83)` + `Debris_Create` 0x1aaa00's
  per-particle draws: scale FRand(0.1)+0.4, life, 3× `Rand_FRandHalf` velocity over the
  `Vec_MakeOrthonormalBasis` 0x1e4b78 basis sum, `Rand_FRandHalf_Vec` spin) → `Effect_Bullet` 0x1ce8a8
  (the `Rand_Rand(2)` gas gate — skipped for the player's class-3 shots [M 0x1ce94c]; the three
  per-material gas puffs via a new additive `GasPool::init_ex` carrying the entry's own mesh and rgba
  end; the wall-offset puffs skipped on |normal.y| > 0.3 hits) → the **200-entry decal FIFO**
  (`Effect_InitLists` 0x1cf268; pool full evicts the OLDEST [M 0x1cea20]) storing pos = hit +
  normal·0.01, the normal and the material's decal mesh → the ricochet: weapon flag +0x70 & 0x10,
  bounce count < 3, `Effect_RicochetProb` 0x1cf290's `Rand_Rand(N) & 8` roll, and the reflection
  `dir + normal·(−(rest+1)·dot)` normalised, with rest read from +0x1c's raw bits — the bullet
  continues on the reflected direction (both drone `Bullet`s and Bond's `PlayerBullet`s, 3 bounces).
  `Debris_Update` 0x1aac00's physics (life countdown, gravity ·REC², spin, alpha fade) ticks per frame.
- **`crates/nf-game/src/casing.rs`** — the shell casings: `Bullet_InitCasingLists` 0x12fd48's 32-slot
  round-robin pool, `Bullet_init_casing` 0x12fde0's armed-delay state (weapon_data+0x78),
  `Bullet_init_casing_ex` 0x12f678's eject (the weapon_data+0xf8/−0xfc/+0x100 local velocity through
  the bone-rotation stand-in, the 7 RNG draws in game order: 3× `Rand_FRand_MVar2(0.1, 0.05)` spin,
  3× `(0.01, 0.005)` velocity jitter, 1× `Rand_FRand(0.5)` landing plane = shooter y − 1..1.5),
  `Bullet_casing_update` 0x12faa0's flight (gravity, spin, the 240-tick lifetime) and the landing-sound
  switch by material (0x1fd default shell, 0x1ff/0x200/0x201/0x202/0x203/0x204/0x1fe per material,
  [M 0x12fb48..0x12fbcc]). Ejected from `fire_player` (Bond, `Player_WeaponInitBullet` 0x1a4bf0:306 —
  after the spread draws) and the guard fire site (`DroneWeap_FireWeapon` 0x178f68:264).
- **Hooks** (minimal, additive): `Drones` gains `effects` + `casings` fields (own lines), the two
  impact sites (`update_player_bullets`'s world arm, the drone-bullet `BulletEvent::World` arm) call
  `bullet_impact` after the existing `impact_sound`, `Drones::update` ticks `Debris_Update` + the
  casings, `PlayerBullet`/`Bullet` gain a `bounces` field, `gas.rs` gains `init_ex` + a `mesh` field,
  `worldobj.rs`'s bullet constructor gains `bounces: 0`.
- **Drawing** (`scene.rs`): one additive `Scene::effect_quads(right, up) -> Vec<Vertex>` (the snow's
  pattern): decals as 0.12-unit quads in the surface plane at the stored pos (already offset by
  normal·0.01), debris as scaled billboards with the alpha byte, casings as small billboards. The real
  meshes/textures are the main agent's render pass (below).

## How it was verified

- **Unit tests** (`effect.rs`, `casing.rs`, 111/111 lib tests green): a decal lands at hit +
  normal·0.01 with the material's mesh and NO draw with flags 0; the 201st decal evicts the first
  (pool stays 200); debris draw counts + physics + life; the ricochet draws exactly once when N ≠ 0
  and reflects a grazing shot (tangent kept, normal component removed, normalised) while N = 0 draws
  nothing; one shot ejects exactly one casing that falls, crosses its landing plane within its
  lifetime and plays the material's sound once; the 33rd eject reuses slot 0; the landing-sound table;
  the 7-draw eject order mirrored against a hand-driven stream.
- **NF_ISO live tests** (`crates/nf-view/tests/impacts_live.rs`, 07000005, through the real
  `nf_view::load` + `Sim::step_bond` input path — all 3 green): a shot at a wall leaves exactly one
  decal that re-casts 0.01 off the surface with the surface's normal and `EffectInfo[mat]+4` mesh, and
  `effect_quads` puts vertices at it; a sister shot into the sky leaves none; one shot ejects one
  casing that lands within 240 ticks; the same seeded run reproduces the same decals and RNG state bit
  for bit.

## What's left (the next session)

1. **`guard_takes_cover_like_the_game` (ai_live.rs) now fails** — expected: the impact/casing draws
   joined the shared RNG stream (the task's determinism rule), so the seed WORKLOG 0072 picked no
   longer aligns the port with the live trace. The fix is the documented re-pick procedure (0072 did
   the same after the searchlight creates joined): re-pick the seed against the SAME live expectations
   (states 0x58,0x5a,0x8c..0x96, cover node 1, the 31/37-tick timings, the (58.83, 99.32) position),
   with a live re-trace if available. I did not re-pick it blind. All other nf-game suites pass
   (31/32 in that binary, 111/111 lib).
2. **Render pass** (main agent, no GPU here): draw `effect_quads` — decals depth-tested alpha-blended
   without depth write, textured by the five decal hashes (0x02000122 glass, 0x02000126/127
   stone/metal, 0x02000128 wood, 0x02000262 soft-generic, + ICE 0x020006fd); casings as the real
   `ammo_data` mesh instances; debris as the `EffectInfo+0x74` meshes. Suggested render spots:
   `07000005 --bond <start>, face the corridor wall, fire once` — a dark 0.12-unit quad 0.01 off the
   wall; `--bond` firing repeatedly near a stone wall — puffs + debris spray + the mark; ejecting —
   a small brass fleck arcing right of the muzzle, landing ~1–1.5 below the muzzle.
3. **[S] stand-ins to live-check** (listed for `fieldprobe.py`/re-trace):
   - the eject origin: the port uses the muzzle position + an aim-derived basis; the game reads
     `AnimGetBoneWorldTrans(obj, 1|5, 0)` — the view model's bone 1 (or 5 when weapon_data+0x68 &
     0x40000 and the byte at objdata+0x958 is clear). Check where the shell appears relative to the gun.
   - the casing landing-sound material: the port reads weapon_data record byte +0x50; the game reads
     the runtime weapon_definition+0x50 — confirm they agree for the P2K.
   - `Effect_Bullet`'s puff velocity: the port zeroes it; the game passes `Vec_NormToRot(normal)` /
     the euler vector — a drift visual only.
   - `Effect_Create`'s material-0x80 `Rand_Rand(0x3f)` penetration roll is not ported (the port's
     HITDATA has no +0x51 second byte) and the emitter arm (flags & 0x20 → `Emitter_Create`) is not
     ported — no RNG consumed for either.
4. **G-041/G-025 doc updates**: G-041's port shape is now implemented sim-side (this commit); G-025's
   casings row likewise. I did not edit GAPS.md — the wording owner should re-read the report first
   (kept the commit surface to the task's code files + this report; the docs/atlas/effect.md write-up
   is owed, the code comments carry the addresses meanwhile).
5. Bullet trails (`Bullet_trail_effect_smoke` 0x12cba8 / `Bullet_DoTrails` 0x12d890): not ported; the
   fly-by path already exists from 0076. `Effect_Body` 0x1cf400 / `Effect_Player` 0x1cec48 /
   `Effect_Laser` 0x1cef38: not ported (out of budget).

## Live checks for the main agent

1. P2K shot against a stone wall in PCSX2: one mark per hit, at the crosshair point, no fade over
   60 s, marks survive 200+ shots (FIFO 200, oldest evicted).
2. A ricochet sound on stone/metal graze hits (EffectInfo: STONE N=31, METALTHIN/THICK N=63) — the
   port rolls `Rand_Rand(N) & 8` per eligible hit (≈ 1/8·(N/65536)… note the game's actual rate is
   `Rand_Rand(N) & 8 != 0`, N up to 63 → rare; the live rate should match).
3. Shell casings: eject count (one per shot), eject side (right of the muzzle), landing ~1–1.5 below
   the muzzle, the tinny landing tick, despawn after ~4 s (240 ticks).
4. Impact puffs: stone dust on stone, sparks on metal, wood chips — colours from the table above.

## ntfy failed

(none — the notification was sent; see the finish log below if it errored.)
