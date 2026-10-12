# models-misc — report (044, branch s/models-misc)

## What's done

Every remaining model class mapped from the decompile with creator-level evidence; the missing draws
ported; the per-level census pinned as a golden. `docs/models/misc.md` is the lane inventory (per kind:
creator address, disc counts, drawn-by-the-game verdict, port status, remaining mechanism, --bond spots).

1. **The static dynamic entities** — 16 placement kinds whose creators attach the placement's glist and
   never move `obj+0x90` now draw as static World instances at the placement transform (scene.rs
   `misc_static_sweep`, counted in `Scene::misc_drawn`): locks 0x2E (24), monitors 0x30 (17), gun-imp
   parents 0x34, PCQ worms 0x35, grapples 0x3A (57), dynamic objects 0x3B (visible ones), creep walls
   0x3F, rotors 0x71, one-sided 0xD5, rigid bodies 0xD8 (35), destroyables 0xE1 (187), flickers 0xE3,
   cars 0xDA + `Car_InitBits`' fixed parts, third icons 0xFA / cam subjects 0xFD / Apocalypses 0x4A
   (mesh-ful only). Equally important — the kinds the game does NOT draw, now asserted never to draw:
   switches 0x29 and fuseboxes 0x31 (no mesh ever attached [M 0x1adcc0:35, 0x1b0448:25]), gas traps 0x27
   (hidden at create [M 0x17ffa0:78]), rainboxes 0x32 (mesh never drawn, only the AABB read [M
   0x18d0d0]), space laser/missile 0x38/0x39, emitters 0xF2, clouds 0xD4 (sky pipeline), subs 0x36/0x37
   (no placements on any on-foot main map [D]).
2. **The LOD block (map 0x27)** parsed (`Map::lod`, `lod_target`) — `Destroy_Smash` 0x12bb60's swap
   glist (`celglist+0x40`, arg proven by disasm 0x12bbb8). Fixed the merge collision: models-world's
   `id::LOD = 0x1f` was the second entity-params id (no 0x1f block exists on disc); renamed
   `ENTITY_PARAMS_2`, their probe now reads the real blocks.
3. **The sky clouds** (0xD4 → `View_AddSkyObj`, slot = param 4 [M 0x18d620:27]) ride `sky_instances`;
   the 0x2A lightning-variant flag no longer misapplied to them.
4. **The gas/smoke system** (nf-game/src/gas.rs): `Gas_Init` 0x1aacc8 / `gases_smoke` 0x1ab2d0 types
   0/4/7, one shared `smoke_1_dust` 0x0200039 quad, billboarded per frame with the live rgba. Bond's
   muzzle smoke (`Player_MuzzleFlash` 0x1a4298:91-103, the 0x2000 skip honoured) and the
   broken-searchlight smoke (0x1cb900:373-385, every 7th frame) spawn; the puff draws, fades and dies.
5. **The explosion visual**: a blast spawns the one-shot 0x6000052 script player at scale
   radius·0.06666667 (`Explode_Create` 0x17cbd0:84-106) and the scene draws its ~40 keyframed glist
   entities (clouds, bursts, the 16 `Blast_frame_N`, sparks, `Blast_radius_ball`) at their live matrices.
6. **The front end measured**: 07000048 has no 3D placements (291 textures + one meshless fusebox); its
   visuals are the kind-8 menu-def sprite pipeline — a separate task, facts asserted in the test.
7. **nf-extract kind-census**: the per-archive placement-kind census tool used for all the counts above.

## How it was verified

- `crates/nf-view/tests/models_misc_live.rs` (10 tests, all green):
  - the census test recomputes every level's expected draw set from the placements independently and
    compares with the sweep (fail-first shown: sweep disabled → `{}` vs the expected map; cars 0 vs 12);
  - spot checks against the disc (0c: 5 locks/5 grapples; 0d: 9 destroyables/1 gun imp; 14: 22 visible
    of 45 dynamic objects — 23 hidden until their channel [D placements; M 0x18dbf8:31]);
  - the meshless-kinds guard (with a sister proving the levels really carry those placements);
  - car parts resolve + draw (12 = 4 × body+2); clouds in the sky pass; LOD lookups (010000cf's
    6→7/9→11, −1 and out-of-range → None); the front-end facts;
  - the smoke tests through the real input path (pulsed trigger): the puff carries the game's
    parameters (0.05→0.1 scale, type 7, rgba 0x48403080), fades and dies; a broken searchlight smokes
    every 7th tick; the pool deterministic and capped (gas.rs unit tests);
  - the explosion test: the blast through the sim's real explode path spawns the scaled script player,
    the blast meshes resolve with zero REF skips, and live entity matrices answer through
    `script_entity`;
  - the 34-archive census golden (numbers+SHA-1 only, no bytes).
- The full suite (`--no-fail-fast`): 103 test binaries, 102 green, only the 3 allowed `disc_golden`
  failures (slim ISO). models_world_live and models_characters_live green after the LOD-id fix.

## Shared files touched (additive, minimal)

- `crates/nf-view/src/scene.rs`: the misc sweep block, the gas slot attachments, `Attachment.gas` +
  `Scene.misc_drawn` (own lines), the explosion hash added to the script-entity scan, the muzzle smoke
  spawn in the fire path.
- `crates/nf-formats/src/level.rs`: block 0x27 parse, `lod_target`, the 0xD4 sky clouds.
- `crates/nf-formats/src/map.rs`: `LOD = 0x27` (models-world's `0x1f` renamed `ENTITY_PARAMS_2` — see
  the merge note above), `pub mod gas` in nf-game/lib.rs.
- `crates/nf-game/src/drone.rs`: `Drones.gas` (own line), its tick update, the searchlight smoke spawn,
  the blast's script spawn.
- `crates/nf-extract/src/main.rs`: the kind-census subcommand.

## What's left (mechanisms documented in docs/models/misc.md)

The lightning (0x2C chains + `FractalThing` + the sky-slot flash driver — fully measured, unported);
the breakables' death (per-triangle HEINFO debris + the glass shard pool — fully measured, unported);
the Destroy swap wiring when a variant exists; the GunImp turret child (needs the ELF TurretGuns
table); rotor/RB/Apocalypse motion once disturbed; the car MP variant; trees (0xF3) via the anim-object
path; the blast's `Mat_Align2Dir` tilt; overlapping blasts share the newest entity set.

## Live checks for the main agent

1. `--bond 7.1,16.5,-112.37,0,0` on 0700000c — the 5 safe locks stacked by the gate (were invisible).
2. `--bond 47.5,3.2,36.8,0,0` on 07000014 — the security monitors + visible dynamic objects.
3. `--bond 35.6,45.2,17.5,0,0` on 07000015 — the 9 mines.
4. `--bond 8.4,2.8,86.0,0,0` on 07000024 — an MP car: body + turret + wheel parts.
5. `--bond -55.4,-12.2,0,0,0` on 0700001b — the 8 Apocalypse glist meshes.
6. Fire the P2K anywhere — a brief smoke wisp drifts from the muzzle; shoot a 07000005 searchlight —
   break smoke every 7th tick. Throw a grenade — the fireball script entities + smoke.
7. Confirm the switches/fuseboxes/rainboxes did NOT start drawing (the game draws none of them).
