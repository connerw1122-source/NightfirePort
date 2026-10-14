## Your task: `window-view`, 07000007: the view out of the windows

PCSX2 comparison (main agent, 2026-10-13) at `--bond 61.32,44.11,-52.58,3.749,-0.056` on 07000007, looking out of a
window: the real game shows the snowy slope with a blue haze over it (distance fog) and nothing else; the port draws a
lit stone building block floating in front of the slope, and the snow texture is smeared/stretched with no haze. Part of
it may be `NF_NOPORTAL=1` (the playtest launcher turns portal culling off until 130g-fix-portal-sweep lands): render the
spot with and without it. Find what the block is (instance/cel) and why the game doesn't draw it from inside, the
outdoor fog the game applies (fog colour/range per cel or per level) and the snow's UVs (texture matrix/scroll or wrap
mode). Tests: at that spot the block's instance is not in the drawn set; the fog parameters for that cel; the snow's UVs
match the disc's; sister assertions.

Files: `crates/nf-view/src/{scene,gpu}.rs`.

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

**Branch:** `s/window-view`.

---
claimed-by: sandbox5 at 2026-10-14T03:34Z

---
## Report (done, 2026-10-14T05:05Z)


---
## Report from the worker (done, 2026-10-14T05:05Z)

# fix-window-view (101i) — the view out of the 07000007 ballroom window

