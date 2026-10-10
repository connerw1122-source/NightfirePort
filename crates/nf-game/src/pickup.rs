//! Pickups: guns dropped by drones (`DroneWeap_DropWeapon` 0x17a2a0, `DroneFunc_DropGrenade`), their fall and lifetime
//! (`Pickup_Update` 0x18e148), Bond taking them (`Pickup_Handler` 0x18e650) into his inventory (`Player_EquipWeapon`
//! 0x1a8a58, `Player_EquipAmmo` 0x1a8d18). Spec: docs/systems/pickups.md.

use crate::world::{CollisionWorld, V3};
use nf_formats::anim::Mat4;
use nf_formats::weapon::WeaponData;

/// The sound `Pickup_CreateSimple` gives a pickup (`Sound_Play3D` when taken).
pub const SOUND: u32 = 0xf5;
/// `FRAME_RATE_INT · 120` (single player) with FRAME_RATE_INT = 60 [L]: a dropped gun lasts two minutes.
pub const DROP_LIFETIME: u16 = 60 * 120;
/// `WldGravity.y · REC_FRAME_RATE²` [L]: the fall per tick².
const GRAVITY: f32 = -9.8 * (1.0 / 60.0) * (1.0 / 60.0);

/// A model's glist bounds (`celglist+0x20`: centre, radius).
#[derive(Clone, Copy, Debug)]
pub struct Model { pub centre: V3, pub radius: f32 }

/// The glist bounds of every model of the level, by hashcode (first wins).
pub fn models(level: &nf_formats::level::Level) -> std::collections::HashMap<u32, Model> {
    let mut out = std::collections::HashMap::new();
    for m in &level.maps {
        for o in &m.objects { out.entry(o.hashcode).or_insert(Model { centre: o.centre, radius: o.radius }); }
    }
    out
}

/// PICKUPINFO `+0x22`.
pub mod kind {
    pub const WEAPON: u16 = 0;
    pub const AMMO: u16 = 1;
    pub const KEY: u16 = 2;
    pub const ARMOUR: u16 = 3;
    pub const BOND_BONUS: u16 = 6;
}

/// One pickup object (`control_create_object(0x40)` + PICKUPINFO at `obj+0xE0`).
#[derive(Clone, Debug)]
pub struct Pickup {
    pub kind: u16,
    pub weapon: u16,
    pub rounds: i16,
    /// The model (`weapon_data+0x80`) and its bounds.
    pub model: u32,
    pub bounds: Model,
    /// `obj+0x90`: rows 0–2 the axes, row 3 the translation.
    pub m: Mat4,
    /// `obj+0x40`: where the fall ray starts.
    pub start: V3,
    pub vel: V3,
    /// `+0x20`: 0 falling, 1 resting.
    pub state: u8,
    /// `+0x2E`: ticks left (0 = forever).
    pub lifetime: u16,
    /// `+0x2A`: the sound when taken.
    pub sound: Option<u32>,
    /// `obj+0xF0 & 0x10` (U-036).
    pub hidden: bool,
    /// Deleted (`obj+0xFE |= 1`).
    pub gone: bool,
    /// The drone that dropped it (index into `Drones::list`), for drawing.
    pub from: Option<usize>,
}

/// What Bond took.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Taken { Weapon { weapon: u16 }, Ammo { weapon: u16, rounds: i16 } }

fn trans(m: &Mat4) -> V3 { [m[3][0], m[3][1], m[3][2]] }

impl Pickup {
    /// `Pickup_CreateSimple(M, id, rounds, kind)` with the single-player stack arguments (sound 0xF5, dropped to the
    /// floor, no respawn, lifetime 7200). `None` as the game refuses: no rounds, ids it never creates, no model.
    pub fn create_simple(m: &Mat4, id: u16, rounds: i16, kind: u16, w: &WeaponData, models: &std::collections::HashMap<u32, Model>) -> Option<Pickup> {
        if rounds == 0 { return None; }
        let id = match id { 0x6a | 0x6e => 0x33, 1 | 0x45 | 0x47 | 0x50 | 0x51 | 0x53 => return None, i => i };
        if (id as u32).wrapping_sub(1) > 0x70 { return None; }
        let model = w.model(id as usize);
        let bounds = *models.get(&model)?;
        Some(Pickup::create(m, w.id(id as usize), rounds, kind, model, bounds))
    }

