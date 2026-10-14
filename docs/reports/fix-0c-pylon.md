# fix-0c-pylon — the 0c pylon is on screen (the drawn-set pin is replaced by a rendered-frame test)

## What was wrong

The 0700000c steel lattice pylon (parts `pylon`/`pylon_back`/`pylon_left`/`pylon_right`, placements
160-163, x 44-60, z −115..-106) was in the drawn set — 102e's pin was accurate as far as it went —
but not on screen: `nf-view <iso> 0700000c --bond 14.22,16.28,-139.54,0.986,-0.269 --skip-cutscenes
--frame 30` rendered the roof, the crates and the pylon's baked shadow, with empty sky/sea where
PCSX2 shows the tower. "In the drawn set" was not "on screen".

## Root cause (all [M], decompile addresses)

The port drew the post-fog sky AFTER the alpha list; the game draws it BEFORE.

- `Game_Draw` (0x1e6f50, decompiled) draws, per viewer: `View_DrawSky(v,1)` (sky group 1) →
  `View_DrawObjList(v,0)` (list 0) → `psiPostSolid` (0x121630 = `psiDrawFogPass` 0x10b960) →
  `View_DrawSky(v,0)` (sky group 0) → `View_DrawObjList(v,1)` (**list 1, the alpha list**) →
  shards/`DrawDrops` → (`psiClearZ`) → `View_DrawObjList(v,2)` (characters) → sprites.
- `View_DrawSky` (0x1e5ad0) selects the 21 sky slots whose `GfxList[i+0x4a] ==` its argument —
  sky group 0/1 (docs/systems/render.md §Sky remainder already pinned this: group
  "0 = after psiPostSolid / before the alpha list").
- The port's gpu.rs frame order sorted every pass by `dc.pass * 2`, which put pass 4 (the sky
  group 0 equivalent) after pass 3 (the alpha list). The sky pipeline is pinned to the far plane
  (`FAR` override sets `o.p.z = o.p.w`) with a `LessEqual` depth test and **no depth write**, so it
  paints wherever the depth buffer is still at the clear value 1.0.
- The pylon rides the alpha list (chain == u32::MAX dynamic World object, glist flags bit 0x1 →
  pass 3, fog 2) with **ZBUF_1 zwrite off**. Its struts therefore leave the depth buffer untouched;
  the sky pass then repainted every one of its pixels with sky. The baked shadow decal (node 74)
  survives because its leaf sets zwrite **on** — exactly the observed "shadow yes, pylon no" split.
  The same mechanism erased the pylon against sky/sea at every camera (the close-up at 14 units
  still showed nothing), while alpha draws over opaque geometry (depth < 1 written by list 0)
  were unaffected — which is why the rest of the level looked right.

## The fix

`crates/nf-view/src/gpu.rs` (one slot-map change in the frame sort, plus the citation comment):
`let slot = match dc.pass { 4 => 3, p => p * 2 }` — the post-fog sky now draws in slot 3, i.e.
after list 0 (slot 2) and before the characters (slot 4) and the alpha list (slot 6), matching
`Game_Draw`'s order. Nothing else moved: pass 0 (pre-world sky) stays first, effects (pass 5)
stay last. The sky keeps its z-pinned far-plane test — before the alpha list that test is exactly
the game's "only where nothing solid drew" behaviour; after the alpha list it was an eraser.

`crates/nf-view/src/scene.rs`: `#[derive(Clone)]` on `Material` (test support, so headless render
tests can stage material subsets per render; additive only).

## Reproduce before fix (all through the real path)

1. The binary render at the user spot showed the empty sky/sea (kept as
   `/renders/ab_base.png` md5 f117649587b43e6dc99a3149ccbf8ec9, identical bytes with
   `NF_NOCELCULL=1` — 2113 extra draws, byte-identical frame: the culls were never the cause).
2. `tests/probe_pylon_draws.rs` (kept, opt-in NF_ISO): the pylon's 12 materials (nodes 116-119)
   are pass 3 / blend 0 / zwrite false / atest 1 / fog 2, textures resolve (45:93 64×64, 1017
   opaque texels of 4096, max alpha 255 — a real alpha-tested lattice; 45:8 and 45:0 fully
   opaque), vertex alpha 128 (→1.0), bbox on screen (NDC x 0.03..0.29), depth 0.999 — the draw
   data was never the problem.
3. `tests/probe_pylon_render.rs` (kept): in-process subset renders through the exact shot() path.
   pylon-only → **the tower renders perfectly**; pylon+list0 (6218 materials) → renders;
   pylon+pass3 (456) → renders; pylon+others (sky/characters/effects, 316) → **gone**; full
   7003 → gone. That pinned the killer to the sky/character/effect group, and the only member
   that covers the tower's pixels is the pass-4 sky.

## Tests (fail-first)

