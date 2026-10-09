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
}

/// What a bullet did this tick.
#[derive(Clone, Debug, PartialEq)]
pub enum BulletEvent {
    World(V3),
    Player { damage: f32, at: V3 },
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
    pub fn update(&mut self, world: &CollisionWorld, player: &PlayerVolume, rng: &mut crate::drone::Rng) -> Option<BulletEvent> {
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
        self.pos = [self.pos[0] + self.dir[0] * step, self.pos[1] + self.dir[1] * step, self.pos[2] + self.dir[2] * step];
        None
    }
}

/// Bond's health and armour (`BL+0x894`, `+0x8B0`; single player starts at 100 / 0) and the hit flash (`BL+0x968`).
#[derive(Clone, Copy, Debug)]
pub struct PlayerHealth { pub health: f32, pub armour: f32, pub flash: f32 }

impl Default for PlayerHealth { fn default() -> Self { PlayerHealth { health: 100.0, armour: 0.0, flash: 0.0 } } }

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
}
