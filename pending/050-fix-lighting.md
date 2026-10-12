Read `docs/playtests/2026-10-11.md` first: it's the user's multi-level playtest, and the item ids below refer to it.
10. **Push your report with your branch.** Write the plain-language report (the same text you give the user) to `docs/reports/<your task id>.md` and commit it as the last commit on your branch. The main agent reads it from there, so the user doesn't have to paste it. Never put credentials or URLs in it.
11. **Write everything in English:** code, comments, docs, the WORKLOG, commit messages and your report to the user.

## Your task: `fix-lighting`, dynamic lights on characters, muzzle-flash light and smoke

The user's PCSX2 comparison: lights visibly fall on and reflect off characters, and firing lights the surroundings. The port only applies the flat cel ambient (`obj+0x100`, fix-render). The main agent traced the game's path in the decompile:

1. **Per-object light selection:** `Lights_CalcClosestLights` (0x1cb2e0) walks `LightList` and keeps up to 30 lights per object sphere, sorted by distance. Kinds: light+0x47 = 0 (only for objects with `flags & 0x80`), 1 (skipped for `flags & 0x100`), 2 (2D distance ≤ 4.0, intensity `(1 − d·0.25)·50`), else 3D distance ≤ sphere r + light radius (+0x3C), with the falloff `d / (radius·0.5)` stored at +0x38. Callers: `View_DrawObjList` (0x1e69a0), `View_DrawCelGlist` (0x1e6538), `View_SetupRenderModes` (0x1e78b0), then `psiLight_SetLights` (0x10e588) uploads them to the VU1 light matrix.
2. **Lights in the level:** `Light_Create` (0x1cc3d8), `Light_Init`, `Light_Update` (0x1cafb8), `Light_SetAmbientRadiators` (0x1caeb8), the map's light placements and block 0x26.
3. **Muzzle-flash light:** `Player_MuzzleFlash` (0x1a4298) calls `Light_Create(radius = weapon_data+0x58, 2.0, the muzzle position, r = +0x56, g = +0x55, b = +0x54, …)` on every shot, a short-lived coloured light at the muzzle, and `Gas_Init(…, 0x2000039, …)` for a smoke puff, unless `+0x68 & 0x2000`. Guards' `DroneWeap_FireWeapon` likely does the same; check it.
4. **Vertex normals: DONE by the main agent** (commit after 6faffb4). Skinned vertices now decode their normal from the high 16 bits of skinning words 0..2 (a signed unit normal × 64; all 20,458 skinned vertices on 07000005 come out unit length, `gpu_skin_live::skinned_normals_are_unit_length`), stored in `Vertex::normal` at the world-normal scale. What's left here: carry the normals through the GPU skinning path. Old note, for reference: skinned meshes don't store plain normals. `ps2gfx.rs` notes "normals come from SkinIt at run time", packed in the skinning qword. Decode them (the VU1 SkinIt microcode's unpack), and carry the normals through the GPU skinning palette (`Scene::skin_layout`, the vertex shader in `gpu.rs`), rotating them by the bone matrices. World meshes have normals already (`Vertex::normal`).
**More from the main agent's decompile reading:**
- `psiLight_SetLights` (0x10e588) uploads **at most 2 point lights** per object: `TLight[i]` = the light position (+0x10..+0x18, w = 1); `TColor[i]` = rgb (+0x44/+0x45/+0x46) × intensity (+0x40) × the caller's tint params; and 1/radius² (+0x3C squared). The closest two are chosen from `Lights_CalcClosestLights`' sorted list.
- `Light_Create` (0x1cc3d8) fills a 0x50-byte record (rgb +0x44..0x46, active +0x48 = 1, radius +0x3C, intensity +0x40, kind +0x47, timers +0x20..+0x2C), and **calls `Rand_Rand(param_11)`**, a draw from the game's shared RNG. Port that draw in the game's order (the muzzle light fires on every shot), or seeded tests drift.
- The per-vertex shading itself runs in the **VU1 microcode** (not decompiled by Ghidra). Disassemble the VU1 lighting routine to get the exact formula (N·L, the falloff using 1/r², the clamp, the ambient add). Don't guess it. If you can't get the microcode, implement the data path (light selection, upload, normals) and leave the formula marked STANDIN with the reason.

**The user's priority:** the room lighting up around Bond when he fires (the muzzle-flash light), more than the flash sprite, which looks right. Note that `View_DrawCelGlist` (0x1e6538) also calls `Lights_CalcClosestLights` and `psiLight_SetLights(1.0, 1.0, 1.0, …)` for world cels, so dynamic lights (the muzzle light included) light **room geometry** too, not only characters. Port the muzzle light for both, as the first deliverable of this task. World vertices already carry normals (`Vertex::normal`).

**Muzzle light, render side: DONE by the main agent** (the commit "muzzle-flash light"). `Scene::dyn_light` returns the light for the frame of Bond's shot (`Sim::shot_tick`, set in `step_bond`; it lasts one tick, as `Light_Update` deletes a life-1 light): the view model's datum 0 as the position, 1/r² from weapon_data+0x58, and the colour from +0x56/+0x55/+0x54 × 2.0/255. `Vertex::nrm` (snorm8) carries normals: world space for World vertices, object/bone space for posed ones, which the shader rotates with the palette. The shader (`gpu.rs`, uniform `lp`/`lc`) adds `lc × 0.25 × N·L × (1 − d²/r²)²` and leaves zero-normal vertices (sprites, snow, radar) unlit. The 0.25 and the squared falloff are **STANDIN, tuned by eye**. The test is `crates/nf-view/tests/lighting_live.rs`. **Still yours:** replace the STANDIN with the VU1 formula (read the microcode); `Light_Create`'s `Rand_Rand(0)` draw and `Gas_Init`'s `Rand_FRand` in the game's order inside the sim (the light itself is render-only now); guards' muzzle lights; level and character point lights with `Lights_CalcClosestLights`' 2-light selection; the smoke puff. Keep `lighting_live` passing.

5. **Shading:** port the VU1 lighting equation (ambient + Σ light colour · max(0, N·L) · falloff), as the matrix `psiLight_SetLights` builds, for characters, view models and dynamic objects. Keep world cels on their baked vertex colours (the game's white tweak).

Tests: a light's selection and falloff values against the decompile; and a render-independent check that a character next to a muzzle-flash light gets brighter that frame, with a sister assertion that without the light it doesn't. The main agent checks the pictures.

Files: `crates/nf-formats/src/ps2gfx.rs` (normals), a new `crates/nf-game/src/light.rs`, `crates/nf-view/src/{scene,gpu}.rs` and the shader.

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

**Branch:** `s/fix-lighting`.
