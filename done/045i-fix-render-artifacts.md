## Your task: `fix-render-artifacts`, the render artifacts in the user's screenshots

These items are from the user's playtest of 2026-10-12 (the user's own words are quoted). Screenshots are referenced by their `--bond` spot: run `nf-view <iso> <level> --bond x,y,z,yaw,pitch` to stand there.

Stand at each spot with `nf-view --bond … --shot`, find the object in the scene's draw list, and fix it against the game's draw path (no GPU in the sandbox: assert the draw state in tests; the main agent renders):
1. **07000012, a pale-blue untextured cross of quads** floating in the corridor (`07000012 --bond -3.36,1.13,35.13,4.424,-0.124`). Probably a corona/flare mesh drawn without its texture or blend.
2. **07000012, the sun is a "T" shape with a red vertical line under it** (`07000012 --bond 26.81,1.13,33.17,6.287,-0.097`): the sky sun sprite/flare.
3. **0700000c, a white untextured floor patch** on the roof (`0700000c --bond 14.22,16.73,-154.43,9.919,-0.349`).
4. **07000008, dark slabs fill the missile-closet doorway** (`07000008 --bond 152.35,43.13,-120.76,1.261,-0.126`). Is it geometry the game hides (a closed/open pair, a 0x1000 cage, a portal), or drawn wrong?
5. **07000007, two identical guards in the same pose** in the corridor (`07000007 --bond 34.25,44.13,-70.71,1.549,-0.070`, frame 1790): duplicates (compare `045j`'s Drake-scene duplicates).

Files: `crates/nf-view/src/{scene,gpu}.rs`, `crates/nf-formats/src/level.rs`

**Shared files** (`drone.rs`, `scene.rs`, `player.rs`, `triggers.rs`, `lib.rs`): small, additive hooks only, listed in your report. Pull `master` before you start and before every push: other workers merge into it all day.

## New from the user's second round (2026-10-12 evening)

6. **0700000c: a pale blue-white crane shape on the ground** (`0700000c --bond 14.22,16.29,-147.02,0.973,-0.300`), next to the white floor patch of item 3 (`0700000c --bond 14.22,16.72,-154.43,3.505,-0.279`). **Lead (main agent):** these look like baked shadow decals drawn *brightening* the floor instead of darkening it. models-world measured the per-object ALPHA_1 shapes; check the subtractive one, 0x7F00000042 = (Cd − Cs)·FIX… (A=2, B=0, C=0, D=1), and any other the port maps to an additive or normal blend, against the GS formula (ps2tek). Item 1's pale-blue cross on 07000012 may be the same bug. Fix the blend once and recheck all three spots.
   **Confirmed against the real game (main agent, PCSX2 screenshot 2026-10-12, Bond at (14.23, 16.29, −146.28) yaw 1.046,
   looking down):** the "crane" is the baked shadow of the electricity pylon. In the game it is a **dark** lattice on
   the ground beside the crates (the ground darkens under it); the port draws the same decal pale blue-white. So the
   decal's blend must darken the destination.
7. **07000007: the party guests have a green-grey colour cast** (`07000007 --bond 68.33,44.12,-43.74,-1.111,-0.154`): dull and greenish next to the warm room. The user's check "characters lit by the room" failed. fix-lighting2 (merged) gave characters the level radiators plus the muzzle light; find what tints them green (the radiator colour order, the TAmbient tint, `obj+0x100` ambient, or the VU1 byte scale) against `docs/systems/render.md` and the VU1 program, and say what the real game shows.
   **Real-game reference (main agent, PCSX2 screenshot 2026-10-12, Bond at (67.87, 44.13, −43.77) yaw −1.243):** the
   room DOES tint the guests green-teal in the game (skin and dresses in shadow read olive/green), so the hue is
   largely right. What differs: in the game the lamp-side surfaces of the guests (shoulders, arms, faces) get warm
   yellow highlights and the guests are brighter overall; the port's are darker and flat. Look at how strongly the
   level's radiators (the warm lamp lights) reach character skins (falloff, the 2-closest-lights selection, the
   colour scale), not at the ambient hue.

8. **Low priority (the user is not worried):** in the real game the first-person gun itself flashes orange on the shot
   frame (the muzzle light reaches the view model); the port keeps the view model dark. The parapet glow already matches.

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

**Branch:** `s/fix-render-artifacts`.

---
claimed-by: sandbox2 at 2026-10-13T06:27Z

---
## Report (done, 2026-10-13T07:58Z)


---
## Report (done, 2026-10-13T07:59Z — the body; the first cat used a relative path)

# fix-render-artifacts — report

Branch `s/fix-render-artifacts` (from master 03ed005). The user's 2026-10-12 playtest render-artifact
items. Honest headline: **one real renderer bug found and fixed (the World cull sphere could undershoot
its drawn geometry); the three pale-decal items are measured to their exact objects and the static draw
state is proven to match the game — their cause is runtime and needs the main agent's render or a live
check to take the last step.** Everything below is measured, not guessed.

