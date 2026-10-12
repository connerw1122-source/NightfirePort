//! Grenades: the throw (`DroneWeap_ThrowGrenade` 0x179558, `DroneWeap_CanThrowGrenade` 0x17a798), the projectile's
//! flight (`Bullet_update` 0x12e7b0 for a gravity-affected bullet), the bounce (`Bullet_CollisionHandler` 0x12ce80),
//! the explosion (`Explode_Create` 0x17cbd0, `Explode_Propagate` 0x17c518, `Drone_ExplosiveHit` 0x13aab8). Spec:
//! docs/systems/drone-ai.md "Grenades", docs/systems/drone-cover.md (the throw state 0x2D).
//!
//! The game has no separate grenade entity — grenades are bullets (`obj+0xFF = 5`) with `weapon_data+0x6C & 0x8`
//! (gravity), `+0x70 & 0x4` (explode on impact) and `+0x70 & 0x100` (bounce). The port models them as a `Grenade`
//! struct (a projectile with velocity, gravity and a bounce counter) plus an `Explosion` (radius + damage + the
//! propagate step). The throw state (0x2D) and the `DroneAnim_EventFunc` datum 1 live in `drone.rs`.

use crate::world::{CollisionWorld, V3};
use crate::drone::{Rng, DIFFICULTY};

/// One grenade in flight (`Bullet_init` with a gravity-affected weapon_definition). Fields carry the game's `BU+0x…`
/// offsets where they match.
#[derive(Clone, Debug)]
pub struct Grenade {
    /// `obj+0x30`: position.
    pub pos: V3,
    /// `BU+0x00`: velocity (units per tick); gravity accumulates here.
    pub velocity: V3,
    /// `BU+0xF8`: speed (`weapon_data+0x20`).
    pub speed: f32,
    /// `BU+0xF4`: distance travelled; ranged-out grenades die.
    pub travelled: f32,
    /// `weapon_data+0x1C` / the per-type range (`Bullet_init`: 210 default, 420 for type 0x37, etc.).
    pub range: f32,
    /// `weapon_data+0x0C` × the explosion damage (`Explode_Propagate`'s `param_2`).
    pub damage: f32,
    /// `weapon_data+0x08`: the explosion radius (`Explode_Propagate`'s `param_1`).
    pub radius: f32,
    /// The weapon's type id (`weapon_data+0x02`): 0x35 frag, 0x36 stun, 0x37 mine.
    pub weapon_type: u16,
    /// The drawn mesh (`weapon_data[w]+0x5C` of the weapon the throw used, `Bullet_update` state 0's
    /// `hashtable_set_object_to_entity_gfx(obj, weapon+0x5C)` disasm 0x12e8f0-0x12e900 [M]; 0 = none). The
    /// renderer poses it at the flight matrix every tick (`View_SetDrawInAllViews` in the game).
    pub mesh: u32,
    pub owner: usize,
    /// `BU+0xF4` state: 0 first tick, 1 flying, 2 dead (exploded).
    pub state: u8,
    /// `BU+0x10`: bounce counter (`Bullet_CollisionHandler`: max 0x14 = 20 bounces; 3 for ricochets).
    pub bounces: u8,
}

/// What a grenade did this tick.
#[derive(Clone, Debug, PartialEq)]
pub enum GrenadeEvent {
    /// Bounced off the world at `pos` (sound 0x51C).
    Bounce(V3),
    /// Exploded at `pos` — the caller applies the explosion (radius, damage, owner) to drones and Bond.
    Explode { pos: V3, radius: f32, damage: f32, owner: usize, weapon_type: u16 },
}

/// `WldGravity` (VECTOR at 0x2d8890): `(0, −9.8, 0)`. [L] docs/systems/pickups.md, player.md.
const WLD_GRAVITY: V3 = [0.0, -9.8, 0.0];
/// `REC_FRAME_RATE` (0x30d0cc) = 1/60.
const REC_FRAME_RATE: f32 = 1.0 / 60.0;
/// `Bullet_CollisionHandler` bounce max: 0x14 (20) bounces.
const BOUNCE_MAX: u8 = 20;
/// `Bullet_CollisionHandler` min speed to keep bouncing: `FRAME_RATE_MUL · 0.01` = 0.01.
const BOUNCE_MIN_SPEED: f32 = 0.01;
/// A per-material bounce restitution stand-in (the game reads `0x3037dc + material·0x88`; the port has no per-triangle
/// material coefficients — 0.4 is the feet-sphere radius and a reasonable floor bounce).
const BOUNCE_RESTITUTION: f32 = 0.4;

