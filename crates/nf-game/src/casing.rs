//! Ejected shell casings: `Bullet_InitCasingLists` 0x12fd48 (the 32-object round-robin pool),
//! `Bullet_init_casing` 0x12fde0 (the delayed activation), `Bullet_init_casing_ex` 0x12f678 (the
//! eject: view-model bone transform, `Rand_FRand_MVar2` velocity/spin jitter, the landing plane)
//! and `Bullet_casing_update` 0x12faa0 (gravity, the landing sound by material, the 240-tick life).
//! Fired from `Player_WeaponInitBullet` 0x1a4bf0 (Bond, after the bullet's spread draws) and
//! `DroneWeap_FireWeapon` 0x178f68 (guards, after `Bullet_init`, before `Rand_Rand(500)`).
//! G-025 / docs/atlas/effect.md.

use crate::drone::Rng;
use crate::effect::frand_mvar2;
use crate::world::V3;

/// `REC_FRAME_RATE` [M grenade.rs:52]; the casing physics scale `REC_FRAME_RATE²` in the update.
const REC: f32 = 1.0 / 60.0;
/// `_WldGravity` [L grenade.rs:51].
const GRAVITY: f32 = -9.8;
/// `Bullet_casing_update` 0x12faa0: `if (240.0 < age) → state 2, undrawn` [M 0x12fc80].
const LIFETIME: f32 = 240.0;
/// The pool: `Bullet_InitCasingLists` 0x12fd48 creates 32 class-0x40 objects, `NextCasing` round-robin.
pub const POOL: usize = 32;
/// The default shell mesh the collision handler special-cases (0x2000548) [M 0x12fb48].
pub const DEFAULT_MESH: u32 = 0x0200_0548;

/// One pooled casing. States as the game's `obj+0xf4`: 0 flying, 2 dead, 3 delayed (armed by
/// `Bullet_init_casing`, activated by `Bullet_init_casing_ex` when the weapon's `+0x78` delay runs out).
#[derive(Clone, Debug, PartialEq)]
pub struct Casing {
    pub state: u8,
    /// `data+0x24` while armed: ticks until `Bullet_init_casing_ex` runs (weapon_data+0x78 as f32).
    pub delay: f32,
    pub pos: V3,
    /// `data+0x00`: world velocity (units/tick).
    pub vel: V3,
    /// `data+0x10`: spin, added to the rotation every tick (radians/tick).
    pub spin: V3,
    /// The accumulated spin angle for the renderer (the game accumulates the obj's rotation quaternion).
    pub rot: f32,
    /// `data+0x24` while flying: the age; despawn at > 240.
    pub age: f32,
    /// `data+0x20`: the landing plane — the shooter's y − (1 + FRand(0.5)) [M 0x12f8e8].
    pub landing_y: f32,
    /// `data+0x2c`: the mesh from `ammo_data[weapon_data+0x90]+4` [M 0x12fe2c].
    pub mesh: u32,
    /// `data+0x30`: the landing-sound material, the shooter's weapon definition +0x50 byte & 0x3f
    /// [M 0x12f8f8] ([S] the port reads the weapon_data record byte).
    pub material: u16,
    /// The landing sound plays once ([S] the game re-triggers while below the plane every tick;
    /// the port gates it — see the report).
    pub landed: bool,
}

/// The 32-slot pool (`ShellCasings[32]` + `NextCasing`, 0x3495e0/0x3495dc [M 0x12fd48]).
#[derive(Clone, Debug)]
pub struct Casings {
    pub list: Vec<Casing>,
    next: usize,
}

impl Default for Casings { fn default() -> Self { Self::new() } }

impl Casings {
    pub fn new() -> Casings { Casings { list: (0..POOL).map(|_| Casing {
        state: 2, delay: 0.0, pos: [0.0; 3], vel: [0.0; 3], spin: [0.0; 3], rot: 0.0, age: 0.0,
        landing_y: 0.0, mesh: 0, material: 0, landed: false,
    }).collect(), next: 0 } }

