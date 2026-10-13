# fix-impacts-render — the impact effects draw the game's own meshes

Branch `s/fix-impacts-render` (master 78d73e0). Task 111. Started 12:34 UTC, sandbox expires
~14:33 UTC; this report written 13:45 UTC.

## What's done

The flat-colour placeholder is gone. `Sim::effect_quads` (fix-impacts' stand-in, drawn in the snow
pass) is removed; `snow_for` is back to snow + emitter particles only, matching the game's pass
order. The decals, casings and debris now draw as the game's own meshes:

- **The draw mechanism** (`crates/nf-view/src/scene.rs`): research showed there is no dedicated
  effect draw in ACTION.ELF — decals are sub-object nodes drawn by `View_DrawObjectGlist` 0x1e6780 /
  `View_DrawCelGlist` 0x1e6538 (`psiDrawObjectMatrix(node+0x10, node+0x20)`), casings and debris are
  ordinary class-0x40 objects drawn through `Control_GetObjMatrix` 0x134210's cached `obj+0x90`
  (euler@+0x50 · pos@+0x30, `Mat_Scale3f(obj+0xe8)` applied once; the & 8 billboard and & 4 jitter
  paths don't apply to them — checked in the decompile, 0x1e6780:47-65). All three render in the
  game's pass 0. The port instances every distinct hash the level archive resolves as Effect-layer
  pools — **200 decal slots** (the FIFO size, `DList_Init(0x3496e0, 0x60, 200)` 0x1cf268),
  **32 casing slots** (`nf_game::casing::POOL`, `Bullet_InitCasingLists` 0x12fd48), **128 debris
  slots** (unbounded in the game; ≤ 6 per impact, ≤ ~54 ticks) — one `add_instance` per (mesh,
  slot) with a new `Attachment::effect: Some((kind, slot, mesh))` (kind 0 decal / 1 casing /
  2 debris; added on its own line at the end of the struct per the merge rule, 11 existing
  literals got `effect: None`).
- **Per-frame posing** (`frame_state`): the decal slot = `align2dir(normal, (0,1,0), (1,0,0))` —
  the same construction as the game's `Mat_Align2Dir` 0x1df2a8 with the same up vectors
  (0x2d6210/0x2d6220) — at the stored `hit + normal·0.01` pose, no scale (the quad size lives in
  the mesh [M 0x1e6780]); the casing slot = euler (eject dir's spherical angles + spin·age) at the
  integrated pos, scale 1; the debris slot = euler (spin·age) at pos with the rows scaled by
  `obj+0xe8` (0.4 + FRand(0.1), single application). Debris age = 255 − alpha — the alpha byte
  starts 0xff at `Debris_Create` 0x1aab10 and steps down 1/tick in `Debris_Update` 0x1aac00 [M], an
  exact tick count with **no nf-game change**. A slot draws only while its live effect carries the
  slot's mesh (the game's glist is set from the hash at create); free slots collapse far below the
  world like the tracers.
- **State rides the disc data**: the pools are ordinary materials, so each decal/debris/casing mesh
  keeps its own leaf GS state — blend/atest/zwrite from the disc glist (the game bakes the decal
  blend/z in the glist packets too); depth-tested via the pass-5 pipeline (compare LessEqual, write
  from ZBUF_1). The pools sit in the Effect layer, which only skips the static-World frustum/cel
  cull (the build-time centre can't cover 200 slots spread over the level — the same reason
  tracers/grenades are there).
- **Casing euler reconstruction** (`casing_matrix`, [S]): the game eulers the eject direction from
  `vecutil_cartesian_to_spherical_acc` through the view-model bone matrix; the port reconstructs the
  eject direction by undoing the tick gravity from the current velocity and takes the tracer's +Z
  euler convention — the tumble itself (spin·age) is the game's. Listed below.

`docs/atlas/effect.md` written (the impact path, its addresses, the draw mapping and the [S]
stand-ins — the entry fix-impacts owed). GAPS G-041 and G-025 rows updated to the post-commit
state (those two rows only; `git diff` = 2 rows).

## How it was verified

- **The failing-first step**: the rewritten NF_ISO tests were run against the scaffolded tree
  (attachment field + accessor, no pools) — both new assertions failed exactly as expected
  (`the decal mesh's draw slots are in the scene` / `the casing mesh's draw slots are in the
  scene`), then passed after the pools + posing landed.
- **NF_ISO tests** (`crates/nf-view/tests/impacts_live.rs`, 07000005, through `nf_view::load` +
  `Sim::step_bond`'s real input path — 4/4 green):
  1. `a_shot_at_a_wall_leaves_a_decal_a_sky_shot_leaves_none` — the wall decal's `EffectInfo+4`
     mesh has draw slots; the slot's frame matrix sits at the stored decal pose (±0.02) and its +Z
     row is the surface normal (dot > 0.99 — the `Mat_Align2Dir` orientation); the debris pool
     exists for EffectInfo's debris hash and a live particle's slot matrix tracks its pose scaled
     by `obj+0xe8`. Sister: the sky shot leaves no decal.
  2. `the_casing_draw_slot_carries_the_p2k_mesh_and_tracks_the_pose` — the P2K (weapon 2, ammo
     byte +0x90 = 1) ejects a casing whose mesh is `ammo_data[1]` = 0x0200043f, the scene has that
     mesh's slots, and the slot matrix tracks the integrated casing pose (±0.05). Sister in
     `each_shot_ejects_one_casing_that_lands_within_its_lifetime` (unchanged, still green).
  3. `the_seeded_run_reproduces_bit_for_bit` — unchanged, still green (the pools consume no RNG).
- **Full suite** (`NF_ISO=… cargo test --offline --release -j2 --no-fail-fast -p nf-game
  -p nf-formats -p nf-audio -p nf-view -- --test-threads=1`, split into two runs to fit the tool
  timeout — everything through `walkreal_07000001_live` in run 1, the rest in run 2): only the 3
  allowed `disc_golden` failures; everything else green, including nf-game's 31-binary AI/live
  suites.
- **GPU**: none in this sandbox (`No suitable graphics adapter found` from wgpu) — the main agent's
  render pass owns the visual check.

## Render spots for the main agent (no GPU here)

1. `07000005 --bond 105.9,26.3,108.1,2.0,0` facing the start corridor wall; fire once at the wall
   (the sim fires through the normal input path). Expect: a small dark bullet-hole mark 0.01 off
   the plaster (mesh DefaultBulletHole 0x02000126), correctly oriented in the wall plane, no
   z-fighting, no depth-write halo; a few plaster chips (spark plaster 0x0200012c, ~0.4–0.5 scale,
   tumbling) arcing off the wall; one brass casing (0x0200043f) ejecting right of the muzzle,
   tumbling, landing ~1–1.5 below the muzzle.
2. Fire 5–6 times at the same wall spot: marks accumulate (distinct quads, no duplicates), the
   oldest evicts after 200.
3. Shoot metal/wood/glass surfaces elsewhere in the level: the mark mesh changes per material
   (MetalBulletHole 0x02000127 / WoodBulletHole 0x02000128 / GlassBulletHole 0x02000122).
4. Check the debris scale/alpha: chips should be roughly half a wall-puff's size, spinning, and
   vanish within ~1 s without fading out (the game applies no per-particle alpha at draw).
5. Watch a casing in flight: euler spin should look right; the initial pose is a listed [S]
   (below) — if the casing visibly starts mis-oriented but tumbles correctly, that's the stand-in.

## What's left / follow-ups

- The eject origin/basis and initial casing euler are [S] stand-ins (the game reads
  `AnimGetBoneWorldTrans`); live-check where the shell appears relative to the gun.
- The pools draw in the Effect layer (pass 5, fog 0) rather than the game's pass 0 — equal for the
  marks, off for heavy distance fog; a pass-0 move would need a per-layer pass override.
- No marks on moving hosts (dynamic-host decals are not modelled — the sim keeps only the
  world-space result); `Effect_Create`'s penetration roll and emitter arm, `Effect_Body/Player/
  Laser` and the bullet trails remain unported (out of budget, unchanged from fix-impacts).
- Casing meshes not in the level archive (e.g. the default shell 0x02000548 on 07000005) don't
  draw there; the session-merge (`merge_weapon_models`) could cover `ammo_data` casing hashes the
  same way it covers weapon models if a level needs it.

## Live checks for the main agent

1. PCSX2, P2K against a plaster wall: does the shell appear right of the muzzle and land ~1–1.5
   below it (the port's muzzle-origin stand-in vs the bone transform)?
2. Do the game's wall marks render at the exact hit point with no offset along the normal beyond
   0.01 (the port matches by construction; a bigger live offset would mean a HITDATA store this
   port missed)?
3. Debris: confirm the pieces are small (≈0.4–0.5 scale), tumble, and do not fade — just vanish.

## ntfy failed

(none — sent at finish.)
