//! Bullets fired by drones: aiming (`DroneWeap_DoBulletAccuracy` 0x179740), spread (`Bullet_init` 0x12c5a0),
//! flight and collision (`Bullet_update` 0x12e7b0, `Collide_Update`, `Bullet_CollisionHandler` 0x12ce80), damage to
//! Bond (`Drone_ModBulletDamage` 0x139f50, `Player_HandlePain` 0x1902c8). Spec: docs/systems/drone-ai.md "Bullets".

use crate::world::{CollisionWorld, V3};

/// One projectile in flight.
#[derive(Clone, Debug)]
pub struct Bullet {
    pub pos: V3,
    /// Unit direction (`BU+0x00`).
    pub dir: V3,
    /// Units per tick (`BU+0xF8` = `weapon_data+0x20`).
    pub speed: f32,
    /// Distance travelled (`BU+0xF4`) and the weapon's range (`weapon_data+0x1C`).
    pub travelled: f32,
    pub range: f32,
    /// Damage after `Drone_ModBulletDamage` (applied when it hits Bond).
    pub damage: f32,
    pub owner: usize,
    /// State 0 (first tick: step (0.5 + FRand 0.5) · speed), 1 flying, 2 dead.
    pub state: u8,
    /// `weapon_data+0x6C & 0x40000`: the weapon's bullets play a fly-by (whiz) sound in `Bullet_DoTrails` [M].
    pub flyby: bool,
}

/// What a bullet did this tick.
#[derive(Clone, Debug, PartialEq)]
pub enum BulletEvent {
    World(V3),
    Player { damage: f32, at: V3 },
    /// `Bullet_DoTrails` fly-by (sound id 0x1dc): a bullet whizzed past the viewer. Played at the closest point on
    /// the bullet's line to the camera, only for weapons with `+0x6C & 0x40000`, in single player, when the bullet's
    /// owner is not the player, within 3 units, on a 1-in-5 `Rand_Rand(5)` roll [M `Bullet_DoTrails`@0x12d890].
    FlyBy(V3),
}

/// Bond's hit volume: an upright cylinder from his feet to above his eye (*deviation*: the game tests 16 boxes on
/// his skeleton, `HumanCollisionBones` 0x2c6be8 via `Collide_Jointy`; the viewer's Bond has no skeleton).
#[derive(Clone, Copy, Debug)]
pub struct PlayerVolume { pub feet: V3, pub height: f32, pub radius: f32 }

fn norm(v: V3) -> V3 { let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt(); if l > 0.0 { [v[0] / l, v[1] / l, v[2] / l] } else { v } }

/// `Bullet_init` direction: normalise(normalise(dir) + spherical→cartesian(r, FRand 2π, FRand π)) with
/// r = (FRand(2s) − s) · 0.0014, s = spread `+0x24` × (0 when `weapon_data+0x68 & 0x400000`, else `Drone+0xC0` = 1);
/// the offset's components are added to x, z, y in that order.
pub fn spread(dir: V3, spread: f32, flags68: u32, rng: &mut crate::drone::Rng) -> V3 {
    let s = if flags68 & 0x40_0000 != 0 { 0.0 } else { spread };
    let r = (rng.frand(2.0 * s) - s) * 0.0014;
    let (th, ph) = (rng.frand(crate::trig::TAU), rng.frand(crate::trig::PI));
    let off = [r * ph.sin() * th.cos(), r * ph.sin() * th.sin(), r * ph.cos()];
    let d = norm(dir);
    norm([d[0] + off[0], d[1] + off[2], d[2] + off[1]])
}

/// First hit of the segment `a → b` with Bond's cylinder, as a fraction.
pub fn segment_cylinder(a: V3, b: V3, v: &PlayerVolume) -> Option<f32> {
    let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
    let (fx, fz) = (a[0] - v.feet[0], a[2] - v.feet[2]);
    let qa = dx * dx + dz * dz;
    let qb = 2.0 * (fx * dx + fz * dz);
    let qc = fx * fx + fz * fz - v.radius * v.radius;
    let t = if qa < 1e-12 { if qc <= 0.0 { 0.0 } else { return None } } else {
        let disc = qb * qb - 4.0 * qa * qc;
        if disc < 0.0 { return None; }
        let t0 = (-qb - disc.sqrt()) / (2.0 * qa);
        if qc <= 0.0 { 0.0 } else { t0 }
    };
    if !(0.0..=1.0).contains(&t) { return None; }
    let y = a[1] + (b[1] - a[1]) * t;
    (y >= v.feet[1] && y <= v.feet[1] + v.height).then_some(t)
}

