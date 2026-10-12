## Your task: `fix-lighting2`, finish fix-lighting so it can merge: the per-draw light selection and the colour scale

`s/fix-lighting` did the hard part: the VU1 microcode is read (the point-light formula, G-014 closed), and it added the
sim light list (`crates/nf-game/src/light.rs`: `Light_Create` with its RNG draw, `Light_Update`, the selection, the
level's ambient radiators and 0x4E lights, the guards' muzzle lights, the smoke). The main agent **did not merge it**
because the render regressed. Start from that branch: check out `s/fix-lighting`, merge current `master` into it
(master moved on), and continue on a new branch `s/fix-lighting2`.

**What the main agent found** (rendering 07000005, the P2K shot at frame 30, `nf-view <iso> 07000005 --frame 30 --pull 30 --shot`):
1. **The muzzle light never reaches the world draws.** Each draw uploads its 2 closest lights (`pick2` over
   `closest(sphere, flags)`). The port has no per-object display flags (`obj+0xF0`) and passes **0x80 for every
   draw** [S], so the level's permanent **kind-0 ambient radiators** (`Light_SetAmbientRadiators`) qualify everywhere,
   rank closer, and fill both slots. The muzzle light (kind 1) is crowded out. Experiment: excluding kind 0 from the
   world draws brings the glow back. The glow the user had approved in play is gone on this branch.
2. **With it back, it saturates.** The gun and hand go solid yellow and the parapet orange. The shader adds
   `lc.rgb · term / 128` onto `c`, but `c` is still the vertex colour in byte/255 units at that point (it's multiplied by
   255/128 only afterwards). Check the unit chain end to end, and how the game avoids blowing out the view model at
   under one unit from the muzzle (a separate light setup for the view model, its own flags, or a clamp in the VU program).

