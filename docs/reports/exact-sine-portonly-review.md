# exact-sine-portonly-review — the G-046 PORT_ONLY trig stand-ins (110y)

Task: G-046's four PORT_ONLY sites — places where the port uses trig and the game has none. For each: find what
the game really does (decompile, with addresses), replace the port's construction with the game's mechanism, or
prove the stand-in equivalent with a test. Every fix fail-first through the real input path; the suite stays green.

## Done

### 1. `casing.rs` eject matrix (the guards' yaw/pitch basis) — REPLACED with the game's mechanism

The game's guard arm of `Bullet_init_casing_ex` 0x12f678:102-114 [M]: `AnimDatumGetWeaponInfo` reads the weapon's
`+0x50` datum byte (0xFF → the game returns and no shell ever appears), then `AnimDatumGetIndex(datum+2)` 0x1c77f8
gates the eject port: when the skin's attachment table carries datum id `base+2` the game calls
`AnimGetBoneWorldTrans(obj, datum, 2)` on it, else it falls back to `AnimGetBoneWorldTrans(obj, datum, 1)` — the
muzzle datum (`DroneWeap_FireWeapon` 0x178f68:154's bullet origin). The returned datum matrix supplies both the
eject origin (`Vec_Copy2` of the VECTOR out, 0x12f678:160) and the velocity basis (`ApplyMatrixLVI` of the
weapon-local `(+0xf8, −0xfc, +0x100)` through the rotation rows, :120), plus 3× `Rand_FRand_MVar2(0.01, 0.005)`
velocity jitter [M 0x12f860..]. The port's `Casings::eject` built the basis from the SHOT direction's yaw/pitch
with host trig instead — a construction the game does not have.

Fix (`drone.rs`, additive): `Drone::eject_matrix(d)` — the same `QuatTransToMat(q,t) ∘ palette[bone]` composition
`weapon_matrix`/`muzzle` already use, selecting datum `base+2` first with the `base+1` fallback (`Skin.datums`
stands in for the game's skin attachment table, `Drone::palette` for `psiBuildMatrixPalette`); the guard fire call
site passes it to the already-game-faithful `Casings::eject_bone` (the player path's merged mechanism). The 7 RNG
draws stay at the same point in the stream (eject_bone's order is unchanged); the 0xFF early return matches the
game's no-eject.

Test `fix_portonly_review_live::guard_casing_ejects_from_the_weapon_datum_matrix` (NF_ISO, level 07000005, the
SG5 guard at (61.34, 99.50), real `Drones::tick` path): the shell's origin equals the eject datum's world position
and its velocity equals that matrix's rows on the weapon-local eject vector ± the MVar2 jitter bounds. Sister:
a 90°-rotated basis must not reproduce the datum eject vector (and the datum basis is asserted to differ from a
body-yaw basis by > 0.05, so the test has teeth). FAIL-FIRST shown: with the call site reverted to the stand-in
the test fails on the origin/velocity asserts.

### 2. `drone.rs` object-space head-track (2495) — PROVEN EQUIVALENT, and the game's gates added

The game measures the head-track ease targets in WORLD space (`AnimObjectHeadTrack` 0x1bf618:189-281: two
plane-projection acos blocks on the world palette's axes — neck = normalize(head − neck-base), side =
neck×fwd, fwd = the head's row 2; no sin/cos of any object yaw). The port measures in OBJECT space: it un-rotates
the target by −yaw with host sin/cos because its pose is object-space. Review verdict: **equivalent** — the
object transform is a rigid yaw+translation (`palette()`'s `object_matrix([0,yaw,0], pos)`; the un-rotation is
exactly the inverse of its rotation block), and every dot/cross/acos/determinant-sign in the game's blocks is
rotation-invariant, so both measurements agree including the sign conventions (yaw positive right about the neck;
the game's positive-down pitch negated into the port's positive-up).

Test `head_track_want_matches_the_game_s_world_space_measurement`: for five non-degenerate targets around a fresh
guard's head, the port's `head_track_want` equals a direct transcription of 0x1bf618's two acos blocks run on the
world palette (1e-3; near-zero-yaw targets are sign-degenerate for the cross-of-parallel-vectors in the game's own
formula, so the comparison there is magnitude-only). Sister: 45° right ≈ +π/4 and the 45° left target mirrors it.

While transcribing the game's blocks, two real divergences surfaced and were fixed (fail-first:
`head_track_pitch_stays_level_for_behind_targets` — an above-behind target must want pitch 0; pre-fix the port
measured it and clamped):
- **per-plane gates** [M 0x1bf618:209-210, 257-258]: the game gates each PLANE-projected distance at 0.25
  separately and leaves the failed angle 0; the port used one full-3D `tv² < 0.25` gate and always measured both.
- **the behind skip** [M 0x1bf618:222-236]: `dot(unit fwd, unit toTarget) < 0` skips the pitch measurement
  (the head returns to level); the port measured behind targets and clamped them.
The one-plane-passed coupling (:284-289) is subsumed by the failed block's own 0.

### 3. `ladder.rs` mesh columns (151) — mechanism identified; replacement blocked by the port's data model

The game's ladder pick has NO trig and NO analytic column [M]: stage 1 is the ×1.5-sphere broad phase
(`Collide_PickObj` 0x1eae80:152-168 — the port's stage 1 is equivalent), stage 2 is a capsule vs the ladder's
AUTHORED COLLISION TRIANGLES in the ladder's stored-matrix local space (`Collide_Update` 0x1ea398 →
`Collide_Intersect` 0x1ebaa8:276-296 inverses the object matrix, `RotPreTransVec`es the segment, then
`Intersect_CylGeom` 0x109da0: AABB ±(r+0.1), box-tree cull, per-triangle closest point, one-sided near-vertical
cull at :321-322). The port's `in_mesh_column` (frontal/lateral decomposition via `yaw.sin_cos()`) is a stand-in
for that mesh test; the 0.4-lateral / capsule_radius-frontal numbers are the port's invention. Also documented
from the same research (adjacent to the trig site, NOT changed — it is playtest-tuned by fix-ladder-water2):
the decompile's capsule arms are substate−1 keyed — case 0/6 = CLIMB/creep (0.275 tall), case 3 = CROUCH (low
0.55), and WALK and SWIM both take the DEFAULT arm (0.55, A = pos+up·0.275, B = pos+up·(0.55−cc)) — the port's
walk-0.275 / swim-low mapping approximates outcomes, not mechanism.

Replacement attempted and blocked: the fix would run the ladder's `world.shape(hash)` triangles (the port already
implements `Intersect_CylGeom` as `capsule_push_scoped`) through a boolean capsule test. Probe on 07000005 (the
only level-05 ladder): the placement resolves to a map object whose hashcode is 0xFFFFFFFF and `world.shape()`
has no entry — the port's collision world keys shapes by hashcode, so the ladder's celglist is unreachable
without a data-model extension (map-object-keyed glists). Not guessed at further in this window; the stand-in
stays with this divergence on record.

### 4. `player.rs` crawl advance (858) — mechanism identified; the game's rate is readable, extraction not finished

The game's crawl displacement rides the crawl anim scripts' root motion [M — `Player_Creep` 0x1969d0 has no
positional advance; the survey's pointer]. Probe result: the scripts ARE in the level bins — 07000005 carries
0x600046d/0x600046f (40-frame, non-looping: the crawl entries/exits), **0x6000471/0x6000475 (90-frame LOOPING:
the left/right locomotion cycles)** and 0x6000476/0x600046c; each script's opcode-0 command resolves to sequences
0x040003f0 / 0x040003e5 (222 channels, skeleton record 0). The root tracks are readable with the port's own
`nf_formats::anim::local_pose_bones` machinery (bone 0's channels per the skeleton's translation mask, scaled by
`skin.scale`), but finishing the extraction needs bond's skeleton-0 skeleton+skin from the level data and the
root-channel order decoded against a PCSX2 snapshot — not done in this window (I probed the channel
polynomials directly; the degree-5 coefficients' scale needs the engine's own evaluation path, not guessed).
The port's 0.02/tick stand-in [S] stays; the follow-up task is: extract bone 0's per-frame translation from
0x040003f0/0x040003e5 with bond's skin, compare against 0.02/tick, and replace the stand-in with the game's rate
(the anim is a table — the game's own mechanism) or fix it if they differ.

## Files touched

- `crates/nf-game/src/drone.rs` (a named task file): `Drone::eject_matrix` added; the guard casing call site
  switched to it; `head_track_want` re-gated (per-plane gates + behind skip) and made `pub` for the equivalence
  test. No other shared file needed changes (`casing.rs` itself untouched — `eject_bone` already existed;
  `ladder.rs`/`player.rs` unchanged — no fix was possible without the blocked data work).
- `crates/nf-game/tests/fix_portonly_review_live.rs` (new): the three tests above.
- `docs/GAPS.md` (G-046 reviewed), `docs/WORKLOG.md` (appended), this report.

## Suite

Final tree (merged with origin/master a2958c03), fast runner foreground chunks (`--test-threads=1`, NF_ISO,
release): chunk 1 — ai_live, fix_ai_remainder_live (the head-track live tests), trig_live, fix_portonly_review_live,
tuning_live, fire_modes3_live, fix_weapon_fire_modes_live, weapon_switch_live, fix_weapons_all_live,
fix_samurai_fire_live: **65 passed, 0 failed, 4 ignored**. Chunk 2 — fix_sg5_casings_live, fix_shell_eject_live,
casings16_live, impacts_live, head_look_live, ai_traces_live, fix_weapon_effects_live, anim_events_live,
fix_searchlight_remainder_live, drone_portcheck_live, exact_vs_host_probe: **37 passed, 0 failed, 1 ignored**.
Total 102 passed / 0 failed / 5 ignored over the affected binaries; the full 4-crate suite did not fit the
session's window (the sandbox reaps background jobs — the chunks above cover every binary that touches the
changed code paths); the slim-ISO disc_golden trio is unaffected by this task's files.

## Live checks for the main agent

PCSX2 (07000005, the corridor with the SG5 guard, or any level with a visible guard):
1. **Guard shell eject basis**: stand in front of a firing guard, then above/below him on a staircase. Expected
   (game, now ported): the shell flies along the WEAPON MODEL's eject port orientation (the posed gun datum),
   not exactly along the shot direction — the shell's trajectory should not change when the guard's aim pitch
   saturates but the gun pose does not. nf-view `--bond` spot at a firing guard: the shell's first-frame velocity
   ≈ the gun datum matrix · (weapon_data+0xf8.., −0xfc.., +0x100..).
2. **Head-track behind behavior**: stand BEHIND a guard (his back turned) within 3 units, slightly above his
   head. Expected (game, now ported): the head does NOT bend up/back to the pitch clamp — with the target behind
   his head's forward, the pitch measurement is skipped and the head stays level; yaw may still track around.
3. **Crawl speed** (for the crawl stand-in): enter a crawl wall (07000005's vent) and hold forward. Measure with
   `fieldprobe.py` on Bond's position (obj+0x30) while the locomotion cycle plays: expected from the anim data
   read — the 90-frame looping cycle (1.5 s at 60 Hz) carries the root along the crawl axis; the measured
   units/tick gives the rate the port's 0.02/tick stand-in should match (my unfinished extraction says the root
   translation track is present and non-zero).
4. **Ladder grab width** (for the ladder column stand-in): strafe into a ladder from the side at increasing
   offsets and record where the grab stops triggering. Expected (game): the boundary follows the authored
   rails/rungs geometry (the celglist triangles), not a uniform ±(0.4 + capsule_radius) corridor about the
   ladder axis — if the grab boundary hugs the geometry unevenly, the port's analytic column diverges there.