    /// `Pickup_Create` (0x18dda0) for a level-placed pickup (type 0xF0, `parsemap_create_dynamic_objects`):
    /// the placement's extra words give the type, id, switch channel, sound, respawn and lifetime. The pickup
    /// starts at rest (state 1, on the floor). The model comes from the placement's celglist (the placement's
    /// object), NOT from `weapon_data[id]+0x80` — the glist is passed as `param_4` to `Pickup_Create` [M 0x1d04f0].
    pub fn create_placed(translation: V3, rotation: [f32; 3], type_: u16, id: u16, rounds: i16, switch_channel: u16, sound: u16, respawn: u32, lifetime: u8, model: u32, bounds: Model, w: &WeaponData) -> Option<Pickup> {
        if id == 0xFFFF { return None; }
        if model == 0 { return None; }
        // Build the matrix directly from the placement translation — do NOT call `create` which lowers the
        // Y by `radius - |centre.y|` (that's for DROPPED pickups that fall to the floor; level-placed
        // pickups are already at the correct height). Lowering puts the sphere centre below the floor,
        // causing the LOS test in `touches` to fail [bug fix C-12].
        let m = nf_formats::placement::object_matrix(rotation, translation);
        let p = Pickup {
            kind: type_, weapon: id, rounds, model, bounds, m,
            start: translation, vel: [0.0; 3], state: 1, lifetime: 0,
            sound: if sound == 0xFFFF { None } else { Some(sound as u32) },
            hidden: false, gone: false, from: None,
        };
        let mut p = p;
        p.lifetime = if lifetime == 0 { 0 } else { lifetime as u16 * 60 };
        if type_ == kind::WEAPON && p.rounds == 0 {
            p.rounds = w.clip_size(id as usize);
        }
        Some(p)
    }

    /// `Pickup_Create` for a dropped pickup: upright with the matrix's heading (`Mat_Align2Up(M, +Y, Mat_GetDir(M))`,
    /// the direction taken as row 2 — *suggested*), lowered by radius − |centre y|, falling. *Deviations*: the
    /// fall ray starts at the given translation (`build_PointOnFloor`'s result is not read) and the sphere push-out
    /// (`Collide_SphereIntersect`) is not done.
    fn create(m: &Mat4, weapon: u16, rounds: i16, kind: u16, model: u32, bounds: Model) -> Pickup {
        let t = trans(m);
        let dir = [m[2][0], 0.0, m[2][2]];
        let l = (dir[0] * dir[0] + dir[2] * dir[2]).sqrt();
        let z = if l > 1e-6 { [dir[0] / l, 0.0, dir[2] / l] } else { [0.0, 0.0, 1.0] };
        // x = up × z, y = up.
        let x = [z[2], 0.0, -z[0]];
        let y = t[1] - (bounds.radius - bounds.centre[1].abs());
        let m = [[x[0], x[1], x[2], 0.0], [0.0, 1.0, 0.0, 0.0], [z[0], z[1], z[2], 0.0], [t[0], y, t[2], 1.0]];
        Pickup { kind, weapon, rounds, model, bounds, m, start: t, vel: [0.0; 3], state: 0, lifetime: DROP_LIFETIME, sound: Some(SOUND), hidden: false, gone: false, from: None }
    }

    /// The world sphere centre (`obj+0x80`, `Control_BuildWorldSph`): the glist centre through the matrix.
    pub fn centre(&self) -> V3 {
        let (c, m) = (self.bounds.centre, &self.m);
        [0, 1, 2].map(|k| c[0] * m[0][k] + c[1] * m[1][k] + c[2] * m[2][k] + m[3][k])
    }