impl Grenade {
    /// `Bullet_update` (0x12e7b0) for a gravity-affected grenade (`+0x6C & 0x8`): the first tick steps
    /// (0.5 + FRand 0.5) · speed [M the default case 0x12e7c0-0x12e828]; subsequent ticks apply
    /// `velocity += WldGravity · REC_FRAME_RATE²` and **`pos += velocity · FRAME_RATE_MUL`** — the game's
    /// velocity is the per-tick displacement (unit direction · speed, re-normalised after the gravity add
    /// with `Vec_NormaliseLen` storing the length back into BU+0xF8), and FRAME_RATE_MUL = 1 at the fixed
    /// 60 Hz [M the movement tail 0x12f040-0x12f058: `pos += fVar16 · velocity` with
    /// fVar16 = speed · FRAME_RATE_MUL = speed; docs/systems/input.md:126]. fix-player3: the port moved
    /// `pos += velocity · REC_FRAME_RATE` — 60× too slow, so a thrown grenade crawled at ~0.17 u/s and the
    /// 210-tick fuse expired at Bond's feet ("the throw doesn't really happen, no explosion").
    /// On a world hit: bounce (reflect the velocity by the surface normal, × the per-material coefficient, up to 20
    /// bounces) or explode (`+0x70 & 0x4`). Ranged-out grenades die.
    pub fn update(&mut self, world: &CollisionWorld, rng: &mut Rng) -> Option<GrenadeEvent> {
        if self.state >= 2 { return None; }
        // The fuse: `+0x3F` (range) is the grenade's tick fuse (210 default, 300 with +0x70 & 0x10000,
        // 420 for the mine 0x37 — `Bullet_init` 0x12c5a0:103,196-223). Count ticks, not distance —
        // `weapon_data+0x1C` (5000) is the max-travelled range, a SILENT state-4 death, not the fuse
        // [M Bullet_update case 1: travelled > +0x1C → state 4, no explosion].
        self.travelled += 1.0;
        if self.travelled > self.range { self.state = 2; return Some(GrenadeEvent::Explode { pos: self.pos, radius: self.radius, damage: self.damage, owner: self.owner, weapon_type: self.weapon_type }); }
        // First tick: half-speed (the game's `Rand_FRand(0.5) + 0.5` step [M 0x12e7c0]).
        let step = if self.state == 0 { rng.frand(0.5) + 0.5 } else { 1.0 };
        self.state = 1;
        // Gravity: `velocity += WldGravity · REC_FRAME_RATE²` (the game's `FRAME_RATE_DIV · REC_FRAME_RATE²` with
        // `FRAME_RATE_DIV = REC_FRAME_RATE = 1/60`).
        self.velocity[0] += WLD_GRAVITY[0] * REC_FRAME_RATE * REC_FRAME_RATE;
        self.velocity[1] += WLD_GRAVITY[1] * REC_FRAME_RATE * REC_FRAME_RATE;
        self.velocity[2] += WLD_GRAVITY[2] * REC_FRAME_RATE * REC_FRAME_RATE;
        // `pos += velocity · step` (the game's per-tick displacement — see the doc comment above).
        let dt = step;
        let end = [self.pos[0] + self.velocity[0] * dt, self.pos[1] + self.velocity[1] * dt, self.pos[2] + self.velocity[2] * dt];
        // Collision: the segment from pos to end against the world.
        if let Some((t, normal, _mat)) = world.segment_surface(self.pos, end) {
            let hit = [self.pos[0] + (end[0] - self.pos[0]) * t, self.pos[1] + (end[1] - self.pos[1]) * t, self.pos[2] + (end[2] - self.pos[2]) * t];
            if self.bounces >= BOUNCE_MAX {
                self.state = 2;
                return Some(GrenadeEvent::Explode { pos: hit, radius: self.radius, damage: self.damage, owner: self.owner, weapon_type: self.weapon_type });
            }
            // Bounce: reflect the velocity by the surface normal, × the restitution.
            let v = self.velocity;
            let dot = v[0] * normal[0] + v[1] * normal[1] + v[2] * normal[2];
            let reflected = [v[0] - 2.0 * dot * normal[0], v[1] - 2.0 * dot * normal[1], v[2] - 2.0 * dot * normal[2]];
            self.velocity = [reflected[0] * BOUNCE_RESTITUTION, reflected[1] * BOUNCE_RESTITUTION, reflected[2] * BOUNCE_RESTITUTION];
            self.pos = hit;
            self.bounces += 1;
            // If too slow, explode.
            let speed = (self.velocity[0] * self.velocity[0] + self.velocity[1] * self.velocity[1] + self.velocity[2] * self.velocity[2]).sqrt();
            if speed < BOUNCE_MIN_SPEED {
                self.state = 2;
                return Some(GrenadeEvent::Explode { pos: hit, radius: self.radius, damage: self.damage, owner: self.owner, weapon_type: self.weapon_type });
            }
            return Some(GrenadeEvent::Bounce(hit));
        }
        self.pos = end;
        None
    }
}

