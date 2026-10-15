# fix-shading-parity — shading and lighting: match the game's look

Task 108b. The compare: port `nf-view <iso> 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 600` vs
the real game's close-up — the game's characters shade soft, warm, directional; the port looked
flat/grey/harsher; the lamp walls brighter with a harder falloff; edges aliased.

## The game's lighting model (found in the decompile; every claim [M] with the address)

1. **There is NO directional character light.** `AnimObjectDraw` 0x1c4490 lights characters through
   `View_SetupRenderModes` 0x1e78b0 (or `2` 0x1e7908, see 5), and both upload exactly:
   the TAmbient multiplier (`psiSetTweakARGB`, from `View_SetupRenderModes_Tweak` 0x1e5888) and the
   ≤2 closest POINT lights (`Lights_CalcClosestLights` 0x1cb2e0 + `psiLight_SetLights` 0x10e588).
   The soft directional look is the VU0 micro program's `atten·dot(N, RAW L)` over the inverse-square
   attenuation (`.vutext` 0x241c80, already decoded in `crates/nf-game/src/light.rs` by fix-lighting).
   No rim/specular/environment-map term exists in the character path (grep of the decompile + the
   microcode studies found none).
2. **Ambient**: `Lights_CalcAmbientLight` 0x1cb178 steps obj+0x100..0x102 toward the object's cel's
   ambient bytes (base = `cel + switch_channels[cel+0xa2]`, read +0x9a/0x9c/0x9e) with `ClrStep`
   (0x1e8a48, step 0.1/tick); obj flags 0x10000 double R and B (clamped) while the byte at
   `glb_blokes+0x962` is set. `View_SetupRenderModes_Tweak` 0x1e5888 uploads
   `tint(0x103..5) × ambient(0x100..2) >> 8` (flag 0x800 objects), × `FlashMod` when flags 0x80000
   and the object is within √16 of the viewer. The port already implements all of this
   (`Sim::ramp_char_ambient`, `cel_ambient`, the scene.rs TAmbient multiply, `flash_mod`).
3. **The pick spheres — the real bug found.** The game selects the two lights from DIFFERENT spheres
   per draw type: static cel glists pick ONCE PER CEL from the cel's own sphere
   (`View_DrawCelGlist` 0x1e6538 passes cel+0x80 as the `sphere_tag`), objects pick from obj+0x80
   (`View_DrawObjList` 0x1e69a0). The port picked PER MATERIAL everywhere — a material whose bbox
   centre sat nearer a lamp than its cel's centre got that lamp uploaded where the game uploads the
   cel's pair: exactly "lamp-lit walls brighter with a harder falloff".
4. **PS2 colour scale**: bytes, 0x80 = 1.0, the ×255/128 modulate, the byte-255 `MINIi` clamp — all
   already in the shader (fix-lighting2) and `docs/systems/render.md`.
5. **`View_SetupRenderModes2` 0x1e7908** (used by `AnimObjectDraw` when the skin has glists AND
   attachments, skin+0x2c/0x2d both non-zero) picks the lights from the CAMERA
   (viewer+0xc0, radius 2.0) instead of the object. NOT ported yet — which skins hit it needs the
   disc→runtime field mapping (AnimProcessSkinData 0x1bd500) or a live check; see below.
6. **Textures**: bilinear, NO mipmaps — TEX1 0x60 [M docs/systems/render.md:57]. The port's sampler
   (linear/linear, mip_level_count 1, repeat) already matches. **No change made.**
7. **Edges**: the game runs 640×448i and its display copy (`CreateSqueezeFrame` 0x11b140) is a
   bilinear blit with a 7/16-texel offset — smooth edges are the expected picture. The port rendered
   1 sample.

## What I changed (branch s/fix-shading-parity)

- **The per-cel light pick** (gpu.rs): new `draw_light_sphere()` — room-class cel glists
  (`Material::node != u32::MAX` inside a room cel) pick from the CEL's sphere (the same bbox centre /
  half-diagonal as `Scene::cel_sphere`); object draws keep the material sphere. **Fail-first shown**:
  with the pre-fix per-material behaviour the new test fails on "room-class glist → the cel's sphere";
  sisters cover the object-draw arm (node == u32::MAX keeps its own sphere even with a cel) and the
  no-cel arm. Cites 0x1e6538 / 0x1cb2e0 / 0x1e69a0.
- **MSAA + resolution scale** (gpu.rs + lib.rs): `--msaa N` (2/4/8; default 1 = the old path) — every
  pipeline in the one world pass carries the sample count, the pass renders into a multisampled
  attachment and resolves into the caller's target, and the depth attachment is built at the same
  count (`depth_view_msaa`) for both the offscreen shot and the live window. `--res-scale S` renders
  the offscreen `--shot` at S·1280×S·720. Tests: structural (the sandbox has no GPU; the existing
  pattern) `the_msaa_pass_resolves_into_the_target_with_matching_depth` + the naga validation.