impl Bullet {
    /// One tick (`Bullet_update` + collision): the hit test covers the segment from the current position along the
    /// direction for max(step, 1) units; the nearest of world and Bond wins; ranged-out bullets die.
    /// `viewer`: the camera eye (`glb_viewer + 0xc0`), for the `Bullet_DoTrails` fly-by test [M].
    pub fn update(&mut self, world: &CollisionWorld, player: &PlayerVolume, viewer: V3, rng: &mut crate::drone::Rng) -> Option<BulletEvent> {
        if self.state >= 2 { return None; }
        if self.travelled > self.range { self.state = 2; return None; }
        let mut step = if self.state == 0 { (rng.frand(0.5) + 0.5) * self.speed } else { self.speed };
        self.state = 1;
        self.travelled += step;
        if self.travelled > self.range { step -= self.travelled - self.range; }
        let l = step.max(1.0);
        let end = [self.pos[0] + self.dir[0] * l, self.pos[1] + self.dir[1] * l, self.pos[2] + self.dir[2] * l];
        let tw = world.segment(self.pos, end);
        let tp = segment_cylinder(self.pos, end, player);
        let at = |t: f32| [self.pos[0] + (end[0] - self.pos[0]) * t, self.pos[1] + (end[1] - self.pos[1]) * t, self.pos[2] + (end[2] - self.pos[2]) * t];
        let ev = match (tw, tp) {
            (Some(w), Some(p)) if p < w => Some(BulletEvent::Player { damage: self.damage, at: at(p) }),
            (_, Some(p)) if tw.is_none() => Some(BulletEvent::Player { damage: self.damage, at: at(p) }),
            (Some(w), _) => Some(BulletEvent::World(at(w))),
            _ => None,
        };
        if ev.is_some() { self.state = 2; return ev; }
        let old = self.pos;
        self.pos = [self.pos[0] + self.dir[0] * step, self.pos[1] + self.dir[1] * step, self.pos[2] + self.dir[2] * step];
        // `Bullet_DoTrails` (0x12d890): the fly-by whiz. Only weapons with `+0x6C & 0x40000`, single player (the port is
        // single-player only), and not the player's own bullet (all Bullets are drone-fired here). The closest point on
        // the bullet's path this tick to the viewer; if within 3 units (SqDist < 9), a 1-in-5 `Rand_Rand(5)` roll plays
        // sound 0x1dc at that point [M]. RNG order: `Rand_FRand` (above) → `Rand_Rand(5)` (here), matching the game.
        if self.flyby {
            let cp = closest_point_on_segment(old, self.pos, viewer);
            let d2 = (cp[0] - viewer[0]).powi(2) + (cp[1] - viewer[1]).powi(2) + (cp[2] - viewer[2]).powi(2);
            if d2 < 9.0 && rng.rand(5) == 0 { return Some(BulletEvent::FlyBy(cp)); }
        }
        None
    }
}

/// `vecutil_calculate_closest_point_on_line` (0x1df7e0): the closest point on segment `a→b` to `p`, clamped to [0,1].
/// Returns the point (the game also returns the parametric t, but only the point is used by DoTrails).
fn closest_point_on_segment(a: V3, b: V3, p: V3) -> V3 {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let len2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    if len2 < 1e-20 { return a; }
    let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1] + (p[2] - a[2]) * d[2]) / len2).clamp(0.0, 1.0);
    [a[0] + d[0] * t, a[1] + d[1] * t, a[2] + d[2] * t]
}