    /// `Pickup_Update`'s fall (state 0): velocity += gravity · (1/60)², translation += velocity; the ray from the start
    /// to the translation lowered by |centre y| finds the floor; once the translation is below the hit, the pickup
    /// rests at hit + `HIT+0x30` · (−|centre y|) + info `+0x10` (translation − sphere centre, fixed at creation), so its
    /// sphere centre sits |centre y| above the hit. *Suggested*: `HIT+0x30` taken as the unit ray (U-036).
    pub fn fall(&mut self, world: &CollisionWorld) {
        if self.state != 0 { return; }
        self.vel[1] += GRAVITY;
        for k in 0..3 { self.m[3][k] += self.vel[k]; }
        let t = trans(&self.m);
        let to = [t[0], t[1] - self.bounds.centre[1].abs(), t[2]];
        if let Some(f) = world.segment(self.start, to) {
            let hit = [0, 1, 2].map(|k| self.start[k] + (to[k] - self.start[k]) * f);
            if t[1] - hit[1] < 0.0 {
                let ray = [0, 1, 2].map(|k| to[k] - self.start[k]);
                let l = (ray[0] * ray[0] + ray[1] * ray[1] + ray[2] * ray[2]).sqrt().max(1e-12);
                let (cy, c) = (self.bounds.centre[1].abs(), self.centre());
                for k in 0..3 { self.m[3][k] = hit[k] - ray[k] / l * cy + (t[k] - c[k]); }
                self.vel = [0.0; 3];
                self.state = 1;
            }
        }
    }

    /// `Pickup_Update`'s touch test for Bond: alive, the sphere centre within radius + 1.1 of his position (squared
    /// distances), and a clear line from his sphere centre (`obj+0x80`; here `body`, *suggested*) to the pickup's.
    pub fn touches(&self, world: &CollisionWorld, pos: V3, body: V3) -> bool {
        let c = self.centre();
        let r = self.bounds.radius + 1.1;
        let d2 = (c[0] - pos[0]).powi(2) + (c[1] - pos[1]).powi(2) + (c[2] - pos[2]).powi(2);
        d2 < r * r && world.line_of_sight(body, c)
    }
}

/// Bond's carried rounds and weapons (BLData = player `obj+0xE0`).
#[derive(Clone, Debug)]
pub struct Inventory {
    /// `+0x170 + 2·t`: rounds carried per ammo type.
    pub carried: Vec<i16>,
    /// `+0x1B8 + 0xC·w`: rounds in weapon w's clip; `+0x1BA`: owned.
    pub clip: Vec<i16>,
    pub owned: Vec<bool>,
}

impl Inventory {
    pub fn empty(weapons: usize, ammo_types: usize) -> Inventory {
        Inventory { carried: vec![0; ammo_types], clip: vec![0; weapons], owned: vec![false; weapons] }
    }

    /// Bond on 07000005 at the level start [L] (`Player_InitWeapon` is not read): weapons 1, 2 (P2K, 7 loaded),
    /// 0x4A, 0x4E, 0x50, 0x54; 41 rounds of type 1.
    pub fn level_start(weapons: usize, ammo_types: usize) -> Inventory {
        let mut inv = Inventory::empty(weapons, ammo_types);
        for (w, c, own) in [(1, 1, true), (2, 7, true), (0x33, 1, false), (0x4a, 100, true), (0x4c, 1, false), (0x4e, 500, true), (0x4f, 1, false), (0x50, 0, true), (0x54, 1, true)] {
            if w < weapons { inv.clip[w] = c; inv.owned[w] = own; }
        }
        if ammo_types > 1 { inv.carried[1] = 41; }
        inv
    }

    fn cap(w: &WeaponData, t: usize) -> i16 { w.ammo_cap.get(t).copied().unwrap_or(0) }

    /// The clip slot a weapon uses: its base's when `+0x05` is set and their ammo types agree.
    fn clip_slot(w: &WeaponData, id: usize) -> usize {
        let base = w.base(id) as usize;
        if w.shares_base_clip(id) && w.ammo_type(id) == w.ammo_type(base) { base } else { id }
    }

