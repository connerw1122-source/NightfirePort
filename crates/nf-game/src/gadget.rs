//! Bond's gadgets (weapons 0x4A..0x5E): the laser-family beam and its lock burn.
//!
//! The game's model [M]:
//! - `Player_LaserPointer` (0x1a65d8): while a laser weapon is out (its `weapon_data+0x68` gate and
//!   `BLData+0x964`), every tick casts a ray from `Player_GetHeadPos` through the hand bone's
//!   direction, 100.0 units (the 0x42c80000 constant at 0x1a6748), and parks the beam object
//!   (`BLData+0x7ec`) at the hand with its far end at the ray hit (`Collide_RayIntersect` 0x1ec690;
//!   no hit → the full 100). The beam mesh swaps 0x2000131 / 0x2000133 per frame (the flicker).
//! - The beam carries a WEAPON ID at `obj+0x44`: `Lock_Update` (0x1ae3d8 case 0) scans the lock's
//!   contact list for class-5 entries whose +0x44 is the Q-Decryptor (0x4E/0x4F, burn 3/tick) or in
//!   the `− 0x56 < 2` band (0x56/0x57 — the watch laser's and the Q-Laser's beams), and advances the
//!   lock's burn timer; at the end `Lock_Unlock` writes the lock's out channel.
//! - The port's `Lock::burn` (fix-objects2) models the burn timer as already expired (the level's
//!   create-time base is 0 [M 0x1ae1f8]), so one tick of beam contact completes it.
//!
//! Port status: the laser-family beam (the ray, the lock burn, the endpoints for the renderer). The
//! grapple (substate 2) and the decryptor scan mode (substate 5) are not started.

use crate::lock::Lock;
use crate::triggers::Channels;
use crate::world::V3;

/// The weapon ids whose shots carry the LASER BEAM bit `weapon_data+0x6C & 0x100` [M
/// Player_MuzzleFlash 0x1a4298:131 → `Draw_LaserBeam`]: the Laser 0x4E and its upgrade 0x4F
/// [D weapon_data records: 0x4E +0x6C = 0x900 (0x800|0x100), 0x4F = 0x900; the disc names them
/// 0x05000043 "Laser" [L USATxt.dat]. The old set (0x50/0x51/0x54/0x55) was the Grapple and the
/// Micro-Camera — their records carry 0x4000/0 in +0x6C, no beam bits].
pub fn is_laser_weapon(w: u8) -> bool {
    matches!(w, 0x4e | 0x4f)
}

/// The weapons whose SHOT draws a beam: the Laser family (the 0x100 bit → `Draw_LaserBeam`) and
/// the Stunner 0x4A/0x4C (the `weapon_data+0x6C & 0x80` bit → `Draw_TaserBeam` [M 0x1a4298:134];
/// their records: +0x6C = 0x880 [L], the disc name 0x05000042 "Stunner" [L USATxt.dat]).
pub fn runs_beam(w: u8) -> bool {
    is_laser_weapon(w) || matches!(w, 0x4a | 0x4c)
}

