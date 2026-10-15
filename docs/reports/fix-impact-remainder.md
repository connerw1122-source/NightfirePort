# fix-impact-remainder — the decal penetration roll and the emitter arm (G-041)

Branch `s/fix-impact-remainder` (from origin/master d4b67a10). The G-041 remainder: the
`Rand_Rand(0x3f)` penetration roll and the `Effect_Create` 0x1ce688 emitter arm — both missing from
the fix-impacts port, both consuming the shared RNG, so every later AI roll after a wall hit
shifted.

## What's done

**1. The penetration roll, fully documented and ported.** [M] evidence:

- `Bullet_CollisionHandler` 0x12ce80 walks the bullet's HITDATA LIST — every front-face hit along
  the tick's segment — and per hit decides PIERCE vs STOP before any effect: weapon id 0x50/0x51
  skip the arm (no draw) [M 0x12cfbc]; the surface byte `& 0x40` forces pierce (no draw) [M
  0x12cfd0]; `& 0x80` draws ONE `Rand_Rand(0x3f)` and ORs bit 0 into pierce [M 0x12cfd8]. The
  surface byte is the level collision blob's per-triangle material byte copied whole to
  `HITDATA+0x50` by `Intersect_RayGeom` 0x10a8c8:326; `Collide_Intersect` 0x1ebaa8:296-300 folds
  0x40/0x80 into `HITDATA+0x51` bit 1, the handler's initial pierce. A pierce keeps the bullet
  alive (`BU+0xF4` = 1 — it already flew the step in `Bullet_update` 0x12e7b0) and takes the NEXT
  HITDATA (its own roll + effects); a stop dies (`BU+0xF0 |= 0x10`) [M 0x12d81c..0x12d82c]. The
  explode arms are gated on NOT piercing [M 0x12d07c]; the ricochet mask is forced to 0 on a pierce
  contact (the roll still draws, never reflects) [M 0x12d6f8].