    /// `Player_EquipWeapon` (single player; `Upgrade_Weapon` taken as the identity — no upgrades bought). `loaded`:
    /// the first-person model `+0xDC` is in memory (`hashtable_getitem`). Not owned (by its base): owned, a full clip,
    /// and the rounds beyond it carried; owned: refused when full or no rounds, else carried. Carried ≤ the cap.
    pub fn equip_weapon(&mut self, w: &WeaponData, id: usize, rounds: i16, loaded: bool) -> bool {
        if w.first_person(id) == 0 || !loaded { return false; }
        let (base, t) = (w.base(id) as usize, w.ammo_type(id));
        if base >= self.owned.len() || t >= self.carried.len() { return false; }
        if !self.owned[base] {
            self.owned[base] = true;
            let slot = Self::clip_slot(w, id);
            let old = self.clip[slot];
            self.clip[slot] = w.clip_size(id);
            let extra = rounds.wrapping_add(old).wrapping_sub(w.clip_size(id));
            if extra > 0 { self.carried[t] = self.carried[t].wrapping_add(extra); }
        } else {
            if self.carried[t] >= Self::cap(w, t) || rounds == 0 { return false; }
            self.carried[t] = self.carried[t].wrapping_add(rounds);
        }
        self.carried[t] = self.carried[t].min(Self::cap(w, t));
        true
    }

    /// `Player_EquipAmmo`: below the cap and with rounds, they are carried (flag `+0x68 & 0x10` weapons fill an empty
    /// clip first), clamped; returns carried − (old + clip filled), 0 = refused (as the game computes it).
    pub fn equip_ammo(&mut self, w: &WeaponData, id: usize, rounds: i16) -> i16 {
        let t = w.ammo_type(id);
        if t >= self.carried.len() || self.carried[t] >= Self::cap(w, t) || rounds == 0 { return 0; }
        let (mut rounds, mut filled) = (rounds, 0i16);
        if w.flags(id) & 0x10 != 0 {
            let slot = Self::clip_slot(w, id);
            if slot < self.clip.len() && self.clip[slot] == 0 {
                filled = rounds.min(w.clip_size(id));
                self.clip[slot] = filled;
                rounds = rounds.wrapping_sub(w.clip_size(id));
                if rounds < 1 { return filled; }
            }
        }
        let old = self.carried[t];
        self.carried[t] = old.wrapping_add(rounds).min(Self::cap(w, t));
        self.carried[t].wrapping_sub(filled.wrapping_add(old))
    }

