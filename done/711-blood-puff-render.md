## Your task: `blood-puff-render` — the blood puff is in the pool but nothing draws on screen

The playtest report "no red puffs" when shooting guards is NOT fixed. `flesh-hit-effects` gave the mesh 0x0200_08bc
gas draw slots. Its test checks that the slots exist, but nobody rendered the result. The main agent rendered it
(2026-10-16, master a4156da0) and nothing shows.

**Repro (headless, deterministic):**

    nf-view <iso> 07000005 --bond 96.9,17.9,98.8,0.53,-0.24 --pull 236 --frame 240 --shot out.png

- The shot hits guard #6 (health 10 → 4.75), and his hit reaction shows in the render.
- `drones.gas.puffs` holds one 0x0200_08bc puff at pool index 1: pos (99.13, 16.96, 101.65), scale 3.0 → 3.2,
  rgba 255 → 238, life 60, kind 5.
- The gas branch of the scene matrix code DOES run for that slot with those values. A debug print in the
  `if let Some((gi, gmesh)) = a.gas` branch confirmed it.
- Nothing red appears anywhere, even with the matrix scale ×10 AND the position forced to head + look·2, right in
  front of the camera.

So the slot's draw path renders nothing for this mesh. Find out why. Candidates, check each and measure:
- the object `level.by_hash[0x0200_08bc]` resolves to: its geometry (vertex count, bounds), its material and
  texture (alpha, blend mode), and the texture's real pixels [D];
- whether `add_instance` for the slot emitted any vertices into `groups` at all (Layer::Effect; the cel/room
  arguments passed are `&room_cels, u32::MAX, u32::MAX`), and whether a cel/portal or layer filter drops it;