- Port: `nf_game::effect::penetration_roll` (the exact gates + one draw) and the HITDATA walk in
  `Drones::update_player_bullets` (player bullets; retrace ε 0.01 past each pierced surface — [S],
  the game's list is prebuilt by `Collide_Pick`) and in the drone-bullet `World` event arm
  (same roll + walk, the bullet survives just past the last pierced surface).

**2. The emitter arm, documented and ported.** [M] `Effect_Create` 0x1ce688's flags & 0x20 block
runs FIRST `Emitter_Create(0, 0, EffectInfo+0x84, hit_pos, normal, 0, 0, 0)` 0x17ae00, then the
debris draw [M 0x1ce688:81-99]. `Emitter_Create` consumes NO RNG (the `hashtable_getitem` def
lookup fails → no object); it builds the basis `Vec_MakeOrthonormalBasis(normal)` →
`Mat_SetDir(u)`/`Mat_SetNorm(v)`/`Mat_SetUp(normal)` (rows +0x20/+0x00/+0x10 — the spawn's local +y
rides the normal), latches ONE-SHOT (`+0x4b = rate == 0`, always for impacts). The draws come on
the object's own update ticks (`Emitter_Update` 0x17b080, first run the NEXT frame): 4 ×
`Rand_FRandHalf(1.0)` per spawned particle — yaw, pitch, speed, life, that order [M 0x17b080:216-247]
— batch-capped by def+0x3a, virgin slots only, dead 3 all-dead ticks later.
Port: `EffectRecord.emitter` (+0x84, new parse field in nf-formats::sfx), `WorldFx::spawn_impact`
(a one-shot `EmitterFx`, basis rows [v, normal, u], `shared_rng: true`), and `WorldFx::tick`
consumes the 4 shared-stream draws per spawn from the tick after creation (`age_ticks` gate — the
object is created mid-walk and does not update its creation frame); the 0xf2 placement emitters
KEEP the counter-hash stand-in (fix-world-fx's discipline, untouched). The creation-tick skip and
the all-dead latch interplay is the game's: the burst fires on the first real update, where +0x49
is still 0.

**3. Docs**: `docs/atlas/effect.md` gains "The penetration roll" and "The impact emitter arm"
sections (all addresses above), and the [S] stand-ins list is corrected (both items ported).

## How it was verified

`crates/nf-view/tests/impact_remainder_live.rs` (new, NF_ISO 07000005), 2/2 green:

- `the_roll_follows_the_decompile_rule_for_a_fixed_seed`: for 64 seeds the function draws exactly
  one `Rand_Rand(0x3f)` when the byte carries 0x80 and the pierce equals the roll's bit 0 (the
  reference draw replayed from the same seed); no draw without 0x80; 0x40 forces pierce with no
  draw; weapons 0x50/0x51 never draw.
- `a_wall_shot_draws_the_games_count_and_a_sky_shot_draws_nothing`: a real P2K shot (input path
  `step_bond(fire)`) into the first wall off the 07000005 start. The wall's surface byte is 0x40
  over material 0 — the live run PROVES the pierce: the first surface carries no effects (the
  material-0 record is empty) yet the far surface's stack ran. The sky run (same aim up 60°) draws
  only the fire+flight baseline; the difference — the game's impact count — decomposes exactly as
  (1 debris-count draw + 8·N particle draws + 1 ricochet draw) + 20 burst draws (the far surface's
  emitter def: max 30 × batch 5 → 4·5 draws on the emitter's first update): observed wall 191 − sky
  161 = 30 [D disc records via the port's parse]. Sister assertions: the sky shot spawns no decals,
  no debris, no impact emitters.
- Draw-count observability: `Rng` gained a `draws` counter (additive field, never read by the sim;
  the five existing `Rng { x, y }` literals updated to `..Default::default()` — light.rs,
  light_selection.rs, rng_live.rs, diff_calls.rs, lighting_live.rs).
- The full 4-crate suite on the pushed tree (foreground, `--test-threads=1`): nf-game lib **170
  passed / 0 failed**; nf-game tests **34/1** — the 1 is `ai_live::guard_takes_cover_like_the_game`
  ("UnderCoverAim lasted 90 (live 37)"), reproduced IDENTICALLY on a pristine origin/master worktree
  (same panic, same number) — pre-existing, not this task's flip; nf-formats lib **64/0**;
  nf-formats tests **75/3** — the 3 are the `disc_golden` class allowed on the slim ISO; nf-view
  lib **8/0**; nf-view tests **103/2** — the 2 are the documented no-GPU wgpu `fix_0c_pylon_live`
  pair; nf-audio lib + tests **22/0**. The new `impact_remainder_live` is 2/2 inside the nf-view
  count.

## What's left

- (Moved to How-it-was-verified — the suite DID run on this tree.) On the FINAL MERGED tree (with
  origin/master): the pre-existing `guard_takes_cover` flip and the master merge's own RNG work
  should be re-checked by the merge driver; my tree is clean against master as of d4b67a10.
- The live test's mirror walk (a diagnostic re-trace of the pierce chain from the eye) resolves
  only the FIRST surface on this wall — the port's real walk starts at the bullet's flown position,
  so the far sliver isn't reachable from the origin. The count assertion runs on the observed
  decomposition (shape + the disc records), which is exact but not record-pinned for the far
  surface; pinning it needs the bullet-position mirror (next session, ~30 min).
- The pierce walk's ε 0.01 retrace and the drone-arm's 64-unit far bound are [S] (documented); a
  PCSX2 check on a known penetrable wall would settle both.
- Not ported (unchanged scope): `Effect_Body`/`Effect_Player`/`Effect_Laser`, the bullet trails.

## Live checks for the main agent

- PCSX2, 07000005, stand ~6 units from a thin wall, shoot with the P2K: bullets should pass through
  the level's 0x40-flagged penetrable volumes (material-0 surfaces) and leave the impact effects
  (debris puff + sound) on the surface BEHIND, not on the penetrable one itself.
- nf-view `--bond` spot: the 07000005 start facing the first wall (yaw 0.0, the test's fan k=0):
  the shot should show NO decal on the near plane and the puff/debris at the far surface.
- The impact emitter's sparks: after any wall hit on a material whose EffectInfo+0x84 resolves
  (e.g. the stone/metal walls), a one-shot spark burst flying off the surface along its normal —
  the render side draws them through the existing `particle_sprites` handoff (fix-world-fx's
  Emitter_Draw path A); the main agent's render check owns the look.

## Files

`crates/nf-game/src/effect.rs` (the roll + the emitter arm + the basis pair), `crates/nf-game/src/
world_fx.rs` (spawn_impact, shared_rng/basis/age_ticks, the tick's rng), `crates/nf-game/src/
drone.rs` (both collision callers + fx.tick), `crates/nf-game/src/drone.rs` Rng `draws`,
`crates/nf-formats/src/sfx.rs` (+0x84), `docs/atlas/effect.md`, the new test. Shared-file hooks:
`Rng` literal fixes in `light.rs`/`light_selection.rs`/`rng_live.rs`/`diff_calls.rs`/
`lighting_live.rs` (`..Default::default()`, mechanical). No game-bytes, no credentials committed.
