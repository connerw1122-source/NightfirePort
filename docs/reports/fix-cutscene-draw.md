# fix-cutscene-draw — the cut-scene cast draws

## What was broken

Every cut-scene actor (Kimiko, the guests, the Bond double — every pre-built script character)
posed as an identity mannequin at the **world origin**, ~100 m from the cut-scene camera. The
sim was alive, the palette read "real" (which is why 101b-fix-cutscene-models could not
reproduce headlessly: its census only checks liveness, finiteness and the room gate — all
true), and the render drew nothing visible. The user saw only the script camera panning an
empty library.

The drop point is **not** between `skin_palette` and the GPU draw (the task's listed suspects
— skin layout, `col` alpha, `vis` masks, the `frame_state` hidden flag, camera-relative
placement, the material filter, the 120 fps blend — all check out; `Renderer::render` does not
frustum-cull Character draws at all). It is one level up, in the pose:

`script_pose_together` (nf-game/src/anim.rs) composes a script's windows first-writer-wins per
bone (fix-cutscenes2, d907aba). The Kimiko scene's script 060004f1 lists its face track first:
windows `[(1, 04000467) — 22 channels, (1, 04000332) — 222 channels]`. The face track won
bones 0..5 of the shared pose, and its leading channels are zeros — so the **root bone
collapsed to t=(0,0,0), q=identity**, while bones 6..73 kept the body track. The body track's
root channel is what carries the actor's absolute world position: the game initialises the
anim object's position `obj+0x30` from that channel's frame-0 value (`AnimGetFirstTrans`
0x1c7348, called by `Script_AnimStart` 0x1d8f40) and advances it per frame in
`AnimFrameResolve` 0x1c35f8 (root delta, rotated, accumulated into `sAnimObject+0x30`), with
the channel→bone mapping going through the skin's table (`AnimFrameCopy` receives the
sAnimSkin; `psiBuildMatrixPalette` 0x111498 reads `sAnimObject+0x94` = the skin). A 22-channel
face track reaches only face bones there — never the body's root. The port's sequential
channel walk cannot express that sparse map, and giving the face track the first bones handed
the root to its zeros. Every script actor in every cut-scene posed at the origin: off-camera,
under the world — the missing cast on 07000007 and 07000008.

## The fix

One hunk in `crates/nf-game/src/anim.rs` (`script_pose_together`): the **base (widest, `fits`)
track owns the bones it covers**; the remaining windows fill only bones the base does not
reach (none when a full body track runs; the ledge's narrow pairs without a full track keep
the old first-writer fill). The hunk carries the decompile citations. Nothing else changed —
no shared files touched (the branch's merge commits absorb other workers' master merges, which
is git bookkeeping, not task code).

## How it was verified

Reproduce-first, on the real input path, with the lavapipe CPU Vulkan driver installed into a
user prefix (the sandbox has no GPU; `VK_ICD_FILENAMES` points at a patched `lvp_icd.json`) —
so `nf-view --shot` renders here and the whole chain from trigger to GPU frame is testable
headlessly:

- **Probes (scratch, not committed):** the pose chain for the scene's 3 script actors read
  `local[0] = (0,0,0)/identity` pre-fix and `(86.83, 43.72, −33.35)` post-fix; the root
  channel of the body seq at frames 1..300 holds the library positions (86.8,−33.3),
  (91.6..92.0,−28.4..−30.8), (91.5..92.2,−35.7..−31.1) — the game's marks; the uploaded
  palette's bone 0 flipped (0,0,0) → (86.8,43.7,−33.3) with the fix.
- **The render proof:** `--shot` of the Kimiko scene on master reproduced the empty frames
  exactly (k150/300/600/1200.png). A characters-only render (all 5052 non-Character materials
  skipped via `NF_SKIP_MAT`) and a manual closeup camera at the actor's root showed an empty
  library pre-fix and **Kimiko herself (red dress, mid-stride, by the pool table) post-fix**.
- **The committed tests** (`crates/nf-view/tests/cutscene_draw_live.rs`, 2 tests through
  `nf_view::load` + `Sim::step_bond` from TouchOnce #412):
  1. `kimiko_scene_actors_pose_from_the_body_track` — the pose gate: each of the scene's 3
     actors' root bone is the body track's non-zero library position (finite, inside the scene
     area), not the face track's zeros;
  2. `kimiko_scene_actors_palette_places_them_in_the_library` — the render gate: the palette
     `Renderer::update_skin` uploads (the exact GPU draw input, built through the census's
     eye_cell + portal walk) places each actor's bone 0 at its library root, not FAR, not
     collapsed, not the origin.
  Both **fail on the pre-fix tree** (checked by checking out the pre-fix `anim.rs`:
  `0 passed; 2 failed` — the root bone "[0.0, 0.0, 0.0] — the body track's root was
  overwritten") and **pass post-fix** (`2 passed`). The 101b census tests stay green
  (liveness, finiteness, the room gate, the persistent-viewer drive).

## What is left

- The body track's per-bone channel→bone mapping is still the port's sequential walk. The
  game maps channels through the skin's table; some per-bone slots still read far-from-root
  values (a probe vertex binding bone-72-ish slots lands ~27 m from the root), so individual
  limb poses may still be imperfect inside the now-correct root. Porting the skin's channel
  table is the clean follow-up.
- A pixel-level GPU assertion (lavapipe render, count non-clear pixels in the actor's
  bounding box) instead of the palette-level gate; the harness exists in scratch but needs a
  home that does not require the software Vulkan driver on every test machine.
- The other cut-scene scripts' face-track windows (07000005/06/08) should get the same
  closeup-render pass; the census drive covers their liveness only.
- Full suite on the final tree: see the line below (slim ISO — the 3 `disc_golden` tests may
  fail).

## Live checks for the main agent

PCSX2 + nf-view (`--bond 94.2,44.6,-45.9,0,0`, level 07000007; frames 150/300/600/900/1200/1500):
1. TouchOnce #412 (the Kimiko scene 0x06000073): Kimiko (red dress) plus two guests must be
   visible in the library pan near the pool table at the marks (86.8,43.7,−33.3),
   (92.0,43.7,−30.8), (92.2,43.9,−31.1). Pre-fix: none of them drew in any frame.
2. nf-view closeup spot: `--bond 86.8,44.0,-25.0,3.1416,0` — Kimiko standing by the pool
   table (the manual render that proved the fix here).
3. 07000008's briefcase/safe scene: Kiko plus the other scene actors besides Bond.
4. gdbwatch (action, optional): an anim object of script 060004f1 — `obj+0x30` advances per
   frame during the scene (the game's accumulated root); the port's palette root now follows
   the same channel value.
