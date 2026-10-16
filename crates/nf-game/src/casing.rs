//! Ejected shell casings: `Bullet_InitCasingLists` 0x12fd48 (the 32-object round-robin pool),
//! `Bullet_init_casing` 0x12fde0 (the delayed activation), `Bullet_init_casing_ex` 0x12f678 (the
//! eject: view-model bone transform, `Rand_FRand_MVar2` velocity/spin jitter, the landing plane)
//! and `Bullet_casing_update` 0x12faa0 (gravity, the landing sound by material, the 240-tick life).
//! Fired from `Player_WeaponInitBullet` 0x1a4bf0 (Bond, after the bullet's spread draws) and
//! `DroneWeap_FireWeapon` 0x178f68 (guards, after `Bullet_init`, before `Rand_Rand(500)`).
//! G-025 / docs/atlas/effect.md.
//!
//! Post-landing, casings-persist [M]: the armed slot's class byte is 0x0d (`Bullet_init_casing`
//! 0x12fde0:24) and `control_funcs` 0x26f890 (class 0x0d) gives update 0x12faa0, collide NULL —
//! casings never collide with the world. The landing plane (data+0x20) is a SOUND-ONLY trigger:
//! on the first tick with pos.y <= plane the sound plays and the plane is parked at −1000000.0
//! (0xc9742400 [M 0x12fb88-0x12fb94]), so the sound fires once and the casing keeps falling
//! through the floor until the despawn at age > 240 (state 2, `View_SetDrawInNoViews`, age =
//! 32767.0 [M 0x12fc58-0x12fc94]). The update's state-1 arm (gravity ×0.1) is unreachable for
//! casings: nothing sets state 1 on a class-0x0d object (the state-1 writers are the class-5
//! bullet paths, `Bullet_CollisionHandler` 0x12ce80 / `Bullet_update` 0x12e7b0). No fade — the
//! casing draws until it despawns (the game applies no alpha to class-0x0d objects).
//! The PC-port option `Casings::set_keep(n)` (0 = the game's behaviour) rests a casing on its
//! landing plane when it crosses and holds the newest N resting ones, recycling oldest-first.

use crate::drone::Rng;
use crate::effect::frand_mvar2;
use crate::world::V3;

/// `REC_FRAME_RATE` [M grenade.rs:52]; the casing physics scale `REC_FRAME_RATE²` in the update.
const REC: f32 = 1.0 / 60.0;
/// `_WldGravity` [L grenade.rs:51].
const GRAVITY: f32 = -9.8;
/// `Bullet_casing_update` 0x12faa0: `if (240.0 < age) → state 2, undrawn` [M 0x12fc80].
const LIFETIME: f32 = 240.0;
/// The pool: `Bullet_InitCasingLists` 0x12fd48 creates 32 objects (the 0x40 there is the ALLOC
/// size — `control_create_object` mallocs param+0x110), armed slots become class 0x0d
/// [M 0x12fde0:24], `NextCasing` round-robin [M 0x12fde0:16-21].
pub const POOL: usize = 32;
/// The default shell mesh the collision handler special-cases (0x2000548) [M 0x12fb48].
pub const DEFAULT_MESH: u32 = 0x0200_0548;
/// Where the game parks the landing plane after the sound: 0xc9742400 = −1000000.0
/// [M Bullet_casing_update 0x12faa0 @0x12fb88-0x12fb94] — the sound can never re-fire.
pub const REST_PLANE: f32 = -1_000_000.0;
/// The despawned slot's age store [M 0x12fc84-0x12fc94] (data+0x24 = 0x46fffe00 = 32767.0).
pub const DEAD_AGE: f32 = 32767.0;
/// The "Keep shell casings" option's maximum — nf-view's casing attachment pool sizes to this so
/// every kept casing has a draw slot (data/sim only otherwise).
pub const KEEP_MAX: usize = 128;

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
    /// The landing sound plays once [M 0x12fb88-0x12fb94: the game parks the plane at −1000000.0
    /// after the sound, so the crossing test can never pass again]. The port mirrors that with the
    /// `REST_PLANE` store and keeps this flag derived (set on the crossing tick) for readers.
    pub landed: bool,
    /// The player's shell waits for its eject (`Bullet_init_casing` 0x12fde0 only reserves the slot and the
    /// delay; `Bullet_init_casing_ex` 0x12f678 reads the view model's datum and draws the RNG when the delay
    /// runs out [M 0x12faa0:31-37]) — `local` is the weapon's eject vector (+0xf8, −+0xfc, +0x100).
    pub pending: bool,
    pub local: V3,
    /// casings-persist [S; PC-port only, never set when keep = 0]: the casing rested on its landing
    /// plane (`Casings::set_keep` > 0) — pos sits at the plane, vel/spin are zeroed, the flight
    /// integration stops and the 240-tick despawn no longer applies (the game has no rest at all:
    /// class 0x0d has no collide handler, the casing sinks through the floor until it despawns).
    pub resting: bool,
}

