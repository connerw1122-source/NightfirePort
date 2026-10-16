The user's playtest 2026-10-15 (results id 1015-1738) said: the Delta Repeater (the crossbow): "model is missing the
arrows" and "when you shoot, it sounds like a grenade is bouncing when the crossbow flies out and bounces against the
wall when it shouldn't be bouncing". 1) The bolt's world hit: port what the game does for this weapon's projectile
(stick/die vs ricochet; the bounce sound must not play) [M Bullet_CollisionHandler + weapon_data +0x6C/+0x70
flags]. 2) The missing arrows: find which sub-meshes/datums/attachments carry the loaded bolts on the view model
and why they're absent (a skin entry not merged, a datum attachment not created, a script op not ported). Deliver
the data/attachments; the main agent checks the look against PCSX2.

**Run the crates your tests touch before pushing**, and report what you ran.

**Branch:** `s/delta-repeater`.

- **Run the full suite with the fast runner, started in the background in your first minutes** (it fits the 2-hour window):
  `NF_ISO=<abs path> nohup python3 tools/test/suite.py > suite.out 2>&1 &` (docs/tools/test-suite.md; once `tools/test/suite.py`
  is on your branch: merge `origin/master` first). Read `tail -1 suite.out` and its FAILED/TIMEOUT lines before you push, and
  run it once more on your final tree (merged with `origin/master`). Report the total line.

---
claimed-by: sandbox2 at 2026-10-16T04:06Z

---
## Report (done, 2026-10-16T05:09Z)


---
## Report from the worker (done, 2026-10-16T05:10Z)

# delta-repeater — the Delta Repeater's missing arrows and the grenade-bounce wall hit

Playtest 1015-1738: the Delta Repeater (the crossbow, weapon 0x11) "model is missing the arrows",
and "when you shoot, it sounds like a grenade is bouncing when the crossbow flies out and bounces
against the wall when it shouldn't be bouncing". Branch `s/delta-repeater`. Both fixes landed; the
visual bolt look (slot positions/scale) needs the main agent's render check against PCSX2.

## 1. The bolt's world hit — it is a gravity bullet, not a grenade

**The port's bug:** the shot dispatch in `scene.rs` routed every weapon whose record carries
`weapon_data+0x6C & 0x8` into the GRENADE path (`Sim`'s shot block, the old gate
`if flags6c & 0x8 != 0`). The crossbow's record carries that bit, so its bolt spawned as a
bouncing grenade — the grenade path's wall bounce plus the grenade bounce sounds, exactly the
playtest's symptom.

**The game's structure:** the gravity bit and the grenade-family collision behavior are
independent flags read by different functions:

- `+0x6C & 0x8` = gravity, applied in `Bullet_update` 0x12e7b0 case 1 to ANY bullet
  [M 0x12e7b0: the `uVar19 & 8` arm — `velocity = dir·speed; velocity += WldGravity·(FRAME_RATE_DIV·REC_FRAME_RATE²)`;
  `Vec_NormaliseLen` writes the direction back and stores the length into BU+0xF8; the spawn
  tick, state 0, applies none].
- The wall-bounce block gates on `weapon_data+0x70 & 0x80` [M `Bullet_CollisionHandler` 0x12ce80,
  the `LAB_0012d4bc` gate — the generic-impact sound 0x51c/0x15/0x13 + the velocity reflection];
  the stick arm on `+0x70 & 0x100` [M 0x12d1a0..0x12d1fc — state 3, the matrix aligned to the
  negated hit normal, the thunk sounds 0x4a7/0x446]; the ricochet on `+0x70 & 0x10` +
  `Effect_RicochetProb` [M 0x12d6f8..0x12d7f0]; the contact-explode on `+0x70 & 4`
  [M 0x12d068..0x12d0bc].

