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

   `prompt_fix-audio2` also lists casing sounds. If master already has them when you start, reuse them; otherwise you own the trigger and fix-audio2 owns only the sample choice. Bond's casings come from the first-person view model, so they need the view-model space-to-world transform the flash uses (`Scene::frame_state` attach matrices).
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