/// The result of an explosion's damage application (`Explode_Propagate` 0x17c518): who was hit and for how much.
#[derive(Clone, Debug, Default)]
pub struct ExplosionHits {
    /// (drone index, damage) — `Drone_ExplosiveHit` → msg 9.
    pub drones: Vec<(usize, f32)>,
    /// Damage to Bond (`Player_Hurt`).
    pub player: f32,
    /// The explosion's position (for the sound and the camera shake).
    pub pos: V3,
}

/// `Explode_Propagate` (0x17c518): a sphere intersect of `radius` from `pos`. For each hit object, `damage = amount ·
/// (1 − dist / radius)` (clamped to ≥ 0.0002). Drones (type 2) take `Drone_ExplosiveHit` (msg 9); Bond (type 3) takes
/// `Player_Hurt`. The min cutoff 0.0002 means distant objects take no damage. [M] linear falloff, min 0.0002.
pub fn explode_propagate(
    pos: V3,
    radius: f32,
    damage: f32,
    drones: &[(usize, V3)],
    player: V3,
    _player_height: f32,
) -> ExplosionHits {
    let mut hits = ExplosionHits { pos, ..Default::default() };
    if radius < 0.0002 || damage < 0.0002 { return hits; }
    // Bond: a cylinder from feet to feet + height (the game tests 16 boxes; the port uses one cylinder).
    let pdy = player[1] - pos[1];
    let pdx = player[0] - pos[0];
    let pdz = player[2] - pos[2];
    let pdist = (pdx * pdx + pdy * pdy + pdz * pdz).sqrt();
    if pdist < radius {
        let d = damage * (1.0 - pdist / radius);
        // `Player_Hurt` applies the level's damage factor (`Plr_DMod_*`); 07000005 difficulty 1 → 0.6.
        if d >= 0.0002 { hits.player = d * crate::bullet::damage_factor(0x0700_0005, DIFFICULTY); }
    }
    for &(i, dpos) in drones {
        let dx = dpos[0] - pos[0];
        let dy = dpos[1] - pos[1];
        let dz = dpos[2] - pos[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist < radius {
            let d = damage * (1.0 - dist / radius);
            if d >= 0.0002 { hits.drones.push((i, d)); }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A floor quad at y = 0 (large enough that a thrown grenade stays above it while it falls).
    fn floor_world() -> CollisionWorld {
        CollisionWorld::from_triangles(vec![
            [[-100.0, 0.0, -100.0], [100.0, 0.0, -100.0], [100.0, 0.0, 100.0]],
            [[-100.0, 0.0, -100.0], [100.0, 0.0, 100.0], [-100.0, 0.0, 100.0]],
        ])
    }

    #[test]
    fn grenade_falls_under_gravity_and_bounces() {
        let w = floor_world();
        // A grenade thrown downward at 5 u/tick from 2 units up: it reaches the floor and bounces before the 210-tick
        // fuse expires.
        let mut g = Grenade {
            pos: [0.0, 2.0, 0.0], velocity: [5.0, -5.0, 0.0], speed: 7.0, travelled: 0.0, range: 210.0,
            damage: 50.0, radius: 4.0, weapon_type: 0x35, mesh: 0x0200_0726, owner: 0, state: 0, bounces: 0,
        };
        let mut rng = Rng::new(0);
        let mut bounced = false;
        for k in 0..2000 {
            if let Some(ev) = g.update(&w, &mut rng) {
                if matches!(ev, GrenadeEvent::Bounce(_)) { bounced = true; break; }
                if matches!(ev, GrenadeEvent::Explode { .. }) { eprintln!("exploded at tick {k} pos={:?}", g.pos); break; }
            }
        }
        assert!(bounced, "grenade bounced off the floor");
    }

    #[test]
    fn grenade_explodes_when_bounces_exceed_max() {
        let w = floor_world();
        // A grenade at the bounce max hitting the floor explodes on contact [M Bullet_CollisionHandler
        // 0x12ce80]. (fix-player3: with the corrected per-tick flight, a grenade one bounce short
        // bounces and flies back up at ~12 u/tick — real gravity (9.8/3600 u/tick²) takes thousands of
        // ticks to bring it down, so the old 5-tick "one short" premise no longer explodes.)
        let mut g = Grenade {
            pos: [0.0, 0.5, 0.0], velocity: [0.0, -30.0, 0.0], speed: 30.0, travelled: 0.0, range: 200.0,
            damage: 50.0, radius: 4.0, weapon_type: 0x35, mesh: 0x0200_0726, owner: 0, state: 0, bounces: BOUNCE_MAX,
        };
        let mut rng = Rng::new(0);
        let mut exploded = false;
        for _ in 0..5 {
            if let Some(GrenadeEvent::Explode { .. }) = g.update(&w, &mut rng) { exploded = true; break; }
        }
        assert!(exploded, "grenade explodes after max bounces");
        // Sister: one bounce short it bounces instead (bounces 19 → 20, still flying).
        let mut g = Grenade {
            pos: [0.0, 10.0, 0.0], velocity: [0.0, -30.0, 0.0], speed: 30.0, travelled: 0.0, range: 200.0,
            damage: 50.0, radius: 4.0, weapon_type: 0x35, mesh: 0x0200_0726, owner: 0, state: 0, bounces: BOUNCE_MAX - 1,
        };
        if let Some(ev) = g.update(&w, &mut rng) { assert!(matches!(ev, GrenadeEvent::Bounce(_)), "one short: a bounce, got {ev:?}"); }
        else { assert!(g.bounces == BOUNCE_MAX, "the bounce counted"); }
    }

    #[test]
    fn explode_propagate_linear_falloff() {
        // A grenade at the origin, radius 4, damage 100. Bond at distance 0 takes 100; at distance 2 takes 50; at
        // distance 4 takes 0 (the cutoff); a drone at distance 1 takes 75.
        let hits = explode_propagate([0.0, 1.0, 0.0], 4.0, 100.0, &[(0, [1.0, 1.0, 0.0])], [0.0, 1.0, 0.0], 1.8);
        // Bond at the centre: damage 100 × the difficulty factor (0.6 on 07000005 difficulty 1) = 60.
        assert!((hits.player - 100.0 * 0.6).abs() < 1e-3, "player at centre: {}", hits.player);
        // Drone at distance 1: damage 100 × (1 − 1/4) = 75.
        assert!((hits.drones[0].1 - 75.0).abs() < 1e-3, "drone at distance 1: {}", hits.drones[0].1);
        // Sister: a target beyond the radius takes nothing.
        let hits2 = explode_propagate([0.0, 1.0, 0.0], 4.0, 100.0, &[(0, [10.0, 1.0, 0.0])], [10.0, 1.0, 0.0], 1.8);
        assert!(hits2.drones.is_empty(), "drone beyond radius: no damage");
        assert!((hits2.player - 0.0).abs() < 1e-6, "player beyond radius: no damage");
    }
}
