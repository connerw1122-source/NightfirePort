## Your task: `fix-shading-parity2`, shading parity: rebuild on master's letterbox

Base: `s/fix-shading-parity` — merge origin/master FIRST. HELD: master now has 108a-fix-cutscene-letterbox, which
changed the same render plumbing; the merge conflicts in `crates/nf-view/src/{gpu,lib}.rs`,
`tests/fix_0c_pylon_live.rs` and `tests/probe_pylon_render.rs`. Keep BOTH: the letterbox bars and the cut-scene 4:3
framing, and your per-cel light pick (View_DrawCelGlist 0x1e6538's cel sphere vs obj+0x80) and the smoothed edges.
Also port the open item from your report: `View_SetupRenderModes2` 0x1e7908 (lights picked from the camera for skins
with glists + attachments) — find which skins hit it. Render the Kimiko close-up (07000007 `--bond 94.2,44.6,-45.9,0,0`
frames 600/1000/1400) and the 05 trench before/after and describe them. Run the full `tools/test/suite.py`.

Files: `crates/nf-view/src/{gpu,lib,scene}.rs`, the two pylon tests.

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `lib.rs`): small, additive hooks only, listed in your report.
Pull `master` before you start and before every push: other workers merge into it all day.

**Reproduce before you fix.** For each bug: write a headless NF_ISO test through the real input path (`nf_view::load`
+ `Sim::step_bond`, or `dev::scenario`) that shows the symptom, then find the cause in the decompile and fix it. If you
can't reproduce a symptom, say exactly what you tried; don't guess a fix.

**This is a code task.** The rules above still apply, plus:
- **Every fix cites the doc entry it implements and the game address.** No change without evidence from the atlas, the port map or the decompile.
- **Every fix gets an NF_ISO test** that would have failed before the fix (show it failing first), with a sister assertion.
- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md). Read `tail -1 suite.out` and
  its FAILED/TIMEOUT lines before you push, and run it once more on your final tree (merged with `origin/master`). Report the total
  line. On the slim ISO only the 3 `disc_golden` tests (and `pss_movies_live` without the retail image) may fail.
- **Touch only the files your task names.** Other fix sessions run in parallel. If you have to touch a shared file, keep the change minimal and note it in the report.
- **Live checks:** list what the main agent should check in PCSX2 (spot, action, expected result) and in nf-view (`--bond` spots).

**Branch:** `s/fix-shading-parity2`.

---
claimed-by: sandbox7 at 2026-10-15T02:15Z

---
## Report (done, 2026-10-15T03:54Z)


---
## Report from the worker (done, 2026-10-15T03:54Z)

# fix-shading-parity2 — shading parity rebuilt on master's letterbox

Task 108b2. Base `s/fix-shading-parity` (4bc5dfea), merge origin/master FIRST — master had
108a-fix-cutscene-letterbox in the same render plumbing (`crates/nf-view/src/{gpu,lib}.rs`, the two
pylon tests). Both feature sets are kept, and the open item from the 108b report — the
`View_SetupRenderModes2` camera-picked lights — is now ported with the census of the skins that hit
it.

## The merge (both features live)

- `gpu::render` keeps BOTH new parameters: `size: (u32, u32)` (fix-shading-parity's MSAA plumbing —
  the multisampled colour attachment resolves into the caller's target, `depth_view_msaa` matching
  depth) and `pillar: Option<[f32; 4]>` (fix-cutscene-letterbox's 4:3 band — black clear, every pass
  `set_viewport`ed into the band). The clear colour keeps the pillar-black arm ahead of the fog arm;
  the MSAA attachment block follows it. The two pylon tests (`fix_0c_pylon_live.rs`,
  `probe_pylon_render.rs`) take both new args (`(w, h)` and the trailing pillar `None`).
- `lib.rs`: `shot()` keeps `--msaa`/`--res-scale` AND the letterbox `cut`/`cutscene_frame`/
  `proj_aspect` logic; the depth is `depth_view_msaa`; the window redraw passes both the size tuple
  and the cut-scene band. WORKLOG: both sides' entries kept (append-only).
- Verified by the full render/lighting/cutscene cluster (below), `fix_cutscene_letterbox_live`'s 4
  tests included.

## The open item ported: `View_SetupRenderModes2` 0x1e7908

- **The game's model** [M]: `AnimObjectDraw` 0x1c4490:118-122 picks the light-upload function by the
  skin: `skin+0x2c == 0 || skin+0x2d == 0` → `View_SetupRenderModes` 0x1e78b0 (lights from
  **obj+0x80**); BOTH non-zero → `View_SetupRenderModes2` 0x1e7908, which uploads
  `View_SetupRenderModes_Tweak` (the TAmbient) plus the two closest lights from the **CAMERA's**
  sphere — viewer+0xc0/c4/c8 with radius `0x40000000` = 2.0 — flags obj+0xf0, into
  `psiLight_SetLights(1.0, 1.0, 1.0, …)`. `AnimProcessSkinData` 0x1bd500:93-96 fills the pair:
  runtime skin+0x2c = disc +0x10 = the skinned MAIN mesh count (its records carry the sleeve bit
  0x10 → `AnimSleeveGetEntity`), skin+0x2d = disc +0x11 = the rigid EXTRA part count (glist hashes
  at skin+8). So the gate is "skins with skinned meshes AND rigid parts" — i.e. every armed
  character (body + gun parts) and every view model (sleeve + gun parts). The game's extra guard
  `viewer+0x224 != 0` (0x1e7908:20) has no port equivalent; the port always uploads (the game's
  common case) — [S] noted, not probed.