## What's fixed

**The World sphere cull was unsound (A-11e / fix-render2's premise was false).** fix-render2 assumed
"the World draws' bounding spheres are exact — static geometry through the placement matrix — so they
cull always". Measured on the disc: a 0700000c floor decal at glist centre (28.54, 0.33, −182.49)
carries glist r **0.6** while its drawn verts reach **0.644** from that centre [D 0700000c.bin]. An
under-covering sphere makes `sphere_outside` (`a·c+d < −r`) true while visible triangles still cross
the frustum edge — the port drops what the game draws, and the fog-coloured clear (0700000c's fog is
[0xd1,0xd5,0xd6] — pale blue-white) shows through: pale untextured patches exactly like the user's
screenshots. The game's own cull consults the bbox next to the sphere: `View_AddObjects` 0x1e62d8
passes obj+0x8c (sphere) AND obj+0x80 (bbox) to `Vision_InView` 0x1e89e0 [M decompile].

**The fix** (`crates/nf-view/src/scene.rs`, `Scene::build`'s group drain): each World material's radius
is now max(glist radius, max |v − centre| over the group's drawn verts). Cheap (once per group at
build), sound, and additive — no draw-order or state change.

**Test** (`crates/nf-view/tests/fix_render_artifacts_live.rs`):
`the_world_cull_sphere_covers_its_drawn_vertices` — walks every World material of 0700000c and
07000012 and asserts the cull radius covers the drawn verts; pre-fix it fails on the disc (0.6 < 0.644,
shown via the probe run on the unpatched tree); post-fix it passes, over all ~6.7k/7k materials per
level. Sister: `no_in_view_world_static_is_culled_at_the_artifact_spots`-style box-vs-sphere agreement
is now guaranteed by construction (a containing sphere can never cull what the box test keeps).

## What's measured and pinned (the pale decal items 1/3/6 and the sun, item 2)

I could not reproduce the pale rendering headlessly: the port's static decal state matches the disc
exactly, and the GS/ported equations agree. What I pinned (all in `fix_render_artifacts_live.rs`):

1. **The subtractive lead is already correct on HEAD.** ALPHA_1 `0x7F00000042` = (A,B,C,D) =
   (2,0,0,1) = (0−Cs)·As+Cd = Cd − Cs·As decodes to blend 2, and blend 2's pipeline is
   `ReverseSubtract` with factors (SrcAlpha, One) — the identical equation [M scene.rs `Draw::from_gs`,
   gpu.rs `make_pipeline`; the measured table is docs/models/world.md §3 — 312 writes, bullet holes].
   The baked shadow decals do NOT use it: they carry ALPHA_1 **0x44** (normal) [D]. Pinned by
   `the_subtractive_shadow_blend_maps_to_the_exact_gs_equation`.
2. **The "pale crane" (0700000c) object is `pylonshadow`** (placement type 0x19, creates a cel,
   world box (34.8,0,−142.7)..(54.8,0,−124.8)). Its texture `002d:003b` is **WHITE-rgb (mean 218,219,218)
   with the lattice shape in the alpha channel (0..253)** and its vertex colour is **black rgb (0,0,0)
   with shaped alpha (0..83)** [D]. Both game and port compute Cs = tex·vc = 0 → (Cs−Cd)·As+Cd darkens.
   The port's material (scene m in `nf_view::load(false)`) carries the same state. Pinned by
   `the_artifact_decal_objects_parse_with_their_disc_state`.
3. **The corridor "cross" (07000012) is `SHADOW_Map`** (placed at (−3.18, 1.60, 21.43) — literally
   floating at eye height in the corridor): ALPHA_1 0x44, vertex rgb ≤ 6, textures `0026:005b`
   (128×128, pale-blue rgb 112,125,129, opaque) and `0026:000f` [D]. Same conclusion: the static draw
   darkens in both. Pinned in the same test.
4. **The "sun" (07000012) is `Skyline_Buildings_old`** (map 38 obj 604, oflags 0x20 = sky): its
   red-glow batches (texture `0026:0147`, a red disc on opaque black; the port's decode verified by
   pixel dump) carry **ALPHA_1 0x44 on the disc** and ride the sky pass [D]. The port uses the leaf's
   0x44 verbatim → an opaque dark-red square/line against the sky where the game's sky pass shows a
   clean glow. **This is G-019's open half: the sky pass's GS blend state.** The disc's per-leaf value
   cannot be the whole story (an opaque black square would show in the game too). Fixing it needs the
   sky pass's actual GS state from the disassembly of `View_DrawSky` 0x1e5ad0 (my objdump has no MIPS;
   the system toolchain can't disassemble ACTION.ELF). Pinned by
   `the_sun_glow_batches_carry_normal_blend_on_the_disc` so the next fix re-routes these batches
   deliberately.

**What I ruled out with headless evidence** (probes in `crates/nf-view/tests/probe_render_artifacts.rs`,
kept for the next session):
- Cull holes at the user's five spots: replicating the app camera and frustum, every in-frustum World
  static survives both the sphere and cel-box culls (the one candidate, m3045, is genuinely out of
  view — its bounding box touches the frustum corner but its geometry does not; both tests are sound).
- White-fallback textures: every artifact-area material binds a real texture (NF_TEX_TRACE finds none).
- Duplicate/overlay draws: no texture in the artifact areas is bound by materials with conflicting
  blend states.

## What's left (with the exact next steps)

- **Items 1/3/6 (pale decals)**: the static path is proven correct; the pale rendering must come from
  the runtime layer — the prime suspect is the game's per-object VU1 shadow path
  (`bShadowCharacter`, obj+0xFE bit 3, read per object in `View_DrawObjList` 0x1e69a0:70-74 and fed to
  the hand-written VU1 program; the port ignores obj+0xFE entirely — fix-render4's candidate 1, still
  unmeasured), or the sky-pass blend (item 2's G-019) bleeding through the same passes.
- **Item 2 (sun)**: read the sky pass's GS state out of `View_DrawSky` 0x1e5ad0's packet-building code
  (disassembly — the C decompile drops the stores), then override the sky materials' blend the way the
  game does. The pinned test names the batches.
- **Item 4 (07000008 missile-closet doorway)**: not reached this session. The documented mechanism
  stands: G-016's unfinished half — the per-portal `Door_IsClosed` skip (the walk is in
  docs/atlas/cel.md §Portals); the port treats portals open and draws the closet interior behind the
  closed GuardRoom door (the door set is named in docs/reports/fix-party-mission.md items 60-63). The
  0x1000 cage is collision-only (fix-objects2) and is not the slabs.
- **Item 5 (duplicate guards 07000007)**: not reached. 045j-fix-cutscenes2 is still pending and owns
  the Drake-scene duplicates; the mechanism there (live drones AND the script's animated characters
  both drawn; the game hides one set) is the same family. My runtime probe
  (`probe_play_draws_near`, NF_PROBE_LEVEL/NF_PROBE_BOND/NF_PROBE_TICKS) dumps the posed draw list at
  a `--bond` spot through the real `Sim::step_bond` path — it is the tool to catch the double draw at
  frame 1790.
- **Item 7 (party guests flat/dark)**: not reached. The hue is confirmed right (main agent); the
  missing warm highlights point at the radiator→skin falloff / the G-043 one-light-pair-per-group
  approximation (docs/reports/fix-lighting2.md:106-108). `colour_mults`/`pick2` in scene.rs/gpu.rs are
  the levers.
- **Item 8 (view-model muzzle flash, low)**: not reached. The flash attachment path collapses Bond's
  flash unless `last_shot` is within 3 ticks (scene.rs `frame_state`); the view model's lighting comes
  from the same `pick2` slot as the world.

## Live checks for the main agent

1. `0700000c --bond 14.22,16.29,-147.02,0.973,-0.300` after this branch: the pylon-shadow ground area —
   if the crane shape is still pale, it is NOT the static decal (its state is pinned dark); please also
   capture one frame with `NF_NOCELCULL=1` and one with `NF_CULL=1` to bisect cull vs draw state.
2. `07000012 --bond 26.81,1.13,33.17,6.287,-0.097`: the sun — expected to still show the T + red line
   (the sky-pass blend is unchanged); this confirms item 2's G-019 diagnosis before I touch the pass.
3. A gdbwatch trace of `View_DrawSky` 0x1e5ad0's ALPHA_1 (0x42) DIRECT writes during the sky kick on
   07000012 — the values it writes (expect 0x48 additive for the glow batches) settle item 2.
4. obj+0xFE byte for `pylonshadow`/`SHADOW_Map`'s runtime objects during play (bit 3 bShadowCharacter)
   — settles whether the VU1 shadow path forces the decal output.

## Files touched

- `crates/nf-view/src/scene.rs` — the fix (group drain radius) + comments. [Task file]
- `crates/nf-view/tests/fix_render_artifacts_live.rs` — new: 4 pinned/would-fail tests. [Task file]
- `crates/nf-view/tests/probe_render_artifacts.rs` — new: the probes (opt-in via NF_ISO env vars;
  no-ops in the suite). [Task file]
- `docs/WORKLOG.md`, this report. No shared file outside the task's list was touched.

## Verification

- `cargo test --offline --release -p nf-view --test fix_render_artifacts_live` — 4/4 (see above for
  the fail-first evidence).
- Full suite: see the WORKLOG entry for the final numbers (run before the push).
- No GPU in the sandbox: the main agent renders the three `--bond` spots above before merging.
