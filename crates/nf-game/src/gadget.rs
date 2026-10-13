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

/// The weapon ids whose equipped state runs the laser beam: the Q-Laser (0x50/0x51) and the laser
/// watch (0x54; its taser variant 0x55 saves as 0x54 in the selection) [D weapon_data: the 0x54
/// record's +0x68 has the scoped flag 0x40; the beam weapon ids the locks read are 0x56/0x57
/// [M 0x1ae3d8:162]].
pub fn is_laser_weapon(w: u8) -> bool {
    matches!(w, 0x50 | 0x51 | 0x54 | 0x55)
}

/// One tick of the laser beam: the ray from `head` along `dir` (unit), 100.0 units [M 0x1a65d8],
/// against the world segment and the locks' world spheres. A lock whose sphere the ray touches
/// (the ray's closest point within its radius — the game's contact-list stand-in) burns; its out
/// channel is written on completion [M 0x1ae3d8:194-195].
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
        for w in [0x50u8, 0x51, 0x54, 0x55] { assert!(is_laser_weapon(w), "{w:#x} is a laser weapon"); }
        for w in [0x4Au8, 0x4e, 0x52, 0x02] { assert!(!is_laser_weapon(w), "{w:#x} is not"); }
    }
}