    /// `Bullet_init_casing` 0x12fde0 + `Bullet_init_casing_ex` 0x12f678, collapsed into one call
    /// (the game splits them over the weapon's `+0x78`-tick delay; the delay is kept in `state 3`).
    ///
    /// `weapon` is the 0x10c-byte weapon_data record: +0x50 material byte, +0x78 eject delay (u8→f32),
    /// +0x90 ammo type byte, +0xf8/+0xfc/+0x100 the local eject velocity (0xfc negated [M 0x12f838]).
    /// `origin` is the eject position ([S] the port passes the muzzle; the game reads the view model's
    /// bone 1/5 world transform — the main agent's render check owns the visual). `yaw`/`pitch` build
    /// the bone-rotation stand-in ([S]). `shooter_y` is the shooter's y (`obj+0x34`). RNG order [M
    /// 0x12f860..0x12f8f0]: 3× `Rand_FRand_MVar2(0.1, 0.05)` (spin), 3× `(0.01, 0.005)` (velocity
    /// jitter), 1× `Rand_FRand(0.5)` (landing plane).
    pub fn eject(&mut self, ammo: &nf_formats::sfx::AmmoData, weapon: &[u8], origin: V3, yaw: f32, pitch: f32, shooter_y: f32, rng: &mut Rng) {
        // The bone-rotation stand-in: the shooter's yaw about y, then the pitch ([S]; the game uses
        // `AnimGetBoneWorldTrans`'s matrix, which carries the view model's own orientation).
        let (cy, sy) = (yaw.cos(), yaw.sin());
        let (cp, sp) = (pitch.cos(), pitch.sin());
        let right = [cy, 0.0, -sy];
        let fwd = [sy * cp, sp, cy * cp];
        let up = [fwd[1] * right[2] - fwd[2] * right[1], fwd[2] * right[0] - fwd[0] * right[2], fwd[0] * right[1] - fwd[1] * right[0]];
        let pad = |v: [f32; 3]| [v[0], v[1], v[2], 0.0];
        let bone = [pad(right), pad(up), pad(fwd), [origin[0], origin[1], origin[2], 1.0]];
        self.eject_bone(ammo, weapon, &bone, shooter_y, rng);
    }

    /// fix-sg5-casings: the game's own eject transform [M Bullet_init_casing_ex 0x12f678:83-160]:
    /// the weapon-local vector (`+0xf8, −+0xfc, +0x100` — the +0xfc negation at 0x12f83c)
    /// through the WEAPON DATUM's bone world matrix (`AnimGetBoneWorldTrans(shooter, datum)`, the
    /// view-model gun `Sim::gun_matrix()` stands in for it), re-positioned at that bone's VECTOR out
    /// (`Vec_Copy2(bone_pos, obj+0x30, obj+0x40)`). `bone` is row-major like the game's MATRIX use
    /// (`ApplyMatrixLVI`): world = local·M, so row 0 carries the datum's +x (the viewer LEFT —
    /// `Player_PositionGun` 0x1a3b40's gl), row 1 +y (up), row 2 +z (the barrel forward), and
    /// `bone[3]` is the eject origin. RNG order [M 0x12f860..0x12f8f0]: 3×
    /// `Rand_FRand_MVar2(0.1, 0.05)` (spin), 3× `(0.01, 0.005)` (velocity jitter), 1×
    /// `Rand_FRand(0.5)` (landing plane) — identical to `eject`, the shared RNG stays bit-exact.
    pub fn eject_bone(&mut self, ammo: &nf_formats::sfx::AmmoData, weapon: &[u8], bone: &[[f32; 4]; 4], shooter_y: f32, rng: &mut Rng) {
        let delay = weapon[0x78] as f32;
        let ammo_byte = weapon[0x90] as usize;
        let Some(&mesh) = ammo.casing_mesh.get(ammo_byte) else { return };
        if mesh == 0 { return; }
        let f = |o: usize| f32::from_le_bytes(weapon[o..o + 4].try_into().unwrap());
        let local = [f(0xf8), -f(0xfc), f(0x100)];
        let spin = [frand_mvar2(rng, 0.1, 0.05), frand_mvar2(rng, 0.1, 0.05), frand_mvar2(rng, 0.1, 0.05)];
        let jitter = [frand_mvar2(rng, 0.01, 0.005), frand_mvar2(rng, 0.01, 0.005), frand_mvar2(rng, 0.01, 0.005)];
        let e = [
            bone[0][0] * local[0] + bone[1][0] * local[1] + bone[2][0] * local[2],
            bone[0][1] * local[0] + bone[1][1] * local[1] + bone[2][1] * local[2],
            bone[0][2] * local[0] + bone[1][2] * local[1] + bone[2][2] * local[2],
        ];
        let vel = [e[0] + jitter[0], e[1] + jitter[1], e[2] + jitter[2]];
        let landing_y = shooter_y - (rng.frand(0.5) + 1.0);
        let material = (weapon[0x50] & 0x3f) as u16;
        let origin = [bone[3][0], bone[3][1], bone[3][2]];
        let slot = &mut self.list[self.next];
        *slot = Casing {
            state: 3, delay, pos: origin, vel, spin, rot: 0.0, age: 0.0, landing_y, mesh, material, landed: false,
        };
        self.next = (self.next + 1) % POOL;
    }