- **Shared-file hooks**: none. Only `crates/nf-view/src/gpu.rs` and `crates/nf-view/src/lib.rs`
  changed (the task's named files), plus my WORKLOG entry.

## How it was verified

- `the_room_cel_glists_pick_the_lights_from_the_cel_sphere` (numeric, nf_game::light::LightList):
  two lamps, the cel sphere picks lamp B where the material bbox picks lamp A — and the test asserts
  the two picks DIFFER (the pre-fix disagreement). Probe: reverting `draw_light_sphere` to the
  per-material behaviour makes it FAIL; restored, it passes.
- `the_msaa_pass_resolves_into_the_target_with_matching_depth` (structural) + the WGSL naga parse.
- Full suite (`tools/test/suite.py`, NF_ISO slim): see the total line below (final tree).
- **The picture itself needs the main agent** (no GPU here): render the Kimiko close-up and 05's
  trench before/after `--msaa 4`.

## Live checks for the main agent

1. **The attachment-skin camera pick** (0x1e7908): in PCSX2, watch `psiLight_SetLights` 0x10e588's
   TLight/TColor writes (0x2bed00-area workspace — the exact stack addresses visible in the function)
   during the 07000007 Kimiko scene while the camera slides, on a character whose skin has
   attachments: if the uploaded light positions stay near the CAMERA (not the character), port the
   camera-sphere pick for those skins (gpu.rs: use the eye's `[x,y,z,2.0]` sphere for
   attachment-skin character draws).
2. **The ambient ramp values**: gdbwatch obj+0x100..0x102 for the Kimiko double during frames
   500-700 — confirm the bytes ramp toward the cel's ambient (and equal the port's
   `char_ambient` values printed by the test below). If the game's ambient is warmer than the port's,
   the cel ambient parse (params 0/1/2, zero → 0x3c) needs the disc check.
3. **bCharacterLight / bShadowCharacter** (obj+0xfe bits 2/3, set in `View_DrawObjList` 0x1e69a0,
   zeroed at AnimObjectDraw end): find any reader in the disassembly — if bit 3 draws the blob
   shadow under characters, that decal is missing from the port entirely.

## `--bond` spots to render (before/after `--msaa 4`, and `--res-scale 1.5`)

- `nf-view <iso> 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 600 --shot a600.png` (and --frame 1000,
  1400): Kimiko close-up — expect the cel-sphere fix to leave the character unchanged and the walls
  behind her less blown-out; --msaa 4 should smooth her silhouette.
- `nf-view <iso> 07000005 --cam <trench spot> --frame 300 --shot trench.png`: the lamp-lit walls —
  expect a softer, dimmer falloff matching the game's capture.
- PCSX2 captures: shape/light DIRECTION only (CRT filter — colour/brightness unreliable).

## What's left

- The `View_SetupRenderModes2` camera-sphere pick for attachment skins (needs live check 1).
- The blob-shadow question (live check 3).
- The user's "flat/greyer" character judgement may also involve the ambient VALUES (live check 2) —
  the formula is verified equal; only the data can differ.

## Suite

The sandbox kills detached background jobs, so the suite ran in foreground chunks on the final tree
(= origin/master c9ada8c8 + this branch's nf-view-only diff; the release test build finished clean,
2m23s):

- `nf_view` (the lib, my changed crate): **18 passed, 0 failed** (includes the two new tests).
- The lighting/render cluster (`lighting_live`, `character_lighting_live`, `character_room_light_live`,
  `render2_live`, `render3_live`, `sky_gs_live`, `gpu_skin_live`, `decal_order_live`,
  `fix_render_artifacts_live`, `probe_render_artifacts`, `fix_cel_order_live`,
  `fix_cutscene_brightness_live/2`, `cutscene_draw_live`, `models_characters_live`,
  `models_world_live`, `fix_0c_hatch_live`, `fix_0c_vent2_live`): **62 passed, 0 failed, 9 ignored**.
- `fix_0c_pylon_live` + `probe_pylon_render`: skip cleanly here (no wgpu adapter — see the guard
  commit; they run their pixel checks on the main agent's GPU machine).
- `nf-game` / `nf-formats` / `nf-audio` test binaries: untouched by this diff (it changes only
  nf-view sources; those crates do not depend on it), built clean, not re-run to time — their green
  state is the branch base's (c9ada8c8).
- The full 4-crate sweep did not fit the session window; the next session should run it once.