**The record** [L ee_start_05.bin @ 0x2bf150 + 0x11·0x10C]:
`+0x6C = 0x00000808` (gravity 0x8, glass-ignore 0x800; NO 0x200 quarter-speed, NO 0x20000 homing,
NO 0x4 camera), `+0x70 = 0x00000002` (the 0x61 effect stack — debris/puff/decal + the
`EffectInfo+0x14` surface impact sound via `Effect_Bullet`'s 0x40 arm; NO 0x4, 0x10, 0x80, 0x100).
Speed `+0x20` = 0.667 u/tick, range `+0x1C` = 120, damage `+0x0C`, clip size `+0x92` = 3, ammo
`+0x90` = 0x0F. A record sweep of all 115 [L]: the grenades/impact family carry the bounce or
explode flags (0x2a 0x828/0x4, 0x2b 0x828/0x280, 0x34–0x36 0x8c08|0x8c0a/0x280|0x2280|0x20280),
the stick family carries 0x100 (0x37/0x3a/0x3b–0x3c, the hat 0x45 0x20c08/0x100).

**So the game's bolt:** flies its lobbed arc under gravity, and on a world hit plays the standard
impact effect (flags 0x61) and DIES — state 2, `Bullet_handle_object_destruction(0, …)` is a
no-op for a world hit [M 0x12cd98: `param_1 != 0` gate]. No bounce, no stick, no ricochet.

**The fix:**
- `scene.rs`'s gate is now `flags6c & 0x8 != 0 && flags70_w & (0x4 | 0x80 | 0x100) != 0` — the
  gravity bit AND a grenade-family collision flag. Record sweep: this moves EXACTLY record 0x11
  off the grenade path; every other gravity record keeps today's routing (0x2a/0x2b/0x34–0x37/
  0x3a–0x3c/0x45 all carry 0x4, 0x80 or 0x100). One caveat: record 0x68 (+0x6C = 0, +0x70 = 0x280)
  stays off the grenade path under the new gate too (it was off under the old one — it has no
  gravity bit); nothing fires it in the port today.