    /// `Pickup_Handler` for Bond, types 0 / 1: a weapon he lacks (`+0x1BA` of the id) is equipped, otherwise its rounds
    /// are taken as ammo. `None`: refused, the pickup stays.
    pub fn take(&mut self, w: &WeaponData, p: &Pickup, loaded: bool) -> Option<Taken> {
        let id = p.weapon as usize;
        match p.kind {
            kind::WEAPON | kind::BOND_BONUS if !self.owned.get(id).copied().unwrap_or(true) => {
                // Type 0 (weapon) or type 6 (Bond bonus — `Pickup_Create` requires the gold medal, but the
                // port doesn't track it; treating type 6 as a weapon pickup is a STANDIN for the gold-medal
                // gate). `Upgrade_Weapon(id)` is the identity in the port (no upgrades bought) [M 0x18e650].
                self.equip_weapon(w, id, p.rounds, loaded).then_some(Taken::Weapon { weapon: p.weapon })
            }
            kind::WEAPON | kind::BOND_BONUS | kind::AMMO => {
                let n = self.equip_ammo(w, id, p.rounds);
                (n != 0).then_some(Taken::Ammo { weapon: p.weapon, rounds: n })
            }
            kind::ARMOUR => {
                // Type 3: armour 50 (`BLData+0x8B0`, only when below 50) [M pickups.md:71]. STANDIN: no
                // armour field in the port's Health struct; the pickup is consumed but has no effect.
                Some(Taken::Weapon { weapon: p.weapon })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A weapon table with the 07000005 records that matter [L]: P2K (2: type 1, clip 7), SG5 (0x16: type 5, clip 30).
    fn table() -> WeaponData {
        let mut records = vec![[0u8; nf_formats::weapon::RECORD]; 0x73];
        for (w, base, t, clip, model, fp) in [(2usize, 2u16, 1u8, 7u16, 0x0200_08b3u32, 0x0500_0088u32), (0x16, 0x16, 5, 30, 0x0200_0b44, 0x0500_00c0)] {
            let r = &mut records[w];
            r[0..2].copy_from_slice(&(w as u16).to_le_bytes());
            r[2..4].copy_from_slice(&base.to_le_bytes());
            r[5] = 1;
            r[0x80..0x84].copy_from_slice(&model.to_le_bytes());
            r[0x90] = t;
            r[0x92..0x94].copy_from_slice(&clip.to_le_bytes());
            r[0xdc..0xe0].copy_from_slice(&fp.to_le_bytes());
        }
        let mut ammo_cap = vec![0i16; 34];
        ammo_cap[1] = 70;
        ammo_cap[5] = 90;
        WeaponData { records, ammo_cap }
    }

    #[test]
    fn a_new_gun_comes_with_a_full_clip_and_then_counts_as_ammo() {
        let w = table();
        let mut inv = Inventory::level_start(0x73, 34);
        assert!(inv.equip_weapon(&w, 0x16, 12, true));
        assert_eq!((inv.owned[0x16], inv.clip[0x16], inv.carried[5]), (true, 30, 0), "12 rounds still fill the clip");
        assert!(!inv.equip_weapon(&w, 0x16, 0, true), "owned and no rounds: refused");
        assert_eq!(inv.equip_ammo(&w, 0x16, 20), 20);
        assert_eq!(inv.equip_ammo(&w, 0x16, 80), 70, "capped at 90");
        assert_eq!(inv.equip_ammo(&w, 0x16, 5), 0, "full: refused");
        assert!(!Inventory::level_start(0x73, 34).equip_weapon(&w, 0x16, 12, false), "first-person model not loaded");
        // Sister: the P2K is owned at the start, so its drop is ammo.
        let p = Pickup::create(&[[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0; 4]], 2, 5, kind::WEAPON, 0, Model { centre: [0.0; 3], radius: 0.5 });
        assert_eq!(inv.take(&w, &p, true), Some(Taken::Ammo { weapon: 2, rounds: 5 }));
        assert_eq!(inv.carried[1], 46);
    }

    #[test]
    fn a_dropped_gun_falls_to_the_floor_and_rests() {
        let floor = CollisionWorld::from_triangles(vec![
            [[-5.0, 0.0, -5.0], [5.0, 0.0, -5.0], [5.0, 0.0, 5.0]], [[-5.0, 0.0, -5.0], [5.0, 0.0, 5.0], [-5.0, 0.0, 5.0]],
        ]);
        let m = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.5, 1.2, 0.5, 1.0]];
        let mut p = Pickup::create(&m, 0x16, 12, kind::WEAPON, 0, Model { centre: [0.0, 0.05, 0.3], radius: 0.4 });
        assert!((p.m[3][1] - (1.2 - 0.35)).abs() < 1e-6, "lowered by radius − |centre y|");
        let mut ticks = 0;
        while p.state == 0 && ticks < 600 { p.fall(&floor); ticks += 1; }
        assert_eq!(p.state, 1);
        assert!((p.centre()[1] - 0.05).abs() < 1e-5, "the sphere centre rests |centre y| above the floor: {:?}", p.centre());
        // 0.85 units at 9.8 / 3600 per tick²: ⌈√(2 · 0.85 · 3600 / 9.8)⌉ ≈ 25 ticks.
        assert!((24..=27).contains(&ticks), "{ticks}");
        assert!(p.touches(&floor, [0.5, 0.0, 1.5], [0.5, 1.0, 1.5]));
        assert!(!p.touches(&floor, [0.5, 0.0, 3.0], [0.5, 1.0, 3.0]), "1.1 + radius away");
    }
}