    /// `Bullet_casing_update` 0x12faa0 per tick. `play` receives the landing sound id. Sound table
    /// [M 0x12fb48..0x12fbcc]: the default shell mesh (0x2000548) with material ∉ {0xd, 0xe} → 0x1fd;
    /// otherwise by material: 1/0x14 → 0x1ff, 3/4 → 0x203, 5 → 0x204, 6 → 0x200, 10/0x15 → 0x1fe,
    /// 0xd/0xe → 0x202, 0xf..0x12 → 0x201, else silent.
    pub fn update(&mut self, mut play: impl FnMut(u16, V3)) {
        for c in self.list.iter_mut() {
            match c.state {
                3 => {
                    c.delay -= 1.0;
                    if c.delay <= 0.0 { c.state = 0; }
                }
                0 => {
                    // Age first (the game adds FRAME_RATE_MUL to data+0x24 before the tests) [M 0x12fb30].
                    c.age += 1.0;
                    // The landing: the sound fires while the casing is at/below its plane
                    // (`landing_y >= pos.y`) [M 0x12fb40]; the port plays it once.
                    if !c.landed && c.landing_y >= c.pos[1] {
                        if let Some(id) = landing_sound(c.mesh, c.material) {
                            play(id, c.pos);
                        }
                        c.landed = true;
                    }
                    if c.age > LIFETIME { c.state = 2; continue; }
                    // Flight: pos += vel; vel += WldGravity·REC² (state 0's factor is 1) [M 0x12fcb8..0x12fd10].
                    c.pos = [c.pos[0] + c.vel[0], c.pos[1] + c.vel[1], c.pos[2] + c.vel[2]];
                    c.vel[1] += GRAVITY * REC * REC;
                    c.rot += c.spin[0] + c.spin[1] + c.spin[2];
                }
                _ => {}
            }
        }
    }

    /// The casings the renderer should draw this frame (state 0, the game's `View_SetDrawInAllViews`).
    pub fn visible(&self) -> impl Iterator<Item = &Casing> { self.list.iter().filter(|c| c.state == 0) }
}