- how `gas_rgba` and the effect layer's blend apply (does the vertex alpha or colour end up 0?);
- whether the other pool meshes draw: SMOKE_QUAD 0x0200_0039 (e.g. a snow hit's floor puff), 0x0200_001f and
  0x0200_026e. If none of them draw either, the bug is shared.

Fix it so the blood puff draws the way the game draws it [M View_DrawObjectGlist 0x1e6780's obj+0xF0 & 8
billboard; the gas object's colour bytes]. Add a test that fails when the puff's draw contributes no vertices or
pixels. Use whatever the scene exposes: the posed vertex list or the draw groups for that slot with non-zero
alpha, at the puff position. Write a render-side assertion, not only a pool check. Include before/after `--shot`
PNG paths in your report (the main agent will look at them). Do not commit images.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/blood-puff-render`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox1 at 2026-10-16T17:40Z

---
## Report (done, 2026-10-16T18:53Z)

Closed by the driver: the report is on `s/blood-puff-render` (docs/reports/blood-puff-render.md); the worker didn't run finish.sh.

---
## Report from the worker (done, 2026-10-16T19:13Z)


# blood-puff-render — report (2026-10-16, branch s/blood-puff-render)

## What was done

The task: the blood puff (mesh 0x0200_08bc) sits in `drones.gas.puffs` and the gas matrix branch
runs for its slot, but nothing draws on screen. I measured the whole draw chain on 07000005 and
found and fixed two render-side defects.

**The mesh, measured [D 07000005 map 10 obj 41 "BodyHit"]**: 8 verts / 6 tris (a strip), radius
0.032 (3 cm!), vertex rgba [53,53,53,128], texture map-10 #21 — a 32×32 GREY smoke texture
(brightest palette entry (213,214,214), all alpha 255). GS state per batch: ALPHA_1 0x7f00000048
→ additive (Cs·As + Cd), TEST_1 0x5001b → alpha test ≥ 1, ZBUF_1 0x131000000 → no depth write.
The flesh hit's colour args are 0xffffffff → 0xffffff00 [M `NDrone2_DealWithObjHit` 0x13fff0
decomp:245-248] — the game's own blood puff is a WHITE-tinted grey additive flash that fades to
0 over 60 ticks, billboarded (`obj+0xF0 |= 8` [M 0x140478 site]). **The disc's blood puff is not
red** — see "For the driver" below.

**Verified working (no change needed)**: the 64 draw slots per pool mesh (attachments 5967..6350
for the six meshes; the blood slots are the last 64), the group material (Effect layer, pass 5,
additive, 1152 indices = exactly 64 strips × 6 tris, lflags 0x329, texture key (10, 21)), the
skin attrs (matrix slot = attach_base + attachment index), the palette billboard (verified with
the shot's own camera axes: translation = the puff position, basis rows = the puff scale), the
CPU posed vertices (8 verts within 0.1 of the puff), the pipeline (LessEqual depth, additive,
AREF 1/128), the texture bind, and the draw submission (Effect was already exempt from the
default frustum cull).

## The two defects, fixed

1. **The Effect pools' frustum cull was unsound** (`crates/nf-view/src/gpu.rs`, rule moved to
   `Layer::frustum_cullable` in `crates/nf-view/src/scene.rs`). Every Effect group keeps its
   BUILD-time sphere: for the gas pool that is the IDENTITY-matrix centre — the `BodyHit`
   object's obj.centre ≈ **the world origin**, radius 0.032 — while its vertices are posed per
   frame through the attachment matrices. Under `NF_CULL` (the cull bisect override;
   docs/live/backlog.md L-549 asks for NF_CULL=1 captures) the group is dropped the moment the
   origin leaves the frustum — **no matter where the puffs are posed**. That is the only
   mechanism that survives the repro's "even with the matrix scale ×10 AND the position forced
   to head + look·2, right in front of the camera" — the cull sphere does not move with the
   pose. The game culls these objects per frame at their LIVE matrix (`View_AddObjects`
   0x1e62d8 passes obj+0x8c sphere AND obj+0x80 bbox to `Vision_InView` 0x1e89e0); the port has
   no per-frame sphere for the merged pools, so it must not cull them at all (the same
   unsoundness as A-11's truck vanish). The fix: `Layer::Effect` never frustum-culls, `NF_CULL`
   or not; World always does; Character/Sky keep the override. The default (no NF_CULL)
   behaviour of every other layer is byte-identical.

2. **The GPU palette never applied the gas rgba** (`Scene::skin_palette`,
   `crates/nf-view/src/scene.rs`). The game tints each object's draw by its obj+0x103..0x106
   bytes — for a gas object those are the pool's live rgba, rewritten every tick by
   `gases_smoke` 0x1ab2d0's tail (the 255→0 fade; `Gas_Init` 0x1aacc8:56-59 seeds them from the
   colour arg) and fed to `psiSetTweakARGB` 0x124770 by `View_SetupRenderModes_Tweak`
   0x1e5888:46-49 — whose byte/255 floats multiply the drawn colour AND the alpha. The port's
   GPU path (the one that renders) carried only the ambient dim for the gas attachments, so the
   puff never faded and the pool's rgba never reached the screen (the CPU `posed_vertices` did
   apply it — the two paths disagreed). The fix sets the slot's colour table entry to
   rgba/255 (colour and alpha); the shader's `c0 × cols` multiply is then exactly the game's
   tweak. Free slots keep the ambient value (their matrix is FAR).

## Tests

`crates/nf-view/tests/blood_puff_render_live.rs` (2 tests, green), through the real fire path
(settle → guard 5 ahead → aim at the torso → `Input { fire: true }`):

- `the_blood_puffs_draw_chain_is_live` — the pool puff's slot must ride a real material (Effect,
  the BodyHit texture, 3 indices × 6 tris × 64 slots, pass 5); its 8 verts' skin attrs must index
  `attach_base + attachment`; `skin_palette`'s matrix for that slot must be the billboard AT the
  puff (translation within 0.5, basis rows = the puff scale, with the shot's camera axes); the
  colour slot must equal the pool's live rgba/255 (the puff is aged 6 ticks first, so the alpha
  has left the 255 seed — the assertion discriminates the fix); and the posed vertex list must
  put all 8 verts within 0.5 of the puff. Any failure here is the "in the pool but nothing
  draws" symptom, without a GPU.
- `the_effect_pools_never_frustum_cull` — pins `Layer::frustum_cullable`: World always culls,
  Effect never (even under the NF_CULL override), Character only under the override.

Also green after the change: gpu_skin_live (the GPU-vs-CPU pose oracle), frame_cost_live,
fix_searchlight_remainder_live, fix_flesh_hit_effects_live, fix_flesh_impacts_live.

## What I ran (the suite, honestly)

The fast runner **could not complete in this sandbox**: background jobs (even `setsid nohup`)
are killed between tool calls, and the full 4-crate run (~50 min) does not fit the foreground
window. What ran in the foreground on the final tree (branch == origin/master + this task, no
drift), via `tools/test/suite.py --only` over 175 of the 436 test binaries (all the
render/draw/palette/gas-adjacent ones, then the next 130 nf-view binaries alphabetically):

- chunk 1 (45 binaries, limit 60 s): **135 passed, 1 failed** — `probe_pylon_render::
  probe_render_pylon_only`, pre-existing (fails in the previous session's chunk logs too,
  ~/suite-chunk-ae.out, alongside the documented allowed set).
- chunk 2a (60 binaries, limit 25 s): **193 passed, 3 failed** — the 2 pre-existing
  `fix_0c_pylon_live` failures + `campaign_flow_live` TIMEOUT (the 25 s limit is far under the
  runner's default 600 s; the binary needs more than 25 s, not a regression).
- chunk 2b (70 binaries, limit 25 s): **249 passed, 1 failed** — `fix_guard_weapons_live`
  TIMEOUT (same limit artifact: its damage-matrix test needs more than 25 s).

Totals over the 175 binaries: **577 passed, 5 failed/TIMEOUT (3 pre-existing failures + 2
limit-timeouts), 21 ignored**. The remaining ~260 binaries and the other crates did not run in
this window — the next session should start the fast runner first thing (default limits) and
read its total line.

## Live checks / renders for the main agent

My sandbox has no GPU (wgpu: "No suitable graphics adapter"), so the `--shot` before/after pair
is the main agent's check:

1. BEFORE (master, with `NF_CULL=1` in the env) the repro shows nothing — please confirm the
   render that judged "nothing shows" had `NF_CULL` set; it explains the symptom exactly.
2. AFTER (this branch), re-run the repro with and without the override — the result must be the
   same now:
   ```
   nf-view <iso> 07000005 --bond 96.9,17.9,98.8,0.53,-0.24 --pull 236 --frame 240 --shot after.png
   NF_CULL=1 nf-view <iso> 07000005 --bond 96.9,17.9,98.8,0.53,-0.24 --pull 236 --frame 240 --shot after_cull.png
   ```
   Expected: a small (~20 px at ~3.7 units) additive grey-white flash at the hit point on the
   guard, fading over 60 ticks. `--bond` spot for a closer look: fire at a guard from ~5 units
   and shoot at frame+2 (`--pull <t> --frame <t+2>`).
3. If STILL nothing draws after the cull fix, the ordered next suspects (from my measurements,
   everything else is verified correct): (a) the depth test at the torso surface — the quad's
   centre sits AT the hitbox surface, and if the hitbox sits proud of the visible mesh the whole
   quad can fail the LessEqual test; a one-frame diagnostic is to pose the puff 0.3 along the
   camera look in `frame_state`'s gas branch; (b) `NF_DRAWS=1` to trace whether the Effect
   material (Effect, key (10, 21)) is submitted at all.

## For the driver

- **The disc's blood puff is not red.** `BodyHit` (0x0200_08bc) is a grey smoke puff, white-tinted
  by the gas colour bytes, additive — measured from the 07000005 archive. The playtest's "no red
  puffs" may describe an expectation from other games/versions; on this disc the correct render
  is a grey-white flash. If the user's report is specifically about RED, that is a content
  question (is there a red blood mesh on another archive? a live check against PCSX2 at the same
  spot would settle it).
- The full-suite total line is missing (see above) — flagged rather than papered over.