- `PlayerBullet` grew a `gravity: bool` (own line, set from `+0x6C & 0x8` in `fire_player`), and
  `update_player_bullets` runs the game's case-1 arm for it: `vel = dir·speed;
  vel.y += WldGravity.y·(1/60)²; dir = norm(vel); speed = len` — the same integration the
  grenade path already ports (`WLD_GRAVITY` (0, −9.8, 0) [L docs/systems/pickups.md],
  `REC_FRAME_RATE` = 1/60; constants shared via `pub(crate)` in grenade.rs). The spawn tick
  applies none (the arm lives in case 1).
- The crossbow's wall hit then flows through `flush_world_impacts`' existing
  `Bullet_CollisionHandler` walk: the penetration roll, the 0x61 effect stack (flags70 & 2 → the
  surface impact sound via `impact_sound`, the debris/gas/decal), the explode arms skipped (0x4
  clear), the ricochet skipped (0x10 clear) — dead bullet, no revive.

**Verified** (crates/nf-view/tests/delta_repeater_live.rs, through the real give → draw → aim →
trigger path on 07000005):
- `bolt_is_a_gravity_bullet_that_dies_at_the_wall`: the shot spawns a weapon-0x11 PlayerBullet
  (never a grenade — the test panics on one), its dir[1] falls over the flight (the gravity arm),
  the flight ends in exactly ONE `PlayerImpact::World` and a dead bullet, no grenade appears.
- `mgl_still_spawns_a_grenade` (sister): the MGL 0x2a still spawns a grenade — the family routing
  is unchanged.

## 2. The missing arrows — datum-entity attachments of the bolt mesh

**The mechanism** [M `Player_WeaponFiring` 0x1a5460:331-381, the base-0x11 branch, re-issued
every tick]: the view model's datums 4/5/6 are cleared (`AnimDatumSetEntity(vm, d, 0, 0xff)`)
then re-attached: the bolt mesh `0x0200816` — the record's own `+0x5C`, the SAME hash the flying
projectile draws [L record 0x11 +0x5C; the mesh has its own ENTITY_PARAMS object, in 07000014's
map right after the Crossbow_3rd_LOD0] — on datum 4 when count > 0 (NOT while the view state is
9, the fire script's), datum 5 when count > 1, datum 6 when count ≥ 3. The count: clip[0x11]
(the s16 at BLData+0x1B8 + 0x11·0xC); +1 while the view state is 9; once the reload script (view
state 6) passes frame 80.0, min(carried[+0x90 = 0x0F], clip_size +0x92 = 3) [M :345-360]. The
datums live on the first-person skin `+0xDC` = 0x05000083: 4/5/6 on bone 40 at t y
0.17/0.06/0.00, z −0.19 — the three magazine slots top→bottom (plus 0/3 on bone 38)
[D 0700000c.bin/07000014.bin kind-3 skin]. Third person attaches no bolt [M AnimObjectDraw
0x1c4490:256-265 — only the gun model on datum +0x50 = 0x4C], so the playtest report is the
first-person model.

**The port's gap:** `scene.rs`'s view-model build attached datum entities for the sleeves, the
muzzle flash, the AIMS-20 lights and the silencer — no crossbow branch; hash 0x0200816 appeared
nowhere under crates/. (docs/reports/fix-fire-modes3.md already listed the quiver as left-over
view-model work.)

**The fix** (the AIMS-lights pattern — build-time attachments + a per-frame visibility arm):
- Build: for the records whose base (`+0x02`) is 0x11, the bolt mesh (`+0x5C`) is attached to
  the FP skin's datums 4/5/6 (`quiver_bolt: Some(id)` on the new `Attachment` field, own line).
- Per frame (`frame_state`'s attach walk): only the held weapon's quiver draws; count =
  clip[0x11], +1 while `gun.fire_state` (the port's view state 9), `min(carried[ammo_type],
  clip_size)` once `reload_state != 0` and the view script passes frame 80; datum 4 shows when
  count > 0 AND not fire state, datum 5 when > 1, datum 6 when ≥ 3. A non-held model's bolts
  collapse (the game writes the held vm's datums each tick; the port collapses — the AIMS arm's
  pattern, *suggested* for the holstered case).

**Verified:** `quiver_shows_the_loaded_bolts` (07000014, where the FP skin ships): three slots
on datums 4/5/6; clip 3 → all three lit; clip 1 → only datum 4; clip 0 → none; fire state with
clip 2 → count 3, datum 4 suppressed, 5/6 lit; reload past frame 80 → min(carried 2, 3) = 2 →
datums 4/5; reload before frame 80 → the clip's count.

## What the main agent should check (PCSX2 render)

- `--level 07000014` (the crossbow starts in hand, 3 loaded): the view model should show THREE
  bolts stacked in the magazine under the barrel (datums 4/5/6 on bone 40, t y 0.17/0.06/0.00,
  z −0.19 on the FP skin 0x05000083). Fire down to 2/1/0 rounds and reload: the stack should
  lose/refill bolts top→bottom, with the top slot dark while the fire script plays.
- The bolt's wall hit: a normal impact puff + decal + the surface impact sound, NO bounce, NO
  grenade sound, the bolt disappears.

## Live checks for the main agent (optional)

- L-1: `gdbwatch` the view model's datum-4/5/6 entity hash (`BL+0x7E8` anim obj, the per-datum
  entity array anim+0xA4) while firing the crossbow down from 3 rounds: expect 0x0200816 on the
  datums while count covers them, cleared at state 9 on datum 4 only.
- L-2: the stuck-bolt question is settled by data (no stick bit), but if PCSX2 shows a bolt
  REMAINING in a wall, check the bolt object's `obj+0xF4` after a wall hit — the port's data
  says state 2 (deleted); a state 3 would contradict the +0x70 = 0x2 read.

## Tests and what ran

- New: `crates/nf-view/tests/delta_repeater_live.rs` (3 tests, all green with NF_ISO).
- Updated: `fire_modes3_live.rs`'s `crossbow_mode_press_is_inert_and_fires_single` pinned the OLD
  model ("one pull fires one gravity bolt (a grenade-path projectile, not a straight bullet)") —
  the assertion now checks the corrected record data (gravity bit set, no family flag) and that a
  pull fires one shot with NO grenade.
- Touched crates, foreground (this sandbox reaps background jobs between tool calls, so the
  `tools/test/suite.py` runner cannot survive a run here — the drive-mission-scripts precedent,
  flagged in its report too):
  - `cargo test --offline --release -j2 -p nf-game -- --test-threads=1`: **440 passed, 0 failed**
    (includes the grenade family's unit tests — their routing is untouched).
  - nf-view, the tests on the touched paths: delta_repeater_live 3/3, fix_weapon_fire_modes_live
    5/5 (the MGL pair), fire_modes3_live 5/5 (+4 ignored, pre-existing harness skips),
    fix_grenade_release_live 4/4, fix_flesh_impacts_live 4/4, fix_guard_misc_live 6/6,
    fix_aims20_lights_render 1/1, fix_aims20_modes_lights_live 3/3, fix_silencer_visual_live 2/2,
    fix_start_loadouts_live 6/6 (the crossbow kit), weapon_assets_live 5/5 — **44 passed,
    0 failed**.
- nf-formats/nf-audio are untouched by this task; no final-tree full-suite line is available for
  the reason above — the touched-crates evidence stands in for it.

## Left

- The bolt VISUAL (the quiver's exact look, the flying projectile's mesh draw) is the main
  agent's render check — no GPU here.
- The holstered quiver state is a port-side choice (collapse); PCSX2 may keep the last state —
  cosmetic either way, flagged above.
- Record 0x68 (+0x70 = 0x280 with no gravity bit) never routes to the grenade path under either
  gate; nothing fires it today. If a future task fires it, it needs its own look.