/// The landing-sound switch of `Bullet_casing_update` [M 0x12fb48..0x12fbcc].
pub fn landing_sound(mesh: u32, material: u16) -> Option<u16> {
    // The default shell mesh falls through to the switch only for 0xd/0xe [M 0x12fb48].
    if mesh == DEFAULT_MESH && material != 0xd && material != 0xe { return Some(0x1fd); }
    Some(match material {
        1 | 0x14 => 0x1ff,
        3 | 4 => 0x203,
        5 => 0x204,
        6 => 0x200,
        10 | 0x15 => 0x1fe,
        0xd | 0xe => 0x202,
        0xf..=0x12 => 0x201,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weapon(delay: u8, ammo: u8) -> [u8; 0x10c] {
        let mut w = [0u8; 0x10c];
        w[0x78] = delay;
        w[0x90] = ammo;
        w[0x50] = 0x03; // wood → landing sound 0x203
        // Local eject velocity: up and to the side (the game's weapon fields).
        w[0xf8..0xfc].copy_from_slice(&0.5f32.to_le_bytes());
        w[0xfc..0x100].copy_from_slice(&1.0f32.to_le_bytes());
        w[0x100..0x104].copy_from_slice(&0.2f32.to_le_bytes());
        w
    }

    fn ammo(mesh: u32) -> nf_formats::sfx::AmmoData {
        nf_formats::sfx::AmmoData { casing_mesh: vec![0, mesh, 0, 0] }
    }

    /// One shot ejects one casing after the weapon's delay; it falls under gravity and lands on the
    /// plane within its 240-tick lifetime, playing the material's sound [M 0x12f678/0x12faa0].
    #[test]
    fn one_shot_ejects_one_landing_casing() {
        let mut cs = Casings::new();
        // A non-default mesh so the landing takes the material switch (wood 3 → 0x203).
        let a = ammo(0x0200_0549);
        let mut rng = Rng::new(11);
        cs.eject(&a, &weapon(0, 1), [10.0, 1.0, 10.0], 0.0, 0.0, 1.0, &mut rng);
        assert_eq!(cs.list.iter().filter(|c| c.state == 3).count(), 1, "exactly one slot armed");
        let mut sounds = Vec::new();
        for _ in 0..240 { cs.update(|id, p| sounds.push((id, p))); }
        let c = cs.list.iter().find(|c| c.material == 3).unwrap();
        assert!(c.landed, "the casing landed within its lifetime");
        assert!(c.pos[1] <= c.landing_y + 1e-3, "it fell below the landing plane");
        assert_eq!(sounds.len(), 1, "the landing sound plays once");
        assert_eq!(sounds[0].0, 0x203, "wood material → sound 0x203 [M 0x12fbcc]");
        assert!(c.pos[1] < 1.0, "it fell from the gun toward the plane");
    }

    /// The pool is a 32-slot round-robin: the 33rd eject reuses the oldest slot [M 0x12fd48,
    /// `NextCasing = (NextCasing + 1) % 32` via the ±0x20 wrap].
    #[test]
    fn pool_round_robin_is_32() {
        let mut cs = Casings::new();
        let a = ammo(0x0200_0548);
        let mut rng = Rng::new(2);
        for k in 0..33 {
            cs.eject(&a, &weapon(0, 1), [k as f32, 1.0, 0.0], 0.0, 0.0, 1.0, &mut rng);
            for c in cs.list.iter_mut() { if c.state == 3 { c.state = 0; } }
        }
        assert_eq!(cs.list.iter().filter(|c| c.state == 0).count(), 32, "the pool never grows past 32");
        assert!((cs.list[0].pos[0] - 32.0).abs() < 1e-4, "slot 0 was reused by the 33rd eject (k=32)");
    }

    /// The landing-sound switch [M 0x12fb48..0x12fbcc]: the default shell mesh with a non-0xd/0xe
    /// material → 0x1fd; metal 5 → 0x204; 0xd/0xe → 0x202 even for the default mesh; other materials
    /// fall through silent.
    #[test]
    fn landing_sound_table() {
        // The default shell mesh only falls through to the material switch for 0xd/0xe [M 0x12fb48].
        assert_eq!(landing_sound(DEFAULT_MESH, 0), Some(0x1fd));
        assert_eq!(landing_sound(DEFAULT_MESH, 5), Some(0x1fd));
        assert_eq!(landing_sound(DEFAULT_MESH, 0xd), Some(0x202));
        assert_eq!(landing_sound(DEFAULT_MESH, 0xe), Some(0x202));
        // A different mesh takes the switch for every listed material.
        assert_eq!(landing_sound(0x1234, 5), Some(0x204));
        assert_eq!(landing_sound(0x1234, 0xd), Some(0x202));
        assert_eq!(landing_sound(0x1234, 0x11), Some(0x201));
        assert_eq!(landing_sound(0x1234, 0x33), None, "unknown material: silent");
    }

    /// The RNG draw order per eject [M 0x12f860..0x12f8f0]: 3× MVar2(0.1, 0.05), 3× MVar2(0.01, 0.005),
    /// 1× FRand(0.5) — 7 draws, mirrored against a hand-driven stream.
    #[test]
    fn eject_draws_seven_in_game_order() {
        let mut cs = Casings::new();
        let a = ammo(0x0200_0548);
        let mut rng = Rng::new(9);
        let (x0, y0) = (rng.x, rng.y);
        cs.eject(&a, &weapon(0, 1), [0.0; 3], 0.0, 0.0, 1.0, &mut rng);
        // Replay the same 7 draws by hand.
        let mut ref_rng = Rng::new(9);
        let mut want_spin = [0.0f32; 3];
        let mut want_vel = [0.0f32; 3];
        for v in want_spin.iter_mut() { *v = frand_mvar2(&mut ref_rng, 0.1, 0.05); }
        for v in want_vel.iter_mut() { *v = frand_mvar2(&mut ref_rng, 0.01, 0.005); }
        let want_land = ref_rng.frand(0.5);
        let c = &cs.list[0];
        assert_eq!((rng.x, rng.y), (ref_rng.x, ref_rng.y), "exactly 7 draws consumed");
        for k in 0..3 {
            assert!((c.spin[k] - want_spin[k]).abs() < 1e-7);
            // Identity basis (yaw = pitch = 0): velocity = the local eject vector + the jitter.
            let local = [0.5f32, -1.0, 0.2][k];
            assert!((c.vel[k] - (local + want_vel[k])).abs() < 1e-6, "velocity = local eject + jitter");
        }
        assert!((c.landing_y - (1.0 - (want_land + 1.0))).abs() < 1e-6, "landing plane = shooter_y − (1 + FRand(0.5))");
        let _ = (x0, y0);
    }
}