`crates/nf-view/tests/fix_0c_pylon_live.rs` — renders the FULL scene headlessly through the exact
`shot()` path (wgpu surfaceless adapter + `Renderer::render` + framebuffer readback) and checks
pixels:

- `the_0c_pylon_is_on_screen_at_the_user_spot`: the bond camera at the user's `--bond` spot
  (pitch 0 so the whole mast is in frame), the lattice band x 590..730, y 90..400 must carry
  >3000 dark pixels (pre-fix 1628 — sky only; post-fix 5180). Sisters: the sky beside the tower
  stays bright (75000 bright pixels; guards against a dark-screen regression) and the baked
  shadow decal still draws (>500 dark pixels; 40113).
- `the_0c_pylon_fills_the_view_from_the_closeup_spot`: the free camera 14 units from the tower;
  >2000 dark pixels in the tower's screen area (pre-fix 75, post-fix 4229).

Both tests were run against the pre-fix tree (gpu.rs stashed): both FAILED (1628 / 75); with the
fix both PASS (5180 / 4229). The harness is deterministic (fixed 60 Hz sim, no wall clock, one
sample, fixed tick 30).

## Render before/after

- Before (`--bond 14.22,16.28,-139.54,0.986,-0.269`): roof, crates, the lattice's baked shadow on
  the concrete, empty sky/sea right of centre.
- After (same args): the steel lattice tower right of centre against the sea, shadow intact.
  Driver-reproducible: `nf-view <iso> 0700000c --bond 14.22,16.28,-139.54,0.986,-0.269
  --skip-cutscenes --frame 30 --shot after.png`; at pitch 0 the whole tower is in frame
  (`...0.986,0.0...`).

## Full suite

`tools/test/suite.py` on the final tree (origin/master 6691ef9 merged; WORKLOG conflict resolved by
keeping both entries): **989 passed, 4 failed, 31 ignored (31 min)**. The 4:
`disc_golden::bigf_header_fields_behave_as_documented`, `disc_golden::bigf_tables_match_the_golden_manifest`,
`disc_golden::directory_tree_matches_the_golden_manifest` — the 3 disc_golden tests the slim ISO is
allowed to fail — and `TIMEOUT weapon_matrix_live after 600 s (last started: matrix_0700001b)`: the
weapon-matrix binary renders its whole matrix through the sandbox's software Vulkan (lavapipe) and
hit the runner's 600 s wall-clock; it is a speed artefact of this sandbox, not a correctness failure
(the change adds no per-draw cost — one integer comparison in the frame sort). `fix_0c_pylon_live`
passed on the merged tree (2/2, ~1 s).

## Files touched

- `crates/nf-view/src/gpu.rs` — the task's file: the slot map + comment.
- `crates/nf-view/src/scene.rs` — the task's file: `#[derive(Clone)]` on `Material` (+ 2-line doc).
- `crates/nf-view/tests/fix_0c_pylon_live.rs` — new test file.
- `crates/nf-view/tests/probe_pylon_draws.rs`, `probe_pylon_render.rs` — kept probes (opt-in NF_ISO).
- Shared files: none outside the task's list (lib.rs/hud.rs/menu.rs changes in the merge are other
  workers' work arriving via master, untouched by me).

## What's left / notes

- The game draws characters (list 2) AFTER the alpha list (`View_DrawObjList(v,2)` at Game_Draw
  0x1e6f50:175); the port still draws pass 2 (characters) before pass 3. Nothing in this playtest
  hinged on it and the change would touch every level's character/alpha interaction, so it is
  left here with the citation — a follow-up if any character-vs-alpha ordering symptom shows up.
- The fog pass stays modelled as the clear colour (the port's documented approximation); the game
  runs `psiDrawFogPass` between list 0 and the sky group 0. The pylon bug did not need that changed.
- The suite's nohup run in this sandbox: the process survives between tool calls here (pid check),
  but the earlier sessions' "nohup dies" caveat may still apply on other sandboxes.

## Live checks for the main agent

nf-view (`--bond` spots, `--skip-cutscenes --frame 30`):

1. `14.22,16.28,-139.54,0.986,-0.269` — the pylon's lattice right of centre against the sea; its
   baked shadow still on the concrete below-left. (The playtest spot.)
2. `14.22,16.28,-139.54,0.986,0.0` — the whole tower in frame from base to mast top.
3. `34 18 -124 0.707 0.05` (as `--cam 34 18 -124 0.707 0.05`) — the tower fills the centre-right
   14 units away; the pier building stays at the left edge.

PCSX2 (spot / action / expected): stand at the 0700000c courtyard spot facing +x/+z
(`--bond 14.22,16.28,-139.54,0.986,-0.269`) and compare the frame against nf-view spot 1 — the
lattice should now agree with the PCSX2 picture (tower right of centre, same shadow). No gameplay
action needed: this is a static-frame comparison.
