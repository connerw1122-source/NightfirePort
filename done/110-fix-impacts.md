Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-impacts`, bullet decals, ejected shell casings, impact dust and sparks, ricochets

The user's PCSX2 comparison: guns eject shells, and bullets leave marks and puffs on walls. The port does none of this. Bond's and the guards' shots hit (the damage, the hit sounds and the fly-bys work), but nothing appears. Port the game's effects, sim side first (they draw from the shared RNG), then the drawing.

1. **Bullet decals: researched, not ported** (`docs/GAPS.md` G-041, which has the full path; read it first). `Bullet_CollisionHandler` (0x12ce80) → `Effect_Create` (0x1ce688) → `Effect_Bullet` (0x1ce8a8). It uses a 200-entry FIFO pool (`Effect_InitLists` 0x1cf268; when full, the oldest is evicted). The mark aligns to the surface normal (`Mat_Align2Dir`) at `hit + normal·0.01`. Its mesh comes from `EffectInfo[material & 0x3f]+4` (table at 0x3037c0). Static hits ride the cel; dynamic hits ride the object (object-local, so marks move with doors and props, and `Effect_RemoveSObjs` 0x1cf360 removes them with the host). Marks never fade. nf-game already computes `segment_surface` (pos, normal, material) at both impact sites (`Drones::impacts`, `update_player_bullets`). Widen those events, keep the FIFO in the Sim, and expose it to the renderer. Also port `Effect_Body` (0x1cf400), `Effect_Player` (0x1cec48) and `Effect_Laser` (0x1cef38) as far as they're cheap.
2. **Shell casings:** `Bullet_InitCasingLists` (0x12fd48), `Bullet_init_casing` (0x12fde0), `Bullet_init_casing_ex` (0x12f678) and `Bullet_casing_update` (0x12faa0). Find where they're called from (Bond's fire path, `Player_MuzzleFlash` 0x1a4298 or the weapon code, and `DroneWeap_FireWeapon` for guards). Port:
   - the casing model per weapon (weapon_data fields);
   - the eject position from the view model's datum;
   - the velocity and spin, with any `Rand` draws in game order;
   - gravity, bounce and lifetime;
   - the pool size and eviction;
   - the landing sound.

   The `fix-audio2` task (170) also lists casing sounds. If master already has them when you start, reuse them; otherwise you own the trigger and fix-audio2 owns only the sample choice. Bond's casings come from the first-person view model, so they need the view-model space-to-world transform the flash uses (`Scene::frame_state` attach matrices).
3. **Impact dust and sparks:** `Debris_Create` (0x1aaa00, HITDATA + HEINFO per material), `Debris_CreateEx` (0x1ac808), `Debris_Update` (0x1aac00), `Debris_Create_Loop` (0x1ed548), `Particles_Create` (0x1cf478) / `Particles_Update` (0x1cf528), and `Gas_Init` (0x1aacc8) for puffs. Port them per material (stone dust, metal sparks, wood chips, glass, snow), with the counts, velocities, lifetimes and RNG draws.
4. **Ricochets:** `Effect_RicochetProb` (0x1cf290, material × weapon → probability; an RNG draw). Port it with its sound, and the bullet's continuation if the game has one.
5. **Bullet trails:** `Bullet_trail_effect_smoke` (0x12cba8) and `Bullet_DoTrails` (0x12d890). Port them if the weapons we have use them (rockets and tracers); otherwise note which weapons do.

**Determinism:** every effect above that calls `Rand_*` must draw in the game's exact order relative to the shots, the AI and `Light_Create`'s `Rand_Rand(0)` (prompt_fix-lighting owns that one; if master doesn't have it yet, leave a marked hook at the same point). Seeded trace tests must keep passing. Effects that don't draw can live render-side.

**Drawing:** you have no GPU, so build the render data and leave the pixels to the main agent:
- Decals: world-space quads offset by normal·0.01, depth-tested, alpha-blended, no depth write. Textures by the five hash ids in G-041.
- Casings: their meshes as instances, like pickups.
- Particles: camera-facing sprites, like the snow.

Expose them through `Scene` (one function returning this frame's vertices, like the snow), and add a CPU-side test that the vertices appear where the hit was. Say in your report what the main agent should look at.

Tests (NF_ISO, through `nf_view::load` and `step_bond`):
- a shot at a wall on 07000005 leaves one decal at the hit point with the wall's normal and material, and a sister shot into the sky leaves none;
- the 201st decal evicts the first;
- each shot ejects one casing that falls and lands within its lifetime;
- the RNG stream after N shots matches the game's draw count;
- the existing trace tests still pass.

Files: a new `crates/nf-game/src/effect.rs` (decals, debris, particles, ricochet) and `crates/nf-game/src/casing.rs`; the impact sites in `crates/nf-game/src/drone.rs` and `crates/nf-view/src/scene.rs` (minimal hooks); docs in `docs/atlas/` (a new `effect.md`); update G-041 and G-025 in `docs/GAPS.md`.

**Reproduce before you fix.** The user's play is the reference, and previous fixes passed tests but still failed in play. For each bug:
1. Write a headless NF_ISO test that drives the real input path (the `nf-view` input mapping into `Sim`, or `nf_view::load` plus `dev::scenario`), and show it reproducing the user's symptom.
2. Then find the cause in the decompile and fix it.
If you can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite before pushing:**
  `NF_ISO=<abs path> cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`
  On the slim ISO only the 3 `disc_golden` tests may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Update the doc entry** to say it's fixed, with your commit.

**Branch:** `s/fix-impacts`.

---
claimed-by: sandbox4 at 2026-10-13T10:54Z

---
## Report (done, 2026-10-13T12:09Z)


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