/// Bond's health and armour (`BL+0x894`, `+0x8B0`; single player starts at 100 / 0), the hit flash (`BL+0x968`)
/// and the hit-direction bits (`BL+0x967`).
#[derive(Clone, Copy, Debug)]
pub struct PlayerHealth { pub health: f32, pub armour: f32, pub flash: f32, /// `BL+0x967`: the damage-direction bits — 1/2 = the view's vertical axis (sprites 20/21), 4/8 = the horizontal (22/23) [M `HUD_UpdateHealthPane` 0x19b9c8:92-113].
pub dir_bits: u8 }

impl Default for PlayerHealth { fn default() -> Self { PlayerHealth { health: 100.0, armour: 0.0, flash: 0.0, dir_bits: 0 } } }

impl PlayerHealth {
    /// `Player_HandlePain` (single player, damage type 0): × the level's damage factor for the difficulty
    /// (`Plr_DMod_*`, `ReadTuningVars`; 4 → ×2), armour absorbs first, health floors at 0 (below 1 → 0);
    /// flash += min(d · 21.333, 128), capped at 255. [L] 07000005 at difficulty 1: SG5 hits cost 1.65 = 2.75 × 0.6.
    pub fn pain(&mut self, damage: f32, difficulty: u32, level: u32) {
        if self.health <= 0.0 || damage <= 0.0 { return; }
        let d = damage * damage_factor(level, difficulty);
        let absorbed = d.min(self.armour);
        self.armour -= absorbed;
        self.health = (self.health - (d - absorbed)).max(0.0);
        if self.health < 1.0 { self.health = 0.0; }
        self.flash = (self.flash + (d * 21.333).min(128.0)).min(255.0);
    }

    /// `Player_HandlePain` 0x1902c8:124-130 also writes `BL+0x967`: the generic pain paths set 0xC (both
    /// sides — drowning/pain-type 7 use it too); pain type 6 sets 2. The directional path is
    /// `Player_DealWithObjHit` 0x194458:201-226, which the callers below mirror instead.
    pub fn pain_generic(&mut self, damage: f32, difficulty: u32, level: u32, pain_type: u32) {
        self.dir_bits = if pain_type == 6 { 2 } else { 0xC };
        self.pain(damage, difficulty, level);
    }

    /// `Player_DealWithObjHit` 0x194458:201-226: the attacker position transformed into the viewer's frame,
    /// normalised, then `if 0.5 <= |x| or |y|`: `|y| < |x|` → `x < 0 ? 4 : 8`, else `y < 0 ? 1 : 2`.
    /// The game reads the matrix at `glb_viewer+0x160`; the port rebuilds the two axes from Bond's yaw
    /// (right = (cos yaw, 0, −sin yaw), forward = (sin yaw, 0, cos yaw)) and takes the view's y as the
    /// attacker's height over the eye — the same x rule exactly; the y rule is [S] (the world-up
    /// projection of a pitch-rotated frame).
    pub fn pain_from(&mut self, damage: f32, difficulty: u32, level: u32, attacker: [f32; 3], eye: [f32; 3], yaw: f32) {
        let d = [attacker[0] - eye[0], attacker[1] - eye[1], attacker[2] - eye[2]];
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        self.dir_bits = 0;
        if len > 1e-4 {
            let (x, y) = (d[0] / len, d[1] / len);
            // The port's object frame has +X = screen-left (see `Sim::crosshair`), so the view's x axis is
            // (−cos yaw, 0, sin yaw); x < 0 = the attacker on the screen's left → bit 4 (sprite 22).
            let (rx, rz) = (-yaw.cos(), yaw.sin());
            let vx = (d[0] / len) * rx + (d[2] / len) * rz;
            let vy = y;
            let (ax, ay) = (vx.abs(), vy.abs());
            if ax >= 0.5 || ay >= 0.5 {
                if ay < ax { self.dir_bits = if vx < 0.0 { 0x4 } else { 0x8 }; }
                else { self.dir_bits = if vy < 0.0 { 0x1 } else { 0x2 }; }
            }
        }
        self.pain(damage, difficulty, level);
    }
}