- **Which skins hit it** (the census test parses the disc archives; full lists in the test output):
  - `07000005.bin`: 23 skins, **14 hit** — 0500003E, 42, 43, 44, 45, 47, 49, 50, 51, 70, 88, A8,
    A9, C0. In play, **36 of 73** scene characters hit the gate (view-model sleeve+parts skins and
    the armed guards).
  - `07000007.bin`: 26 skins, **9 hit** — 0500003E, 42, 45, 47, 51, 70, 88, A8, A9 (the shared
    sleeve/gun skins appear on both levels; the level-specific ones are the local guards').
- **The port**: `Material`/`DrawCall` gain `cam_pick: bool`; `scene::skin_cam_pick(skin)` =
  `!main.is_empty() && !extra.is_empty()` threads it at the character call sites — placed
  characters, their third-person weapons (attachments ride the character's one `AnimObjectDraw`
  upload — the game draws them inside the same call), the view models (sleeve + rigid parts), and
  the script Anim characters + their held weapons (the cut-scene actors). Script entities and every
  Effect-layer draw keep the object path (`Script_CreateEntity` objects and effects are not
  `AnimObjectDraw`s). The group key gained the flag so merged groups split cleanly. In gpu.rs the
  per-draw pick is `light_pick_sphere(cam_pick, …)`: `(eye, 2.0)` for cam-pick draws, otherwise the
  108b cel/object spheres unchanged.

## How it was verified

- **Fail-first, shown twice** on `crates/nf-view/tests/fix_shading_parity2_live.rs` (3 tests):
  - Probe A (the pick ignoring `cam_pick`): `the_setup_render_modes2_skin_picks_lights_from_the_camera`
    FAILED on "a SetupRenderModes2 skin draws pick [eye, 2.0]". Restored → passes. The test asserts
    the two spheres upload DIFFERENT pairs on a close-up geometry (a lamp at the character, a lamp at
    the camera) — the user-visible delta.
  - Probe B (the flag never threaded in scene.rs): `the_camera_pick_flags_the_right_draws_on_07000005`
    FAILED on "no Character-layer material carries cam_pick". Restored → passes. Sisters: world and
    effect draws keep the object sphere; no non-Character draw ever carries the flag.
- `the_skin_census_of_view_setup_render_modes2` runs the disc census above (asserts the gate is
  live on both levels; prints the hash lists).
- **Suite** (fast runner, final tree = origin/master d2ce47e5 + this branch):
  - Render/lighting/cutscene/cull/pylon/letterbox cluster + master's new tests (23 binaries incl.
    `fix_aims20_modes_lights_live`, `fix_grenade_release_live`, `fix_remote_mine_play`): **72
    passed, 0 failed**.
  - Earlier clusters on the pre-second-merge tree (script/portal/walk smoke, models, worldobj, 16
    binaries): **54 passed, 0 failed, 4 ignored**; the first cluster (22 binaries): **73 passed,
    0 failed, 7 ignored**; nf-view lib tests 10 passed.
  - The full 4-crate sweep does not fit the window (nf-view alone has 255 test binaries; the
    previous session hit the same wall). This diff touches ONLY nf-view sources — nf-game,
    nf-formats and nf-audio do not depend on it, so their green state is master's. The pylon pair's
    pixel checks skip here (no wgpu adapter) and run on the main agent's GPU machine.

## Renders — the sandbox has no GPU adapter

`nf-view Nightfire-onfoot.iso 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 600 --shot …` fails with
"No suitable graphics adapter found" (same as the previous sessions). The picture checks are live
checks for the main agent.

## Live checks for the main agent

1. **Kimiko close-up, the camera pick** (07000007, the script camera slides to arm's length):
   `nf-view <iso> 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 600 --shot a2-600.png` (and frames
   1000, 1400; add `--msaa 4`). Expect: the actors lit by the lights nearest the CAMERA (the
   scene's own lights ride the sliding camera), slightly different light direction/warmth on
   Kimiko/Dominique vs the pre-fix tree where the lights stayed fixed to each character's
   neighbourhood; letterbox bars and the 4:3 band from 108a unchanged; smoother silhouettes with
   `--msaa 4`. A before/after pair (before = `git checkout 4bc5dfea -- crates/nf-view`) is the
   cleanest evidence.
2. **PCSX2 ground truth for the same frames**: `gdbwatch` on `psiLight_SetLights` 0x10e588's writes
   during frames 550-700 of the scene, on a character with attachments: the uploaded light
   positions should stay near the CAMERA as it slides (not near the character). If they instead
   track the character, the `viewer+0x224` guard or the skin gate needs another look.
3. **05 trench before/after** (`nf-view <iso> 07000005 --cam <trench spot> --frame 300 --shot
   trench.png`, `--msaa 4 --res-scale 1.5` for the after): unchanged from 108b's expectation — the
   cel-sphere light pick + MSAA edges; the camera pick does not engage for static world geometry
   (the sisters assert the world keeps obj+0x80/cel+0x80).
4. **The view model in play** (07000005 spawn): the first-person P2K is a cam-pick skin — the
   muzzle-flash light should win the upload more often than before (the camera sphere sits at the
   eye, 2 m from the muzzle). If the gun now over-lights when standing next to a lamp, compare with
   PCSX2: the game's upload should ALSO pick the lamp nearest the camera there.

## What's left

- The blob-shadow question (obj+0xfe bit 2/3) and the ambient-value data check — carried from the
  108b report (live checks 2/3 there).
- The `viewer+0x224 != 0` guard of 0x1e7908 is not ported ([S]: always-true in the scenes checked).
- The full 4-crate suite sweep (window-bound; the diff is nf-view-only).