/// One tick of the laser beam: the ray from `head` along `dir` (unit), 100.0 units [M 0x1a65d8],
/// against the world segment and the locks' world spheres. A lock whose sphere the ray touches
/// (the ray's closest point within its radius — the game's contact-list stand-in) burns; its out
/// channel is written on completion [M 0x1ae3d8:194-195].
/// `head`/`dir` are THE CROSSHAIR RAY the caller fires bullets down (the game's shot ray runs from
/// `Player_GetHeadPos` through `Player_GetAimingPoint` [M Player_WeaponInitBullet 0x1a4bf0:152-159],
/// and the beam draws from the muzzle to that ray's re-cast hit [M 0x1a4298:107-132]) — the old
/// `bond.look()` ray was the user's "the laser isn't accurate" on the 0700000c display vent.
/// Returns the beam's endpoints (the hand end is the caller's; here the head) and the hit point
/// for the renderer.
pub fn laser_tick(
    head: V3,
    dir: V3,
    locks: &mut [Lock],
    world: &crate::world::CollisionWorld,
    ch: &mut Channels,
    frame: u32,
) -> (V3, V3) {
    let len = 100.0f32;
    let end = [head[0] + dir[0] * len, head[1] + dir[1] * len, head[2] + dir[2] * len];
    // The world hit caps the beam's visual end (Collide_RayIntersect 0x1ec690's first-hit t).
    let hit_t = world.segment(head, end).unwrap_or(1.0);
    let hit = [head[0] + dir[0] * len * hit_t, head[1] + dir[1] * len * hit_t, head[2] + dir[2] * len * hit_t];
    // The lock contact: the game reads the beam OBJECT out of the lock's contact list [M 0x1ae3d8
    // case 0]. The port's stand-in uses the AIM LINE's closest approach to the lock's centre — on
    // 07000008 the safe door's collision plane sits ~0.9 in front of the bolt centres [D the probe
    // grid: every standpoint's world-capped beam lands ≥ 0.88 from a bolt], while the aimed line
    // passes within centimetres of the aimed-at bolt, so the game's beam object (its sphere at the
    // knob it burns) is represented by the line's approach within the lock's reach. [S the exact
    // beam-object radius is unmeasured; 0.25.]
    const BEAM_RADIUS: f32 = 0.25;
    let ab = [end[0] - head[0], end[1] - head[1], end[2] - head[2]];
    let ab2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    for lock in locks.iter_mut() {
        if lock.burned { continue; }
        let reach = lock.radius + BEAM_RADIUS;
        if ab2 < 1e-9 { continue; }
        let ap = [lock.pos[0] - head[0], lock.pos[1] - head[1], lock.pos[2] - head[2]];
        let t = ((ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / ab2).clamp(0.0, 1.0);
        let closest = [head[0] + ab[0] * t, head[1] + ab[1] * t, head[2] + ab[2] * t];
        let d2 = (closest[0] - lock.pos[0]).powi(2) + (closest[1] - lock.pos[1]).powi(2) + (closest[2] - lock.pos[2]).powi(2);
        if d2 <= reach * reach {
            lock.burn(ch, frame);
        }
    }
    (head, hit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::CollisionWorld;

    fn empty_world() -> CollisionWorld { CollisionWorld::empty() }

    #[test]
    fn laser_burns_the_lock_it_touches_and_writes_the_out_channel() {
        let mut ch = Channels::default();
        let world = empty_world();
        let mut locks = vec![Lock {
            placement: 0, pos: [99.5, 0.0, 0.0], radius: 1.0, out: 134, key_channels: [0xffff; 2], burn: 0, burned: false,
        }];
        let (a, b) = laser_tick([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], &mut locks, &world, &mut ch, 10);
        assert!(locks[0].burned, "the aim line passes 0.5 from the lock's centre — within 1.0+0.25");
        assert!(ch.on(134), "the burn writes the lock's out channel [M 0x1ae3d8:194-195]");
        assert!((b[0] - 100.0).abs() < 0.01, "the empty world's beam runs its full 100 units");
        let _ = a;
    }

    #[test]
    fn laser_misses_the_lock_it_does_not_touch() {
        let mut ch = Channels::default();
        let world = empty_world();
        let mut locks = vec![Lock {
            placement: 0, pos: [99.5, 4.0, 0.0], radius: 1.0, out: 135, key_channels: [0xffff; 2], burn: 0, burned: false,
        }];
        let _ = laser_tick([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], &mut locks, &world, &mut ch, 10);
        assert!(!locks[0].burned, "the aim line passes 4 units off the lock's centre — no contact");
        assert!(!ch.on(135), "no burn, no channel");
    }

    #[test]
    fn is_laser_weapon_matches_the_laser_family() {
        // The disc records settle the ids: 0x4E/0x4F carry +0x6C & 0x100 (Draw_LaserBeam) and the
        // name label 0x05000043 "Laser"; 0x50/0x51 are the Grapple (0x4000) and 0x54/0x55 the
        // Micro-Camera (0) [L weapon_data + USATxt.dat].
        for w in [0x4eu8, 0x4f] { assert!(is_laser_weapon(w), "{w:#x} is a laser weapon"); }
        for w in [0x50u8, 0x51, 0x54, 0x55, 0x4a, 0x4c, 0x02] { assert!(!is_laser_weapon(w), "{w:#x} is not"); }
    }

    #[test]
    fn runs_beam_is_the_laser_and_the_stunner() {
        // +0x6C & 0x180 routes the shot's beam draw: 0x4E/0x4F → 0x900 (the laser), 0x4A/0x4C →
        // 0x880 (the taser) [L weapon_data; M 0x1a4298:131/134].
        for w in [0x4eu8, 0x4f, 0x4a, 0x4c] { assert!(runs_beam(w), "{w:#x} runs the beam"); }
        for w in [0x50u8, 0x54, 0x52] { assert!(!runs_beam(w), "{w:#x} does not"); }
    }
}

// ---- the Q-Claw grapple (0x4A/0x4C, substate 2) ----------------------------------------------------

/// A grapple point (placement 0x3A, `Grapple_Create` 0x198298 — the class-0x51 object, the world
/// sphere like the ladders [M Control_BuildWorldSph 0x133070]).
#[derive(Clone, Debug)]
pub struct GrapplePoint {
    pub pos: V3,
    pub radius: f32,
}

impl GrapplePoint {
    /// `Grapple_Create` over the main map's 0x3A placements (05's #281/#282 [D probe]).
    pub fn create(placements: &[nf_formats::placement::Placement], object: impl Fn(&nf_formats::placement::Placement) -> Option<(V3, f32)>) -> Vec<GrapplePoint> {
        let mut out = Vec::new();
        for p in placements.iter() {
            if p.flags & 0xff != 0x3a { continue; }
            let (centre, radius) = object(p).unwrap_or((p.translation, 1.0));
            out.push(GrapplePoint {
                pos: [p.translation[0] + centre[0], p.translation[1] + centre[1], p.translation[2] + centre[2]],
                radius,
            });
        }
        out
    }
}

/// The grapple substate's runtime state (`BLData+0x140..0x160`'s fields the substate reads).
#[derive(Clone, Debug)]
pub struct GrappleState {
    /// The point Bond hangs from (`+0x14C`'s object position + `+0xC0`'s offset [M 0x192860 tail]).
    pub point: V3,
    /// `+0x15C`: 1 = reeling, 2 = hanging.
    pub phase: u8,
    /// `+0x150`: the reel speed accumulator (0.05/tick up to 1.5 single-player [M 0x192860:96-104]).
    pub pull: f32,
}

/// The fire-time point selection: the nearest point whose sphere the aim ray touches within
/// `max_dist`. The game's test is a physical one: the claw's bolt (a flying bullet whose object
/// radius is 1.0 [M Bullet_init 0x12c5a0:92 — `obj+0xe8 = 1.0`, never overridden]) sweeps the ray
/// and its death check is the travelled-distance cap `weapon_data+0x1c` [M Bullet_update
/// 0x12e7b0:158], so the acceptance window is the point's sphere grown by the bolt's radius.
pub fn pick_grapple_point(points: &[GrapplePoint], from: V3, dir: V3, max_dist: f32) -> Option<V3> {
    const BOLT_RADIUS: f32 = 1.0; // [M 0x12c5a0:92]
    let mut best: Option<(f32, V3)> = None;
    for p in points {
        let Some(t) = ray_closest_t(from, dir, p.pos, max_dist) else { continue };
        let closest = [from[0] + dir[0] * t, from[1] + dir[1] * t, from[2] + dir[2] * t];
        let d2 = (closest[0] - p.pos[0]).powi(2) + (closest[1] - p.pos[1]).powi(2) + (closest[2] - p.pos[2]).powi(2);
        let reach = p.radius + BOLT_RADIUS;
        if d2 > reach * reach { continue; }
        if best.is_none_or(|(bt, _)| t < bt) { best = Some((t, p.pos)); }
    }
    best.map(|(_, pos)| pos)
}

/// The claw bolt's range: `weapon_data+0x1c` (the Q-Claw 0x4a 5.0, the upgrade 0x4c 10.0
/// [D weapon_data_live.bin — the fists 1.75 / P2K 50 / sniper 1200 scale confirms the unit]).
pub fn grapple_range(w: u8) -> f32 { if w == 0x4c { 10.0 } else { 5.0 } }

fn ray_closest_t(a: V3, dir: V3, p: V3, max_dist: f32) -> Option<f32> {
    let ab2 = dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2];
    if ab2 < 1e-9 { return None; }
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let t = (ap[0] * dir[0] + ap[1] * dir[1] + ap[2] * dir[2]) / ab2;
    (t >= 0.0 && t <= max_dist).then_some(t)
}

/// One tick of the grapple substate [M Player_SSGrapple 0x192860]. The fire polarity is the game's
/// own and it is the opposite of a press-to-release model: phase 1 (the reel) reads NO fire input
/// at all [M 0x192860:79-121 — the exits are the MP line-of-sight miss, the rope-fire anim done
/// with the 15-tick timer expired, and the arrival < 2.0]; the pull accumulates 0.05/tick up to
/// **1.5 single-player** [M 0x192860:96-104 — `fVar10 = 1.5` when `MPSettings == 0`], and the
/// arrival < 2.0 hangs [M 0x192860:91-94]. Phase 2 (the hang) holds while the fire is HELD and
/// detaches the tick it is RELEASED [M 0x192860:56-66 — `Input_Action(9,1)` non-zero commits the
/// wish velocity +0x10..0x1F → +0x20..0x2F and stays; zero → phase 4]. Returns whether the grapple
/// detached this tick.
pub fn grapple_tick(pos: &mut V3, state: &mut GrappleState, fire: bool) -> bool {
    match state.phase {
        1 => {
            // The reel: no fire test [M 0x192860:79-121]; the pull cap is 1.5 single-player.
            state.pull = (state.pull + 0.05).min(1.5);
            let d = [state.point[0] - pos[0], state.point[1] - pos[1], state.point[2] - pos[2]];
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            if len < 2.0 {
                state.phase = 2; // the hang [M 0x192860:91-94]
            } else {
                let step = state.pull / len;
                for k in 0..3 { pos[k] += d[k] * step; }
            }
            false
        }
        2 => {
            // The hang: HELD fire stays (the velocity commit [M 0x192860:57-62]); the RELEASE
            // detaches [M 0x192860:64-66 — phase 4]. The return is the detach.
            !fire
        }
        _ => false,
    }
}

#[cfg(test)]
mod grapple_tests {
    use super::*;

    #[test]
    fn grapple_reels_bond_to_the_point_and_hangs_while_the_fire_is_held() {
        let points = vec![GrapplePoint { pos: [10.0, 20.0, 0.0], radius: 2.0 }];
        let Some(point) = pick_grapple_point(&points, [0.0, 12.0, 0.0], [1.0, 1.0, 0.0].map(|v| v / 2f32.sqrt().powi(2) * 2f32.sqrt()), 100.0) else {
            panic!("the aim ray through the point's sphere picks it");
        };
        let mut state = GrappleState { point, phase: 1, pull: 0.0 };
        let mut pos = [0.0f32, 12.0, 0.0];
        let mut ticks = 0;
        // The reel runs with the fire HELD (phase 1 reads no fire input [M 0x192860:79-121]).
        while state.phase == 1 && ticks < 600 {
            grapple_tick(&mut pos, &mut state, true);
            ticks += 1;
        }
        assert_eq!(state.phase, 2, "the reel arrives (the distance < 2.0 [M 0x192860:91-94])");
        let d = ((pos[0] - point[0]).powi(2) + (pos[1] - point[1]).powi(2) + (pos[2] - point[2]).powi(2)).sqrt();
        assert!(d < 2.5, "Bond hangs within reach of the point (got {d:.2})");
        // The hang holds while the fire stays held [M 0x192860:57-62].
        for _ in 0..30 { assert!(!grapple_tick(&mut pos, &mut state, true), "held fire must NOT detach"); }
        // The RELEASE detaches [M 0x192860:64-66].
        assert!(grapple_tick(&mut pos, &mut state, false), "the fire release detaches");
    }

    #[test]
    fn grapple_acceptance_is_the_bolt_sweep_and_the_range_is_weapon_data() {
        // The bolt's radius grows the window: a ray passing 1.2 from a radius-0.64 point misses a
        // bare ray but the game's bolt (radius 1.0 [M 0x12c5a0:92]) hits.
        let points = vec![GrapplePoint { pos: [10.0, 0.0, 0.0], radius: 0.64 }];
        let dir = [10.0f32, 1.2, 0.0].map(|v| v / (10.0f32 * 10.0 + 1.2 * 1.2).sqrt());
        assert!(pick_grapple_point(&points, [0.0, 0.0, 0.0], dir, 100.0).is_some(),
            "the bolt sweep (radius 1.0) accepts a 1.2-unit pass");
        // The range is weapon_data+0x1c [M 0x12e7b0:158]: 5.0 for the claw, 10.0 for the upgrade.
        assert_eq!(grapple_range(0x4a), 5.0);
        assert_eq!(grapple_range(0x4c), 10.0);
        // Beyond the range the bolt dies mid-flight — no pick.
        let far = [10.0f32, 0.0, 0.0];
        let far_dir = [1.0f32, 0.0, 0.0];
        assert!(pick_grapple_point(&points, [0.0, 0.0, 0.0], far_dir, grapple_range(0x4a)).is_none()
            || (far[0] - 0.0) <= grapple_range(0x4a), "the point within the range still picks");
    }

    #[test]
    fn grapple_ignores_points_off_the_ray() {
        let points = vec![GrapplePoint { pos: [10.0, 0.0, 0.0], radius: 2.0 }];
        assert!(pick_grapple_point(&points, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0], 100.0).is_none(),
            "the ray pointing up misses the point on the +x axis");
    }
}