/// The 32-slot pool (`ShellCasings[32]` + `NextCasing`, 0x3495e0/0x3495dc [M 0x12fd48]).
#[derive(Clone, Debug)]
pub struct Casings {
    pub list: Vec<Casing>,
    next: usize,
    /// The PC-port "Keep shell casings: N" option (0 = the game's behaviour). 0 rests nothing and
    /// keeps the game's exact pool + 240-tick despawn; N > 0 grows the pool to max(32, N) and rests
    /// each casing on its landing plane, holding the newest N resting ones (the pool recycles
    /// oldest-first, the game's NextCasing order [M 0x12fde0:16-21]).
    keep: usize,
    /// Every slot takeover since the pool was created — one per actual eject (the player's
    /// reserve 0x12fde0 or a guard's `eject_bone` 0x12f678; the mesh-less early returns don't
    /// take a slot [M 0x12fde0:13-15]). A stat for tests and the eventual options UI.
    pub ejected: u32,
}

impl Default for Casings { fn default() -> Self { Self::new() } }

impl Casings {
    pub fn new() -> Casings { Casings { list: (0..POOL).map(|_| Casing {
        state: 2, delay: 0.0, pos: [0.0; 3], vel: [0.0; 3], spin: [0.0; 3], rot: 0.0, age: 0.0,
        landing_y: 0.0, mesh: 0, material: 0, landed: false, pending: false, local: [0.0; 3],
        resting: false,
    }).collect(), next: 0, keep: 0, ejected: 0 } }

    /// The PC-port option's current value (0 = the game's behaviour).
    pub fn keep(&self) -> usize { self.keep }