Branch `s/fix-window-view` (from origin/master eed9a87; final tree merges origin/master 6a118b8).
Spot: `--bond 61.32,44.11,-52.58,3.749,-0.056` on 07000007 (the Mayhew party, the ballroom window;
the same window as the ground-truth doc's `61.11,44.12,-52.58,-2.541,-0.010` — −2.541 + 2π ≈ 3.742).

## What the investigation found (all three symptoms)

**1. The "lit stone building block floating in front of the slope" is the launcher's
`NF_NOPORTAL=1`, not a port bug.** With the portal walk off, the renderer falls back to the
box test everywhere and the view gains 543 World draws — led by **`Room_Gallery_K_2`** (world
centre (49.1, 45.6, −51.1), graph cell 26, room cel 20, 12.3 units in front of the window) and its
~140 materials, plus the other cross-building rooms. The block is lit (interior vertex colours),
its 32×32 interior textures tile hugely across its faces seen from outside, and being list-0
geometry it depth-writes over the sky domes — "a lit stone building block floating in front of the
slope, the snow texture smeared/stretched". With the game's cel walk
(`Vision_Portal_Recurse` 0x1e86e8 → the renderer's per-frame cell mask, `View_AddCels` 0x1e6ba0)
the block's cell is NOT visible and the drawn set is 280 draws: the ballroom interior
(`Room_Gallery_K_3`), the window glass and the two sky domes — exactly the real game's view
("only the snowy mountain through the window; no lit building" [L
docs/live/2026-10-13-portal-ground-truth.md, the ballroom-window row]). The walk-on port already
matched that ground truth on a5bff8b + fix-portal2; this task re-proves it on the current tree.

**2. There is NO level fog on 07000007 — the game's blue haze over the slope is the skybox dome's
own dusk textures.** `psiPreGameRun` (0x11e2e8) puts 07000007 in its `default` arm: `FogOn = 0`,
`Ps2Far1 = 4194240` [M decompile re-read]; `psiDrawFogPass` (0x10b960) runs its two packets only
`if (FogOn != 0)` [M]. So the port's `scene::fog_for(0x0700_0007) == None` is correct and must NOT
be "fixed" by adding a fog entry. The haze the game shows through the window is painted by the
sky domes themselves: map47 `skybox` (object 279, r 1449.8) carries tex 177/179/180/181 — dusky
pink-blue sky bands with a hazy horizon (dumped to BMPs from the disc; the textures are a purple
dusk sky, cloud bands and the sun), and `Mountains_skybox` (object 280, r 244.6) carries tex
178 (plain snow) + 182/183/184 (snowy craggy rock) — the snowy slope. The port draws both domes
with the disc's own UVs (see 3), so with the walk on it paints the same haze.

**3. The snow's UVs match the disc bit for bit.** For both dome objects the multiset of (u, v) f32
bit patterns over the port's index buffer equals the multiset over the disc's glist vertex stream
(`add_instance` copies `v.uv` verbatim; the stream format itself was cross-checked against
nightfire-re's own VIF reader — docs/formats/ps2gfx.md §Results). The suspicious-looking "smear"
UVs are the disc's own data: a skybox batch of 288 corner references samples tex 178 only in
u[0.48..0.51] × v[0.48..0.52] — a small centre patch stretched over the dome (the game's own haze
band), asserted in the test.

**4. The one real port bug found and fixed: the frame's pass order.** `Game_Draw` (0x1e6f50)
draws: `View_DrawSky(viewer, 1)` (line 93) → list 0 (line 108) → `psiPostSolid()` (line 114, the
fog pass) → `View_DrawSky(viewer, 0)` (line 123) → **the alpha list** `View_DrawObjList(viewer, 1)`
(line 139) → shards → `DrawDrops` (line 151) → list 2 (line 175) [M]. The port's render sorted the
passes ascending (`dc.pass * 2`), drawing the alpha list (pass 3, the window glass) BEFORE the
after-fog sky (pass 4). The sky pipeline is far-pinned and depth-write-off, and the glass writes no
depth, so the dome overpainted the glass everywhere: the window's glass texture (tex 11, a soft
white radial sheen) and its tint never reached the frame — a real contributor to the "no haze"
half of the report, on every level with glass over sky, not just 07000007. Fixed with
`gpu::pass_rank` ([0,1,2,4,3,5] — pass 4 before pass 3, everything else unchanged) used as the
primary sort key in `render`. The sky's own GS state on 07000007 (all 10 dome draws): pass 4,
blend 0 normal, zwrite on, atest As ≥ 1, fog 0 — pinned by the test.

The sky draw path itself matches the decompile and needed no change: `View_DrawSky` (0x1e5ad0)
builds the dome matrix as identity rotation (0x2d6230, identity since bootup — 0x1c7e78) ⊗
`RotMatrix` of the slot's stored rotation, translation from the slot's stored vector (the
placement's, `View_AddSkyObj` 0x1e79a0 +0x20..0x2c) — or the viewer's rotation/translation for
glist-flag bits 0x8/0x2 (both domes carry neither bit; obj flags 0x21/0x20). The port's
`sky_instances` + `Layer::SkyFixed`/`SkyCamera` reproduce exactly that. Registration also sets
`obj+0xf0 |= 0x10` (hidden for the cel pass — 0x1e79a0:37-39), so the domes draw only through the
sky pass; the port drops the 0x2a placements from the dynamic instances the same way
(scene.rs `instances.retain`).

## Files

- `crates/nf-view/src/gpu.rs` — the fix: `pass_rank` (the game's frame order) as the render sort's
  primary key, with the `Game_Draw` line citations.
- `crates/nf-view/tests/fix_window_view_live.rs` — 5 NF_ISO tests (below).
- `crates/nf-view/tests/fix_window_view_dump.rs` — the disc-texture dump helper (opt-in via
  `NF_TEX_OUT`; wrote the dome/glass BMPs I inspected, outside the repo).
- Shared files: none touched. The merge with origin/master conflicted only in `docs/WORKLOG.md`
  (both entries kept). `docs/systems/render.md` already documents the sky/fog/order facts this
  task verified (its "Sky: View_DrawSky twice" line and the alpha-list fade note).

## Tests (`crates/nf-view/tests/fix_window_view_live.rs`, opt-in via `NF_ISO`, all green)

- `block_hidden_by_the_walk` — the Gallery_K_2 block's materials are NOT in the drawn set at the
  window spot with the walk on, ARE with `NF_NOPORTAL` (sister), and the walk-on set is less than
  half the walk-off set. Fails under any portal-mask regression (e.g. the pre-fix-portal5 state).
- `window_view_is_the_sky_domes` — both domes (r 244.6 and r 1449.8) sit in the window frustum,
  all dome draws carry pass 4 and fog 0.
- `sky_draws_before_the_alpha_list` — the pass ranks match `Game_Draw` 0x1e6f50:93/108/123/139.
  Against the pre-fix ranking (`pass * 2`: sky 8 > glass 6) the first assertion fails — that was
  the reported overdraw.
- `fog_params_for_07000007` — `fog_for(0x0700_0007) == None` [M 0x11e2e8 default]; sisters: 08 =
  ([0x3b,0x50,0x69], 13191350.0), 05 = ([0x3a,0x4d,0x68], 19118528.0).
- `snow_uvs_match_the_disc` — the dome UV multisets equal the disc's glist stream bit for bit
  (both objects), and the disc's own tiny centre-patch haze batch exists (the "smear" is the
  game's data).

## Full suite on the final tree (origin/master 6a118b8 merged)

The suite runner (`tools/test/suite.py`) cannot survive this sandbox's tool timeout (10 min cap;
it needs ~20) nor stay alive between tool calls, so the rule-5 command was run in package chunks,
all with `--test-threads=1`, on the merged tree: **nf-game 256 passed, nf-formats 74, nf-audio 13,
nf-view 575 — 918 passed, 0 failed, 0 ignored, no TIMEOUT/CRASH**. The 3 `disc_golden` tests (and
`pss_movies_live`) did not fail on the slim ISO. Before the merge, every nf-view binary was also
green on the pre-merge tree.

## What's left / open

- **The visual confirmation is the main agent's** (no GPU here): render the spot with the walk ON
  and no `NF_NOPORTAL`, before/after the pass-order fix (below).
- The VU1 microcode stays undecoded (G-014). The dome UVs match the disc's stream, and the port's
  shader consumes them directly; if the main agent's render still shows wrong dome texels after
  the order fix, the next suspect is a VU1-side UV/env term for the sky batches — that needs the
  microcode, not a guess.
- The playtest symptom's complete fix is the launcher stopping `NF_NOPORTAL=1` once
  130g-fix-portal-sweep lands (its 07000006 courtyard bug is not this task's).

## Live checks for the main agent

PCSX2 / nf-view renders (`--bond` spots):
1. `07000007 61.32,44.11,-52.58,3.749,-0.056` — walk ON, post-fix: expect ONLY the ballroom
   interior + the window frames + the dusky sky/mountain domes through the glass, the glass sheen
   (tex 11) visible over the sky, NO building block. Compare with the real game's shot in the
   ground-truth doc.
2. Same spot with `NF_NOPORTAL=1`: expect the lit Gallery_K_2 block to reappear covering the
   slope (documents the playtest symptom as the launcher flag).
3. Same spot pre/post pass-order fix with the walk on: the glass sheen over the dome should
   appear only post-fix (the dome used to overpaint it).
4. A glass-over-sky spot on another level as a regression check for the order change, e.g.
   `07000008` outdoors windows / `07000005` `--bond 33.61,32.12,59.22,1.117,-0.2` (the pine-tree
   alpha-cutout spot from models-world.md) — expect no new overdraw differences.
5. `07000007 34.25,44.13,-70.71,1.549,-0.070` (the clock hall, the ground-truth row): guards,
   clock, shields — confirms the order fix did not disturb the walk's matches.

gdbwatch (if convenient): during a held frame at spot 1, PINE `VisCelCount` 0x30d0bc /
`VisCelList` 0x34b3b0 for the main viewer — expect the eye room + the rooms the ground-truth walk
names, and NO Gallery_K_2 cel in the main view's list.