**What to do:**
- Find the real per-object flags: `obj+0xF0`'s 0x80 / 0x100 bits per object class (world cels in `View_DrawCelGlist`
  0x1e6538, objects in `View_DrawObjList` 0x1e69a0, characters, the first-person view model), and which kinds of
  light each class really gets. Map them per draw in the port (`Material` can carry the source object's class).
- Fix the colour units so the shader's sum is the game's (byte scale, 0x80 = 1.0), with a test that pins a known vertex's
  lit colour to the formula's number.
- Write the live checks for the main agent (see `docs/fields/README.md` and the probe `tools/trace/fieldprobe.py`): which
  light-list address to read in PCSX2 at a P2K shot on 07000005, what `obj+0xF0` should read for the parapet's cel, and so on.
  The main agent will run them on the real game and compare its render with a PCSX2 screenshot.
- Keep the render-independent tests, and add one that fails on this branch as-is: on 07000005 at the shot, the world
  draw nearest the muzzle must select the muzzle light.

Files: `crates/nf-game/src/light.rs`, `crates/nf-view/src/{gpu,scene}.rs`, `crates/nf-view/tests/lighting_live.rs`.

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

**Branch:** `s/fix-lighting2` (from `s/fix-lighting` + master).

---
claimed-by: sandbox2 at 2026-10-12T19:52Z

---
## Report (done, 2026-10-12T21:47Z)


---
## Report (done, 2026-10-12T21:48Z)

# fix-lighting2 — the per-draw light selection and the colour scale

Branch `s/fix-lighting2` (from `s/fix-lighting` + current master). Fixes the two render regressions
the merge flagged on `nf-view <iso> 07000005 --frame 30 --pull 30 --shot`: the muzzle light never
reaching the world draws, and the saturation that came back with it.

## What's done

**1. The per-draw light flags are the real ones** (`crates/nf-view/src/scene.rs` `Material::lflags`,
`crates/nf-view/src/gpu.rs` render). The port used to pass 0x80 for every draw [S, the old
fix-lighting note] — that made 07000005's 23 permanent kind-0 ambient radiators qualify on every
world draw, rank closer than the muzzle light, and fill both `psiLight_SetLights` slots. The game's
gate is the source object's glist flags word:

- `Control_SetGList` 0x134960 writes `obj+0xF0 = celglist+0x44 | (old & 0x30008000)` [M];
  `View_DrawObjList` 0x1e69a0:79 passes `*(u16*)(obj+0xF0)` to `Lights_CalcClosestLights`
  0x1cb2e0; `View_DrawCelGlist` 0x1e6538:59 passes `*(u16*)(cel+0x94)` (the same word — the port's
  `MapObject.flags` is exactly it, docs/formats level.rs ENTITY_PARAMS +4) [M]; the anim-object
  path (`AnimObjectDraw` 0x1c4490:119 → `View_SetupRenderModes` 0x1e78b0:21) passes the same
  obj+0xF0 u16 [M]. Kind gates [M 0x1cb2e0]: kinds 0/2 need flags & 0x80, kind 1 (muzzle) is
  skipped when flags & 0x100.
- **Measured on the disc** (probe in the report history, 07000005): world cels carry almost no
  0x80/0x100 — 131 of 132 cel-creating placements' glists have neither bit; the radiators never
  light world geometry. Character/weapon skins carry **0x8a0** (guards' meshes, the held guns) and
  **0x28a0** (the first-person weapon parts, the hands) — 0x80 set, 0x100 clear, so characters get
  the level radiators (the system's actual purpose) AND the muzzle light — the user's "lights fall
  on characters". Effect objects carry **0x0321/0x0329** (0x100 set — muzzle-excluded). The wine
  truck 0x0880, the ammo pickup 0x08c0, the alarm switches 0x0080.
- The port now threads `MapObject.flags & 0xffff` per instance into the material group key
  (`add_instance`) and the `DrawCall`; every class (world statics, dynamics, character-layer
  posed objects, drone skins via `character_instances`' per-mesh objects, view models, effects)
  gets its source object's word.
- The sky and sprite passes upload NO lights: `View_DrawSky` 0x1e5ad0:89 and `View_DrawSprites`
  0x1e5290:41 call `psiLight_SetLights(0,0,0,0,0)` [M] (Game_Draw's shard pass too, 0x1e6f50:150).
  The port zeroed the sky slots (it used to pass flags 0, which still admitted kind-1 lights).

**2. The colour units are the game's** (`gpu.rs` shader, `light.rs::shade_vertex`). The merge's
"2×" suspicion was right, one step earlier in the chain: the port's vertex colour arrives as
**Unorm8x4 = byte/255** (gpu.rs attribute 2), while the old shader added `lc.rgb·term/128` — the
game adds `lc.rgb·term` in BYTES over its byte-scaled vertex colour, so the port's add was
255/128 ≈ 2× too strong (the parapet's ~76-byte glow rendered as ~152; the view model went solid).
The VU1 program (.vutext 0x241c80, re-disassembled this session — decoder against PCSX2's
microVU_Tables) settles the rest:

- per vertex: `L = lightpos − p` (object space), `d² = |L|²`, `1/d²` via
  `DIV Q, VF00.w, VF28.y` (I-mem 866/902 — `_Fsf_` = bits 21-22 = w, the 1.0-in-VF00.w trick;
  PCSX2 microVU_Misc.h defines `_Fsf_` as bits 21-22, my first read had the two fields swapped),
  `atten = max(0, 1/d² − 1/r²)` (SUBw 877/879 + MAXx 884), `diffuse = max(0, atten·dot(N,L))`
  with the RAW L (the broadcast-sum dots at 890-894/901-904), the sum
  `TAmbient⊙rgba + TColor0·term0 + TColor1·term1` (895/905/911/912),
- **the clamp the port was missing**: `MINIi .xyzw VF23, VF22, I` at I-mem **919/954** with
  **I = 255.0** (the LOI at 900/936 — the immediate rides the lower word, 0x437f0000) — the game
  clamps the summed byte colour to 255 per channel before the FTOI0 output store. The view model
  at 0.5 units from the muzzle gets an add of ~(0, 549, 1015) bytes and the game clamps it to
  (·, 255, 255) — the one-frame flash saturates there too; what the port lacked was the clamp
  that keeps the texture detail (min after the modulate flattens everything).
- The shader now: `c = min(c.rgb + lc.rgb * term / 255.0, vec3(1.0))` — the byte-scale add in
  byte/255 units plus the MINIi clamp. `shade_vertex` mirrors the clamp (min 255).

**How the game avoids blowing out the view model**: it doesn't — it clamps at byte 255 (the MINIi),
and the muzzle light (radius 7 [D weapon_data+0x58], rgb (0,138,255) [D], intensity 2.0) legitimately
saturates the gun for its one tick. The parapet's orange at 2-4 units is the game's glow.

**3. The fail-first** (`crates/nf-view/tests/lighting_live.rs`): `the_world_draw_nearest_the_muzzle_selects_it`
loads 07000005, drives the real input path (`step_bond` shot), and asserts the world draw nearest
the muzzle selects the muzzle light in slot 0 with nothing in slot 1. Shown failing first with the
old hardcoded 0x80 gate ("the world draw's slot-0 light is NOT the muzzle (dist 2.83) — the
radiators crowd it out"), passing with the real flags; a permanent sister asserts the 0x80 pick
still selects a radiator (the bug's witness). Plus `the_shaded_colour_matches_the_vu1_byte_scale_and_clamps_at_255`
(pins the formula's numbers: term = 120/441 at d=3 → (0, 75.1, 138.8); the d=0.5 clamp to (0,255,255)),
`character_skins_take_radiators_and_the_muzzle_world_cels_do_not` (the disc's per-class flags
through the built materials), and gpu.rs `the_light_add_is_in_byte_255_units_with_the_minii_clamp`
(pins the shader line). The existing 9 `light_selection.rs` tests still pass (shade_vertex's clamp
doesn't engage at their values).

**4. Merge fallout, fixed in passing** (each cited):
- `s/fix-lighting` was committed with `[m`-byte sequences stripped out of two index expressions
  (`level.maps].radiators`, `level.mapsap].placements` in light.rs — the same corruption this
  sandbox's text pipeline inflicts) — the branch as pushed could not compile; fixed, and the
  merge commit's drone.rs `Drones::new` was missing master's `useobjs/bullet_path_weapons/copters/locks`
  initialisers (my merge-resolution miss) — fixed in 859afe1.
- `fix_mission_live.rs`'s 07000001 escort exit (a file this task does not own — minimal, noted):
  the merged RNG stream (fix-lighting's light draws + master's changes combined) left Mayhew
  oscillating at the courtyard door whose opening sweep was frozen at 0.233 by Bond's own capsule
  (the pause-when-blocked state; master's stream slipped past it). The test now presses from the
  use sphere's edge with a step-back cadence so the door completes its opening, and stands Bond ON
  the exit plate when Mayhew nears it (the game's Bond waits there), so the escort leaves the
  PlayerInWay band and walks the last stretch. All 4 of its tests pass.

## How it was verified

- Full suite: `cargo test --offline --release -j2 --no-fail-fast -p nf-game -p nf-formats
  -p nf-audio -p nf-view -- --test-threads=1` — **495 passed, 3 failed — only the 3 `disc_golden`**
  (the slim ISO, the task's allowance).
- The fail-first transcript is in the report history above; the naga WGSL validation still passes
  with the new shader line.
- No GPU here: the picture needs the main agent's render (below).

## What's left / not ported

- The VU1's second-light reciprocal is computed by an ERSADD→P→MFP chain whose consumer I could
  not fully close statically (the MFP'd value never re-enters the FP registers in .vutext —
  possibly dead or fed through the packet scratch stream the 770-831 program writes). The port
  computes both lights with their own 1/d²; if the game really shares light-0's 1/d² for the
  second slot, that's a sub-visible quirk — a live trace can settle it (below).
- The per-frame light selection for the merged Character-layer materials (all same-skin guards
  share one material/sphere) is the pre-existing G-043 batching approximation — one light pair for
  the group, not per guard. The game selects per object. Not this task's files.
- The smoke puff's render, the flicker's Sound_Play3D — unchanged from fix-lighting's list.

## Live checks for the main agent

Run in PCSX2 with the debugger (`tools/trace/gdbwatch.py` reads, `tools/trace/fieldprobe.py`
pokes), 07000005, Bond at the start with the P2K, one shot:

1. **The light list at the shot.** Read `LightList` (0x2c74c0) the frame after the shot: expect
   23 permanent kind-0 lights (+0x47 = 0) plus one kind-1 (+0x47 = 1) light at the muzzle
   (eye + aim·0.5 [S]), radius 7.0 (+0x3C), intensity 2.0 (+0x40), rgb (0,138,255) (+0x44..46),
   life +0x20 = 1 — and gone the next frame.
2. **The world draw's flags.** Break on `View_DrawCelGlist` 0x1e6538 for the parapet cel in front
   of the shot (the courtyard wall ahead of the player start): read `*(u16*)(cel+0x94)` — expect
   **0x0000** (no 0x80, no 0x100: no radiator light, muzzle light admitted). Also read
   `*(u16*)(obj+0xF0)` at the `View_DrawObjList` 0x1e69a0 break for a guard object — expect
   **0x08a0** (or 0x0880/0x28a0 on weapon parts): 0x80 set, 0x100 clear.
3. **The view model's selection.** At `AnimObjectDraw` 0x1c4490's `View_SetupRenderModes` call on
   the shot frame: the player object's `*(u16*)(obj+0xF0)` — expect 0x8a0-class (radiators + muzzle
   eligible), and `Lights_CalcClosestLights`' output: slot 0 = the muzzle light (the falloff
   0.5/(7·0.5) ≈ 0.14 beats the near radiators' 0.27+).
4. **The saturation clamp.** Screenshot the shot frame (the main agent's render should match): the
   first-person gun/hand go full-bright (the muzzle colour, byte-clamped 255) for exactly one
   frame, the parapet/wall ahead glow orange-blue at ~2-5 units WITHOUT flattening (texture detail
   survives), and everything keeps the detail in the following frame (the light is deleted).
5. **The second-light reciprocal (optional, settles the ERSADD question).** With gdbwatch on the
   VU1 Q/P registers at the point-light program: whether light-1's atten uses P (=1/d1²) or a
   stale Q — the port assumes per-light 1/d². Only matters for two-light vertices.

## Render spots for the picture check

`nf-view <iso> 07000005 --frame 30 --pull 30 --shot` (the P2K shot frame 30):
- The first-person gun/hand: bright (muzzle-coloured, texture detail surviving the clamp — not
  flat white).
- The parapet/courtyard wall ahead: the one-frame glow, NOT the flat saturation the merge flagged.
- A guard in view: lit by the level radiators (0x80 on the skins) — the character lighting the
  user approved, unchanged.