/// `Plr_DMod_Easy/Normal/Hard` per level (`ReadTuningVars`): 0x7000001–04 0.5/0.7/1.2, 0x7000005–08 0.6/0.7/1.0,
/// otherwise the ELF defaults 0.75/1.0/1.25; difficulty 4 doubles.
pub fn damage_factor(level: u32, difficulty: u32) -> f32 {
    let (e, n, h) = match level & 0xff { 0x01..=0x04 => (0.5, 0.7, 1.2), 0x05..=0x08 => (0.6, 0.7, 1.0), _ => (0.75, 1.0, 1.25) };
    match difficulty { 1 => e, 2 => n, 3 => h, 4 => 2.0, _ => 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cylinder_and_pain() {
        let v = PlayerVolume { feet: [0.0, 0.0, 0.0], height: 1.8, radius: 0.3 };
        assert!(segment_cylinder([-5.0, 1.0, 0.0], [5.0, 1.0, 0.0], &v).is_some_and(|t| (t - 0.47).abs() < 1e-3));
        assert!(segment_cylinder([-5.0, 2.5, 0.0], [5.0, 2.5, 0.0], &v).is_none(), "over his head");
        assert!(segment_cylinder([-5.0, 1.0, 1.0], [5.0, 1.0, 1.0], &v).is_none(), "beside him");
        let mut h = PlayerHealth::default();
        h.pain(2.75, 1, 0x0700_0005);
        assert!((h.health - (100.0 - 1.65)).abs() < 1e-4, "live: 1.65 per SG5 hit");
        assert!((h.flash - 1.65 * 21.333).abs() < 1e-3);
    }

    /// `Bullet_DoTrails` fly-by (0x12d890): a bullet passing within 3 units of the viewer on a 1-in-5 roll plays
    /// sound 0x1dc. Sister: a bullet with `flyby=false` never produces a FlyBy, and one too far never does.
    #[test]
    fn flyby_plays_within_three_units_on_one_in_five() {
        let world = CollisionWorld::empty();
        // A clean world (no collision) so the bullet flies past.
        let vol = PlayerVolume { feet: [100.0, 0.0, 100.0], height: 1.8, radius: 0.25 };
        // Viewer at the origin; bullet flies along +z, 2 units to the side (within 3).
        let mut b = Bullet { pos: [0.0, 1.0, -50.0], dir: [0.0, 0.0, 1.0], speed: 15.0, travelled: 0.0, range: 200.0,
            damage: 0.0, owner: 1, state: 0, flyby: true };
        let mut rng = crate::drone::Rng::new(0);
        // The bullet flies many ticks past the viewer; the 1-in-5 roll should eventually fire.
        let mut fired = false;
        for _ in 0..60 {
            if let Some(BulletEvent::FlyBy(_)) = b.update(&world, &vol, [0.0; 3], &mut rng) { fired = true; break; }
        }
        assert!(fired, "a fly-by bullet within 3 units should fire on the 1-in-5 roll within 60 ticks");
        // Sister: a bullet without the fly-by flag never fires.
        let mut b2 = Bullet { pos: [0.0, 1.0, -50.0], dir: [0.0, 0.0, 1.0], speed: 15.0, travelled: 0.0, range: 200.0,
            damage: 0.0, owner: 1, state: 0, flyby: false };
        let mut rng2 = crate::drone::Rng::new(0);
        let mut fired2 = false;
        for _ in 0..60 {
            if let Some(BulletEvent::FlyBy(_)) = b2.update(&world, &vol, [0.0; 3], &mut rng2) { fired2 = true; break; }
        }
        assert!(!fired2, "no fly-by flag → no FlyBy event");
        // Sister: a bullet 10 units away (SqDist 100 > 9) never fires even with the flag.
        let mut b3 = Bullet { pos: [10.0, 1.0, -50.0], dir: [0.0, 0.0, 1.0], speed: 15.0, travelled: 0.0, range: 200.0,
            damage: 0.0, owner: 1, state: 0, flyby: true };
        let mut rng3 = crate::drone::Rng::new(0);
        let mut fired3 = false;
        for _ in 0..60 {
            if let Some(BulletEvent::FlyBy(_)) = b3.update(&world, &vol, [0.0; 3], &mut rng3) { fired3 = true; break; }
        }
        assert!(!fired3, "beyond 3 units → no fly-by");
    }
}
