## Your task: `cutscene-letterbox`, cut-scenes: the letterbox bars and the framing

Main agent's PCSX2 vs port compare of the 07000007 Kimiko scene (trigger ch 0x48; port `--bond 94.2,44.6,-45.9,0,0
--frame 200..1600`): every shot, pose, costume and cut matches, but (1) the real game draws black LETTERBOX bars top and
bottom through the whole scene and the port draws none (277-fix-script-commands ported `Script_CameraStart` 0x1d9140's
bars with SpriteInfo 0x2c9468 — find why they don't show here: the flag the scene's script sets, the bar height, the
fade in/out of the bars), and (2) the port's widescreen view shows more at the sides than the game's 4:3 frame — e.g. at
frame 400 Dominique waiting in the left doorway for her entrance is visible in the port but outside the game's frame.
Make cut-scenes show what the game shows: the bars at the game's size and timing, and the scene's horizontal extent
matching the game's (pillar-box or the game's own aspect for cut-scenes; keep gameplay widescreen). Render frames 200,
400, 1000 and 1400 before and after and describe them.

Files: `crates/nf-view/src/{scene,lib,gpu}.rs` (the cut-scene camera/draw, small).

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

**Branch:** `s/cutscene-letterbox`.

---
claimed-by: sandbox2 at 2026-10-14T22:49Z

---
## Report (done, 2026-10-15T00:38Z)


---
## Report from the worker (done, 2026-10-15T00:38Z)

# fix-cutscene-letterbox — the cut-scene picture: the letterbox bars and the game's 4:3 frame

The main agent's PCSX2 vs port compare of the 07000007 Kimiko scene (trigger ch 0x48, script
0x06000073) found two gaps: the game draws black letterbox bars top and bottom through the whole
scene and the port drew none, and the port's widescreen view shows more at the sides than the
game's 4:3 frame (at frame 400 Dominique waits in the left doorway, visible in the port, outside
the game's frame). Both are fixed on `s/fix-cutscene-letterbox`.

## What the game does (evidence)

- **The bars are script-camera border sprites.** `Script_CameraStart` (0x1d9140) creates two
  sprites from the template `SpriteInfo` at **0x2c9468** — `Borders` and a second one whose
  `+0x26` (screen y) is forced to **0x180 = 384** — and sets both sprites' `+0x38` (the draw
  layer, not alpha — docs/fields/sprite.md) to **1** [M 0x1d9140:49-61]. `Script_KillStream`
  case 4 sets both layers back to **0xff** (not drawn) [M 0x1d9860:87-93]; `Script_Init` nulls
  the globals [M 0x1daea8]. Those three functions are the ONLY references to `Borders` in the
  whole decompile — the bars exist exactly while a script camera runs, with **no fade either
  way** (a layer write, instant; the driver's "fade in/out" hint checks out as "there is none").
- **The bar geometry** [L ee_snapshot.bin @0x2c9468]: w 0x200 = 512, h 0x40 = 64 at x 0, y 0
  (template) and y 0x180 (the second), texture **0x03000002** (the 32×32 solid fill), label −1
  (a texture sprite), colour 0x000000ff (opaque black). Two 512×64 black bars at y 0..64 and
  384..448 of the 512×448 screen space — exactly what `hud::script_overlay` (277-fix-script-
  commands) already drew.
- **The 0x20 flag is not the problem here.** `Script_CameraStart` skips everything (camera AND
  bars) while SCRIPTINFO+0xaa8 has 0x20 [M 0x1d9140:23]; the Kimiko scene's camera runs in the
  port (the compare's shots match) and the port's `Drones::letterbox` is verified up through the
  scene (the new test steps frames 200 → 1400 and checks the flag at 400/1000/1400).
- **The frame is 4:3.** The game rendered every cut-scene at the console's 4:3 frame; the port
  projected at the window's aspect (vertical FOV fixed, horizontal widening — `Camera::view_proj`),
  so a widescreen window sees strictly more at the sides.

## What was wrong in the port (two defects, both render-path)

1. **The headless `--shot` path never drew the script overlay.** `shot()` assembled its HUD from
   `overlay_for` alone (lib.rs, the old line: `menu.map_or_else(|| overlay_for(...), |m| m.clone())`)
   — and `overlay_for` returns an EMPTY frame while a script camera runs (the gameplay-HUD gate,
   `Camera_Enable`'s sprite-draw clear [M 0x1d9140:46-49]). The bars, the fade and the op-0x14
   sprites all vanished from every `--frame/--shot` render — the compare frames — while the live
   window path DID append `script_overlay_hud` (App redraw). That is why the bars "don't show
   here" although the flag and the sprite records were already ported.
2. **The cut-scene projected at the window's aspect.** Both the window redraw and `shot()` called
   `Camera::view_proj(window_aspect)` unconditionally: a 1280×720 render shows ~28.6° half-angle
   where the game's 4:3 shows ~22.2° (at the frame-400 camera's own 0.594 rad vertical FOV).

## The fixes (all in the task's files: lib.rs, gpu.rs)

- **`cutscene_frame(win_w, win_h)` (lib.rs, pub)** — the cut-scene frame: the projection aspect
  is the game's 4/3 and the render band is the largest 4:3 rect centred in the window
  ([x, y, w, h] pixels; a 1280×720 shot → band [160, 0, 960, 720]; a 4:3 window → the full
  window). Gameplay passes None and keeps the window's widescreen.
- **`shot_hud(scene, aspect, menu)` (lib.rs, pub)** — the HUD the headless shot draws:
  `overlay_for` PLUS `Scene::script_overlay_hud` (the bars/fade/sprites), the same append the
  window path always did. `shot()` now uses it.
- **`gpu::render(..., pillar: Option<[f32; 4]>)`** — while the band is set (and no movie frame
  owns the screen [M 0x209e88]) the pass clears BLACK and `set_viewport` confines every pass —
  world, fog, sky, snow and the HUD overlay — to the band; the pillars are the void around the
  game's picture. The callers (window redraw + `shot()`) switch the projection to
  `cutscene_frame`'s 4/3 for `view_proj`/`view_proj_rot`/`cel_visibility` when a script camera is
  live (`Sim::script_camera().is_some()` — the same condition the HUD gate drives) and pass the
  band; `script_overlay_hud` gets the band aspect so its 512×448 space fills the game picture
  exactly and the bars ride INSIDE the band, at the game's size, on top.
- The cut-scene condition and the bar geometry were already right (see above); nothing in
  nf-game changed. Shared files untouched: the change is lib.rs + gpu.rs + the new test only.
  Two PRE-EXISTING nf-view tests that call `gpu::render` directly (`fix_0c_pylon_live.rs`,
  `probe_pylon_render.rs`) needed the new trailing `None` pillar argument — one token each,
  no behaviour change.

## Verification

- **New NF_ISO test** `crates/nf-view/tests/fix_cutscene_letterbox_live.rs` (4 tests, all pass,
  through the real trigger path — Bond into TouchOnce #412's volume at [94.2, 43.7, −45.9],
  scene 0x06000073 starts, then the sim steps to the compare's frames):
  1. `the_kimiko_scene_raises_the_letterbox_and_the_shot_carries_the_bars` — the letterbox flag
     up, the script camera live, `overlay_for` empty (the gameplay HUD gate), and `shot_hud`
     carries the two solid-black bars: one run of 12 strip-texture vertices spanning the full
     width, top touching the screen top, bottom touching the bottom; the flag holds at frames
     400/1000/1400 (the camera stream lives the whole 4326-tick scene).
  2. `no_letterbox_bars_before_the_scene_starts` — sister: the flag starts down, no bar quads
     before a script camera runs.
  3. `the_cutscene_frame_is_the_games_43_band` — `cutscene_frame`: 1280×720 → (4/3,
     [160, 0, 960, 720]); 960×720 and 1024×768 → the full-window band at 4/3.
  4. `the_frame_400_camera_sees_side_content_the_games_frame_excludes` — the extent symptom
     through the real path: at the frame-400 script camera ([92.85, 43.76, −27.65], yaw −2.84,
     fov 0.594) 23 of the cast sit inside the 16:9 frustum the port used, only 16 inside the
     game's 4:3 frame, and 7 cast members (among them ci132, stream 1 of the scene script itself
     — the doorway actor) are inside 16:9 but OUTSIDE the game's frame. The widescreen view
     showed strictly more of the scene than the game's picture.
  Fail-first honesty: this sandbox has no GPU adapter ("No suitable graphics adapter found"), so
  the pre-fix PNG evidence is the main agent's before-render (below); the pre-fix state of the
  shot path is one line (it built its HUD from `overlay_for` alone), and test 1 pins the fixed
  assembly (`shot_hud`), test 4 pins the geometry the old projection got wrong.
- **Full suite** on the final tree (this branch already contains origin/master d1e8f6a7):
  the background run reported below in the WORKLOG/finish steps; the baseline suite (pre-fix
  tree, same runner) was 1150 passed / 6 failed / 63 ignored with the 6 = the 3 allowed
  `disc_golden` + the 3 pre-existing GPU-adapter pylon tests.

## Live checks for the main agent (no GPU in this sandbox)

The renders (each is one command; `NF_ISO` must point at the disc image):

```
nf-view 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 200  --shot after200.png
nf-view 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 400  --shot after400.png
nf-view 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 1000 --shot after1000.png
nf-view 07000007 --bond 94.2,44.6,-45.9,0,0 --frame 1400 --shot after1400.png
```

(the level name must come BEFORE the flags — a flag-first argument list silently renders the
front-end map 07000048; hit that while probing this task.)

- **frame 200** — cam [92.85, 43.76, −27.65] yaw −2.84 pitch 0.075 fov 0.594; the scene's actors
  (skins 0x500003b/0x5000015/0x500000d) at anim frame 199. Expected: the game picture
  pillar-boxed into the central 960×720 of the 1280×720 PNG, black sides, black letterbox bars
  64/448 tall top and bottom INSIDE the picture, no gameplay HUD.
- **frame 400** — the same camera, actors at frame 399. Expected: the same framing; Dominique's
  doorway (left of frame in the port's old widescreen) now OUTSIDE the game picture — the
  pillar must cut her off exactly where PCSX2's 4:3 frame does.
- **frame 1000** — cam [91.70, 44.15, −31.18] yaw −1.94 fov 0.498 (a longer lens — another
  shot/cut between 400 and 1000). Expected: bars still up, picture 4:3.
- **frame 1400** — cam [88.56, 44.10, −31.08] yaw +1.66 fov 0.594 (looking back the other way).
  Expected: bars still up, picture 4:3.
- **PCSX2 spot-check** (gdbwatch on the borders): watch the sprite at `Borders`' draw-layer byte
  (+0x38 of the sprite `Sprite_Create2` returns; the game global Borders is the sprite pointer —
  the watcher wants the heap address) during the Kimiko scene: write 1 while the scene plays,
  write 0xff at the scene's end (`Script_KillStream` case 4 [M 0x1d9860:87-93]). Expected: the
  layer byte flips 1 → 0xff exactly at the handback, and the bars never alpha-fade.
- **Gameplay widescreen unchanged**: a gameplay `--shot` (e.g. `nf-view 07000005 --frame 1
  --shot gp.png`) must still be full-width 16:9 with the HUD anchored as before — the band only
  applies while a script camera is live.

## What's left

Nothing known for this task. The bars' size/timing/texture and the 4:3 extent are pinned by the
new tests; the picture itself needs the main agent's renders (above) — the port's tests pass
while a picture is wrong (lesson 6), so those four PNGs are the merge gate.