    /// "Keep shell casings: N" (0, 32, 64, 128 — any N clamps the POOL to max(32, N)). The game's
    /// own pool is the 32-object round-robin [M 0x12fd48]; N > 0 extends it (the new slots start
    /// dead like `Bullet_InitCasingLists`' do [M 0x12fd48:14-17]) and turns the landing into a
    /// rest. Deterministic: tick-counted, no wall-clock. Call after a scene load (a new `Sim`
    /// starts at the game's default 0).
    pub fn set_keep(&mut self, n: usize) {
        self.keep = n.min(KEEP_MAX);
        let want = POOL.max(self.keep);
        while self.list.len() < want {
            self.list.push(Casing {
                state: 2, delay: 0.0, pos: [0.0; 3], vel: [0.0; 3], spin: [0.0; 3], rot: 0.0, age: 0.0,
                landing_y: 0.0, mesh: 0, material: 0, landed: false, pending: false, local: [0.0; 3],
                resting: false,
            });
        }
        // Dropping the option releases the extension slots (their casings leave with them).
        if self.list.len() > want {
            self.list.truncate(want);
        }
        if self.next >= self.list.len() { self.next = 0; }
    }

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
            pending: false, local, resting: false,
        };
        self.ejected += 1;
        self.next = (self.next + 1) % self.list.len();
    }

    /// The player's shot: `Bullet_init_casing` 0x12fde0 — reserve the next pool slot (state 3, the weapon's
    /// `+0x78` delay, the ammo type's shell mesh) and nothing else; the eject itself (datum, RNG, landing
    /// plane) waits for the delay [M 0x12fde0:9-30]. The Frinesi's 53-tick delay puts its shell out mid-pump,
    /// next to the camera (PCSX2 07000001, 2026-10-15) — computed at the shot it left from the pre-pump gun.
    pub fn reserve(&mut self, ammo: &nf_formats::sfx::AmmoData, weapon: &[u8]) {
        let Some(&mesh) = ammo.casing_mesh.get(weapon[0x90] as usize) else { return };
        if mesh == 0 { return; }
        let f = |o: usize| f32::from_le_bytes(weapon[o..o + 4].try_into().unwrap());
        let slot = &mut self.list[self.next];
        *slot = Casing {
            state: 3, delay: weapon[0x78] as f32, pos: [0.0; 3], vel: [0.0; 3], spin: [0.0; 3], rot: 0.0, age: 0.0,
            landing_y: 0.0, mesh, material: (weapon[0x50] & 0x3f) as u16, landed: false, pending: true,
            local: [f(0xf8), -f(0xfc), f(0x100)], resting: false,
        };
        self.ejected += 1;
        self.next = (self.next + 1) % self.list.len();
    }

    /// `Bullet_init_casing_ex` 0x12f678 for a reserved player shell: the datum's world matrix NOW, the 7 RNG
    /// draws in the game's order, the landing plane from the shooter's y.
    fn arm(c: &mut Casing, bone: &[[f32; 4]; 4], shooter_y: f32, rng: &mut Rng) {
        let local = c.local;
        let spin = [frand_mvar2(rng, 0.1, 0.05), frand_mvar2(rng, 0.1, 0.05), frand_mvar2(rng, 0.1, 0.05)];
        let jitter = [frand_mvar2(rng, 0.01, 0.005), frand_mvar2(rng, 0.01, 0.005), frand_mvar2(rng, 0.01, 0.005)];
        let e = [
            bone[0][0] * local[0] + bone[1][0] * local[1] + bone[2][0] * local[2],
            bone[0][1] * local[0] + bone[1][1] * local[1] + bone[2][1] * local[2],
            bone[0][2] * local[0] + bone[1][2] * local[1] + bone[2][2] * local[2],
        ];
        c.vel = [e[0] + jitter[0], e[1] + jitter[1], e[2] + jitter[2]];
        c.spin = spin;
        c.landing_y = shooter_y - (rng.frand(0.5) + 1.0);
        c.pos = [bone[3][0], bone[3][1], bone[3][2]];
        c.pending = false;
    }

    /// `Bullet_casing_update` 0x12faa0 per tick. `play` receives the landing sound id. Sound table
    /// [M 0x12fb48..0x12fbcc]: the default shell mesh (0x2000548) with material ∉ {0xd, 0xe} → 0x1fd;
    /// otherwise by material: 1/0x14 → 0x1ff, 3/4 → 0x203, 5 → 0x204, 6 → 0x200, 10/0x15 → 0x1fe,
    /// 0xd/0xe → 0x202, 0xf..0x12 → 0x201, else silent.
    /// `player_bone`: the view model's eject datum matrix this tick and the player's y, for reserved player
    /// shells (`None` with no view model — a due shell is dropped).
    pub fn update(&mut self, player_bone: Option<(&[[f32; 4]; 4], f32)>, rng: &mut Rng, mut play: impl FnMut(u16, V3)) {
        for c in self.list.iter_mut() {
            match c.state {
                3 => {
                    c.delay -= 1.0;
                    if c.delay <= 0.0 {
                        if c.pending {
                            match player_bone {
                                Some((bone, y)) => { Self::arm(c, bone, y, rng); c.state = 0; }
                                None => { c.pending = false; c.state = 2; }
                            }
                        } else { c.state = 0; }
                    }
                }
                0 => {
                    // Age first (the game adds FRAME_RATE_MUL to data+0x24 before the tests) [M 0x12fb44-58].
                    c.age += 1.0;
                    // The landing: the sound fires while the casing is at/below its plane, tested
                    // against the PRE-integration pos [M 0x12fb70-0x12fb80], and the game parks the
                    // plane at −1000000.0 right after the sound so it can never re-fire
                    // [M 0x12fb88-0x12fb94] (the old [S] "re-triggers every tick" is wrong).
                    if !c.resting && c.landing_y >= c.pos[1] {
                        let plane = c.landing_y;
                        if let Some(id) = landing_sound(c.mesh, c.material) {
                            play(id, c.pos);
                        }
                        c.landing_y = REST_PLANE;
                        c.landed = true;
                        // casings-persist [S]: with the keep option the crossing becomes a rest —
                        // the casing settles exactly on its plane (the game itself keeps falling
                        // through the floor: class 0x0d has no collide handler). No RNG draws.
                        if self.keep > 0 {
                            c.resting = true;
                            c.pos[1] = plane;
                            c.vel = [0.0; 3];
                            c.spin = [0.0; 3];
                        }
                    }
                    // The despawn: age > 240 → state 2, undrawn, age = 32767.0 [M 0x12fc58-0x12fc94].
                    // With keep > 0 a resting casing is held (the option's point); an in-flight one
                    // dies exactly as the game's.
                    if c.age > LIFETIME && !(c.resting && self.keep > 0) {
                        c.state = 2;
                        c.age = DEAD_AGE;
                        // No `continue`: the game still integrates this tick's motion after the
                        // despawn stores (0x12fc94 falls through to the flight at 0x12fc98) [M].
                    }
                    // Flight: pos += vel; vel += WldGravity·REC² (state 0's factor is 1) [M 0x12fcb8..0x12fd10].
                    // A rested casing (keep > 0) is frozen: no integration, no despawn.
                    if !c.resting {
                        c.pos = [c.pos[0] + c.vel[0], c.pos[1] + c.vel[1], c.pos[2] + c.vel[2]];
                        c.vel[1] += GRAVITY * REC * REC;
                        c.rot += c.spin[0] + c.spin[1] + c.spin[2];
                    }
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

    /// One shot ejects one casing after the weapon's delay; it falls under gravity, crosses the
    /// plane, plays the material's sound ONCE (the plane parks at −1000000.0 [M 0x12fb88-94]) and
    /// keeps falling — the game's casings never collide [M control_funcs 0x26f890 class 0x0d] —
    /// until the despawn at age > 240 (state 2, age 32767 [M 0x12fc58-94]).
    #[test]
    fn one_shot_ejects_one_landing_casing() {
        let mut cs = Casings::new();
        // A non-default mesh so the landing takes the material switch (wood 3 → 0x203).
        let a = ammo(0x0200_0549);
        let mut rng = Rng::new(11);
        cs.eject(&a, &weapon(0, 1), [10.0, 1.0, 10.0], 0.0, 0.0, 1.0, &mut rng);
        assert_eq!(cs.list.iter().filter(|c| c.state == 3).count(), 1, "exactly one slot armed");
        let mut sounds = Vec::new();
        for _ in 0..260 { cs.update(None, &mut Rng::new(1), |id, p| sounds.push((id, p))); }
        let c = cs.list.iter().find(|c| c.material == 3).unwrap();
        assert!(c.landed, "the casing crossed its landing plane within its lifetime");
        assert_eq!(c.landing_y, REST_PLANE, "the game parks the plane at −1000000.0 after the sound [M 0x12fb88-94]");
        assert_eq!(c.age, DEAD_AGE, "the despawned slot's age store [M 0x12fc84-94]");
        assert_eq!(c.state, 2, "despawned at age > 240 [M 0x12fc78]");
        assert_eq!(sounds.len(), 1, "the landing sound plays once (the parked plane can never re-fire)");
        assert_eq!(sounds[0].0, 0x203, "wood material → sound 0x203 [M 0x12fbcc]");
        assert!(c.pos[1] < 1.0, "it fell from the gun toward and past the plane (no collision [M])");
        assert!(!c.resting, "keep = 0: the game's exact behaviour, no rest");
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

    /// The game's default: fire 40 rounds and the 32-slot pool caps the count at 32 (the 33rd eject
    /// recycled slot 0, round-robin [M 0x12fde0:16-21]) and every casing still despawns at its
    /// 240-tick lifetime [M 0x12fc58-0x12fc94] — with keep = 0 nothing is held.
    #[test]
    fn default_keep_forty_rounds_cap_at_32_and_all_despawn_at_240() {
        let mut cs = Casings::new();
        assert_eq!(cs.keep(), 0, "the default is the game's behaviour");
        let a = ammo(0x0200_0548);
        let mut rng = Rng::new(7);
        // Fire 40 rounds, one tick of flight between shots (a fast automatic).
        for k in 0..40 {
            cs.eject(&a, &weapon(0, 1), [k as f32, 1.0, 0.0], 0.0, 0.0, 1.0, &mut rng);
            for c in cs.list.iter_mut() { if c.state == 3 { c.state = 0; } }
            cs.update(None, &mut Rng::new(1), |_, _| {});
            assert!(cs.list.iter().filter(|c| c.state == 0).count() <= POOL, "the pool never exceeds 32 [M 0x12fd48]");
        }
        assert_eq!(cs.list.iter().filter(|c| c.state == 0).count(), 32, "40 rounds, 32 visible: the oldest 8 were recycled");
        // Slot 0 holds the 33rd shot (k=32): recycled mid-loop, so it is the YOUNGEST life in the
        // pool — 8 updates old (iterations k=32..39) while the untouched slots run 32..39.
        assert!((cs.list[0].age - 8.0).abs() < 1e-4, "slot 0 was recycled by the 33rd eject (round-robin, oldest-first)");
        // The lifetime: tick until every casing's age passes 240 — all despawn, none held.
        for _ in 0..400 { cs.update(None, &mut Rng::new(1), |_, _| {}); }
        assert_eq!(cs.list.iter().filter(|c| c.state != 2).count(), 0, "keep = 0: the 240-tick lifetime kills everything [M 0x12fc68]");
    }

    /// keep = 64: the resting casings are held past their 240-tick lifetime and the pool recycles
    /// oldest-first — 40 rounds all stay (the pool grew to 64); the 65th eject recycles slot 0.
    #[test]
    fn keep_64_holds_the_resting_casings_and_recycles_oldest_first() {
        let mut cs = Casings::new();
        cs.set_keep(64);
        assert_eq!(cs.keep(), 64);
        assert_eq!(cs.list.len(), 64, "the pool grows to max(32, keep) [S]");
        let a = ammo(0x0200_0548);
        let mut rng = Rng::new(7);
        for k in 0..40 {
            cs.eject(&a, &weapon(0, 1), [k as f32, 1.0, 0.0], 0.0, 0.0, 1.0, &mut rng);
            for c in cs.list.iter_mut() { if c.state == 3 { c.state = 0; } }
            cs.update(None, &mut Rng::new(1), |_, _| {});
        }
        // Every casing crossed its plane quickly (origin y 1.0, plane ≤ 1.0): all rest, none despawn.
        for _ in 0..400 { cs.update(None, &mut Rng::new(1), |_, _| {}); }
        let alive: Vec<&Casing> = cs.list.iter().filter(|c| c.state != 2).collect();
        assert_eq!(alive.len(), 40, "keep = 64 holds all 40 resting casings past the 240-tick lifetime");
        assert!(alive.iter().all(|c| c.resting), "each held casing rested on its landing plane");
        assert!(alive.iter().all(|c| c.vel == [0.0; 3] && c.spin == [0.0; 3]), "a rested casing is frozen");
        // The pool recycles oldest-first: 25 more rounds reuse slots 0..24 (shots 1..25).
        for k in 40..65 {
            cs.eject(&a, &weapon(0, 1), [k as f32, 1.0, 0.0], 0.0, 0.0, 1.0, &mut rng);
            for c in cs.list.iter_mut() { if c.state == 3 { c.state = 0; } }
            cs.update(None, &mut Rng::new(1), |_, _| {});
        }
        // The 65th eject (k=64) recycled slot 0, the oldest — one tick into its flight.
        assert!((cs.list[0].pos[0] - 64.5).abs() < 0.05, "the 65th eject recycled slot 0, the oldest resting casing");
        assert_eq!(cs.list.iter().filter(|c| c.state != 2).count(), 64, "the pool holds the newest 64 resting casings");
        // The recycled slot is a fresh shot: not resting yet, flying.
        assert!(!cs.list[0].resting && cs.list[0].state == 0, "a recycled slot flies again");
    }

    /// Sister: the flight is untouched by the option — the same seed fires the same shots with
    /// keep = 0 and keep = 64 and every in-flight tick matches bit-for-bit (pos, vel, the sound
    /// schedule); the paths split ONLY at the plane crossing, where the game's casing keeps
    /// falling through the floor [M: class 0x0d has no collide handler] while the keep=64 casing
    /// rests exactly on its plane, frozen.
    #[test]
    fn in_flight_flight_is_identical_regardless_of_keep() {
        let a = ammo(0x0200_0548);
        let mut game = Casings::new();
        let mut kept = Casings::new();
        kept.set_keep(64);
        let mut rng_g = Rng::new(21);
        let mut rng_k = Rng::new(21);
        // shooter_y 2.0 → each plane is 2.0 − (1 + FRand(0.5)); replay the draw groups for it.
        let mut rr = Rng::new(21);
        let mut planes = [0.0f32; 3];
        for p in planes.iter_mut() {
            for _ in 0..3 { let _ = frand_mvar2(&mut rr, 0.1, 0.05); }
            for _ in 0..3 { let _ = frand_mvar2(&mut rr, 0.01, 0.005); }
            *p = 2.0 - (rr.frand(0.5) + 1.0);
        }
        for k in 0..3u32 {
            let origin = [k as f32 * 3.0, 2.0, 0.5];
            game.eject(&a, &weapon(0, 1), origin, 0.0, 0.1, 2.0, &mut rng_g);
            kept.eject(&a, &weapon(0, 1), origin, 0.0, 0.1, 2.0, &mut rng_k);
            for c in game.list.iter_mut() { if c.state == 3 { c.state = 0; } }
            for c in kept.list.iter_mut() { if c.state == 3 { c.state = 0; } }
        }
        assert_eq!((rng_g.x, rng_g.y), (rng_k.x, rng_k.y), "the option draws no extra RNG");
        let mut sounds_g = Vec::new();
        let mut sounds_k = Vec::new();
        let mut split_at = [None; 3]; // the tick each slot's paths split (the landing)
        for t in 0..300u32 {
            game.update(None, &mut Rng::new(1), |id, p| sounds_g.push((t, id, p)));
            kept.update(None, &mut Rng::new(1), |id, p| sounds_k.push((t, id, p)));
            for slot in 0..POOL {
                let (g, k) = (&game.list[slot], &kept.list[slot]);
                if k.resting {
                    if split_at[slot].is_none() {
                        split_at[slot] = Some(t);
                        assert!(!g.resting, "keep = 0 never rests [M: the game has no rest]");
                        assert_eq!(k.pos[1], planes[slot], "the rest sits exactly on the landing plane");
                        assert_eq!((k.vel, k.spin), ([0.0; 3], [0.0; 3]), "the rest is frozen");
                    }
                    // The game's casing is still falling below the rest point.
                    if g.state == 0 { assert!(g.pos[1] < k.pos[1], "the game's casing sinks past the plane"); }
                } else if g.state == 0 && k.state == 0 && g.state == k.state {
                    assert_eq!(g.pos, k.pos, "tick {t} slot {slot}: identical flight before the landing");
                    assert_eq!(g.vel, k.vel, "tick {t} slot {slot}: identical velocity before the landing");
                }
            }
        }
        assert_eq!(sounds_g, sounds_k, "the sound schedule is identical (the landing tick is the same)");
        assert_eq!(sounds_g.len(), 3, "exactly one landing sound per casing (the plane parks [M 0x12fb88-94])");
        assert!(split_at.iter().all(|s| s.is_some()), "every casing landed and split");
        assert_eq!(game.list.iter().filter(|c| c.state != 2).count(), 0, "keep = 0: all sunk casings despawned at 240 [M]");
        assert_eq!(kept.list.iter().filter(|c| c.state != 2).count(), 3, "keep = 64: the resting casings are held");
    }
}
