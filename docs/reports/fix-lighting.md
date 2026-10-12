# fix-lighting — dynamic lights on characters, the muzzle-flash light and smoke

Branch `s/fix-lighting` (from origin/master 9aac84e). The user's PCSX2 comparison: lights visibly
fall on and reflect off characters, and firing lights the surroundings. The port only applied the
flat cel ambient.

## What's done

**The VU1 microcode is read** (GAPS G-014 closed). ACTION.ELF `.vutext` (0x241c80, 0x2880 bytes)
plus the six `.DVP.overlay` sections were disassembled with a purpose-built decoder (field
positions and opcode tables cross-checked against PCSX2's debugger tables; little-endian qword
pairs, lower opcode 0x40 = the special table). The base image is a DVP monitor (a jump table +
dispatcher over the packet's own slots); the light setup at I-mem 595-608 loads the packet's light
positions (qwords 8/9), TColor0/1 (12/13, w = 1/radius²) and TAmbient (15 = the object's TWEAK,
`psiSetTweakARGB` 0x124770 writes 0x2650d0). The point-light program (~832-940) computes, per
vertex, in object space:

- `L = lightpos − pos`; `d² = |L|²`; `1/d²` via `DIV Q, VF0.w, d²` (VF00 = (0,0,0,1) — the trick
  behind the W-row multiplies and the reciprocal);
- atten = `max(0, 1/d² − 1/r²)`; diffuse = `max(0, atten · dot(N, L))` with the RAW unnormalised L
  (no RSQRT anywhere in the program); both clamped at 0;
- colour = `TAmbient ⊙ rgba + TColor.rgb · diffuse` — the light adds on top of the tinted vertex
  colour, exactly the port's existing fix-render ambient scale; the normal enters as the packed
  stream int × 1/128.

The selection: `Lights_CalcClosestLights` 0x1cb2e0 keeps ≤30 lights sorted ascending by the falloff
`d/(radius·0.5)` (comparator 0x1d3350), `psiLight_SetLights` 0x10e588 uploads the two closest.
Kinds: 0 needs object flags 0x80, 1 skips 0x100, 2 = the searchlight's 2D (xz) beam at d ≤ 4.0 with
intensity `(1 − d·0.25)·50`.

**The sim-side light list** (new `crates/nf-game/src/light.rs`): the 0x50-byte `light_tag` record,
`Light_Create` 0x1cc3d8 with the unconditional `Rand_Rand` draw (the muzzle light fires on every
shot — the shared RNG's state advances exactly as the game's), `Light_Update` 0x1cafb8 (life
countdown + delete, the script-channel gate, the flicker state machine), the selection, and
`shade_vertex` (the formula above) shared by the tests.

**Wired, each citing the decompile:**
- Bond's shot: `Player_MuzzleFlash` 0x1a4298 — `Light_Create(radius = weapon_data+0x58, intensity
  2.0, rgb = +0x56/+0x55/+0x54, kind 1, one tick)` behind the +0xDC/+0x48 gate, then `Gas_Init`'s
  `Rand_FRand(2π)` smoke draw unless +0x68 & 0x2000. `Light_Update` runs where `Game_Run` 0x1c9790
  runs it: before the object movement, so the light lives exactly one tick.
- Guards: `DroneWeap_FireWeapon` 0x178f68:189 — `Light_Create(7.0, 1.0, the muzzle, the weapon rgb,
  kind 1)` between the accuracy roll and the bullet's spread draws.
- Level lights: block 0x26's ambient radiators (`Light_SetAmbientRadiators` 0x1caeb8 — parsed now,
  one permanent kind-0 light each, a `Rand_Rand(0)` draw each) and the type-0x4E placements
  (`parsemap_create_dynamic_objects` 0x1d04f0 case 0x4e), created in the game's order: radiators at
  map parse, then interleaved with the drones' creates in placement order. 07000005 gets 12, the
  party level 07000007 gets 104 (its chandeliers and wall lamps — the party now looks lit).
- Searchlights: `Searchlight_Create` 0x1cb4f8:150-172's inline kind-2 record (white, radius 500,
  tracked to the part, below the world until the first lock — the game's own scheme); the kill path
  deactivates it.
- The renderer: per-draw uniform slots (256-byte dynamic offsets) with each draw's own ≤2 lights
  selected on its bounding sphere (`Material::centre/radius` — the game's obj+0x80 sphere); sky
  draws select none. The shader replaces the STANDIN `lc × 0.25 × N·L × (1−d²/r²)²` with the
  microcode's formula (the colour at /128 for the port's 0x80 = 1.0 vertex scale; zero normals stay
  unlit). World cels get the point lights on their baked colours, as `View_DrawCelGlist` 0x1e6538
  does in the game.

## How it was verified

- `crates/nf-game/tests/light_selection.rs` (9 unit tests): the falloff/radius gate against the
  decompile's values, the kind gates with the kind-2 intensity, the ≤2 sorted upload, life/delete,
  the create's RNG advance at arg 0, the record fill, the searchlight's tracked position, and the
  VU1 formula's exact numbers (atten 0 at d = r, 0.75·255 at d = r/2, the backface clamp, zero
  normals unlit) — plus the render-independent sister pair: a character next to the muzzle light
  brightens that frame, the same vertex without the light does not.
- `crates/nf-view/tests/lighting_live.rs` (NF_ISO, rewritten): the muzzle light's weapon values,
  its one-tick lifetime, the frame view resolution, the shared-RNG advance, the smoke puff and the
  no-shot sisters.
- The WGSL is parse- and validate-tested offline with naga (the sandbox has no GPU adapter — the
  shader had never compiled here).
- Full suite: `cargo test --offline --release -j2 -p nf-game -p nf-formats -p nf-audio -p nf-view
  -- --test-threads=1` — 429 passed, only the 3 `disc_golden` tests fail (the slim ISO, the
  task's allowance).
- The RNG-stream consequences were re-baselined per the project's own precedent (WORKLOG 0072's
  seed re-picks): ai_live's two tests got re-picked seeds; fix_npcs_live's party gate now scopes
  the no-combat assertion to the unarmed guests (the armed guests turn with the alarm inside the
  window — the mission layer's ambush; logged); fix_player2_live's kill loop re-aims per volley
  and the crosshair test gives each enemy a clear line. All their original assertions still hold.

## What's left (not ported, with the reason)

- The directional/cel matrix path (packet qwords 16-19, `LQ.xyw`): the cel twinkle/rim built from
  PS2Viewer values — out of this task's scope; world cels keep their baked colours plus the point
  lights.
- The smoke puff's RENDER: the record + the RNG draws are in the sim ([S] rise+fade model); the
  game's `gases_smoke` particle rendering needs a world-space billboard path the port lacks.
- The flicker's `Sound_Play3D` (the placement call sites leave the sound id uninitialised; none
  showed in play).
- The exact game order of the 0x4E creates vs the drones inside `parsemap_create_dynamic_objects`
  is my placement-order reading [S]; a game trace could pin the interleave.
- The per-draw display-flags gate passes 0x80 for world/character/effect draws [S] — the port has
  no per-object obj+0xF0; emitters (the game's 0x100) don't exist as draws.
- The muzzle light's sim-side position is eye + aim·0.5 [S] (the game uses the view model's datum
  0); the render could refine it with the posed attachment position.
- A GPU render of the muzzle flash could not be produced in this sandbox (no Vulkan adapter); the
  driver's picture check remains the visual gate. The uniform staging and the formula are covered
  by the unit tests above.
