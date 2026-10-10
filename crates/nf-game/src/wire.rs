//! The zip wire / creep ledge (`Wire_Create` 0x1981d8, `Player_Creep` 0x1969d0, `Player_Wire` 0x197658).
//! A wire is a line segment in the world; when Bond grabs it he enters the creep substate, which posts
//! `Music_Event(0xD, 1)` every tick (docs/systems/music.md event 0xD: the music jumps to section 2 stealth,
//! or 6 if in combat). Spec: docs/systems/player.md, docs/systems/world-objects.md.

use crate::world::V3;
use nf_formats::placement::Placement;

/// A wire object (`control_create_object(8)`, class 0x2C). The wire's endpoints come from the placement's
/// translation (one end) and the glist model's extent rotated by the placement's yaw (the other end) [L].
#[derive(Clone, Debug)]
pub struct Wire {
    /// The two endpoints in world space.
    pub a: V3,
    pub b: V3,
    /// `WIREINFO+0x02` = `placement.extra[2]` low 16 bits — a switch channel or parameter (0x016D on
    /// 07000005's wire) [L].
    pub channel: u16,
    /// `WIREINFO+0x00` = `placement.extra[3]` low 16 bits — the glist hashcode for the wire's visual model
    /// (0xC510 on 07000005) [L].
    pub glist: u16,
}

impl Wire {
    /// `Wire_Create` (0x1981d8): creates a wire from a placement (type 0x3A). The placement's translation is
    /// one endpoint; the other is `translation + direction · length`, where the direction is the placement's
    /// yaw + π/2 (the wire runs perpendicular to the facing direction) and the length is the glist model's
    /// extent. The z-rotation is negated (like ladders) [M].
    pub fn create(p: &Placement, length: f32) -> Wire {
        let a = p.translation;
        // The wire's direction: yaw + π/2 (the placement faces along the wire's perpendicular).
        // `Vec_Spherical_2_Cartesian(length - 0.5, yaw + π/2, 0)` in `Player_Creep` [M 0x1969d0].
        let yaw = p.rotation[1] + std::f32::consts::FRAC_PI_2;
        let l = (length - 0.5).max(0.0);
        let b = [a[0] + l * yaw.cos(), a[1], a[2] - l * yaw.sin()];
        let channel = p.extra[2] as u16;
        let glist = p.extra[3] as u16;
        Wire { a, b, channel, glist }
    }

    /// The closest point on the wire segment to `p` (`vecutil_calculate_closest_point_on_line`).
    pub fn closest_point(&self, p: V3) -> V3 {
        let ab = [self.b[0] - self.a[0], self.b[1] - self.a[1], self.b[2] - self.a[2]];
        let ap = [p[0] - self.a[0], p[1] - self.a[1], p[2] - self.a[2]];
        let ab2 = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
        let t = if ab2 > 1e-9 { (ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / ab2 } else { 0.0 };
        let t = t.clamp(0.0, 1.0);
        [self.a[0] + t * ab[0], self.a[1] + t * ab[1], self.a[2] + t * ab[2]]
    }

    /// Whether Bond at `pos` is within `r` units of the wire segment (the straddle/grab zone).
    pub fn near(&self, pos: V3, r: f32) -> bool {
        let c = self.closest_point(pos);
        let d = [c[0] - pos[0], c[1] - pos[1], c[2] - pos[2]];
        d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < r * r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closest_point_and_near() {
        let w = Wire { a: [0.0, 1.0, 0.0], b: [10.0, 1.0, 0.0], channel: 0, glist: 0 };
        // Closest point to (5, 0, 0) is (5, 1, 0) — the middle of the segment.
        let c = w.closest_point([5.0, 0.0, 0.0]);
        assert!((c[0] - 5.0).abs() < 1e-6 && (c[1] - 1.0).abs() < 1e-6);
        // Near: within 1.5 units of the wire.
        assert!(w.near([5.0, 0.0, 0.0], 1.5));
        assert!(!w.near([5.0, 0.0, 0.0], 0.5), "1.0 below the wire is > 0.5 away");
        // Sister: off the end of the segment, the closest point is clamped to the endpoint.
        let c2 = w.closest_point([15.0, 1.0, 0.0]);
        assert!((c2[0] - 10.0).abs() < 1e-6, "clamped to endpoint b");
    }
}
