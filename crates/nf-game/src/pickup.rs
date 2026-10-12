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
    /// The label/item pickup (`Pickup_Handler` case 4 [M 0x18e650]: binds `PICKUPINFO+0x34`'s label
    /// text when non-zero, always consumed). 07000008's `oBriefcase_closed` (the guidance chip,
    /// switch ch 0x4A) is one — label 0, so no message [D #290].
    pub const LABEL: u16 = 4;
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
    /// `+0x28`: the switch channel set to 1 when taken (`Pickup_Handler` 0x18e650:330-333). 0 = none.
    pub switch_channel: u16,
    /// `+0x2A`: the sound when taken.
    pub sound: Option<u32>,
    /// `obj+0xF0 & 0x10` (U-036).
    pub hidden: bool,
    /// Deleted (`obj+0xFE |= 1`).
    pub gone: bool,
    /// The drone that dropped it (index into `Drones::list`), for drawing.
    pub from: Option<usize>,
    /// The object the renderer draws (`obj+0xD8` = the celglist handed to `Pickup_Create`, [M]
    /// 0x1d04f0:459-462 passes the placement's object; `Pickup_CreateSimple` 0x18f348:45 the
    /// `weapon_data[id]+0x80` glist): (map, object) into the level archive for placed pickups,
    /// `None` for drops (the renderer falls back to the `model` hashcode). The game draws the glist
    /// pointer, not a hashcode re-lookup — an object with hashcode 0xFFFFFFFF (07000005's
    /// `oOBJECT_AmmoPickUp_9mm` [D]) has no by-hash entry and would never be drawn.
    pub obj: Option<(usize, usize)>,
}

/// What Bond took.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Taken {
    Weapon { weapon: u16 },
    Ammo { weapon: u16, rounds: i16 },
    /// Kind 2 (`Player_EquipKey` 0x1a97a0: `BLData+0x70C+id = 1`; message 0x2000010) [M].
    Key { id: u16 },
    /// Kind 3: `BLData+0x8B0 = 50.0`, `+0x8BC = 1.0`; message 0x200000F, sound 0x141 [M 0x18e650:220-225].
    Armour,
    /// Kind 6 (Bond bonus): `PlrStat_LogBondBonus()` only in the take-effect switch [M 0x18e650:234-236];
    /// the shared tail still runs — message 0x200004A, and `+0x2A` is overwritten with sound 0x14C at take
    /// time (`Pickup_Handler` :319-321 writes the per-kind sounds 0xF5/0x141/0x14C before `Sound_Play3D`).
    /// fix-objects: the port consumed the bonus silently — no sound, no log (the 0x200004A HUD message
    /// has no port path yet).
    Bonus,
    /// Kind 4, the label/item pickup (the guidance-chip briefcase): consumed, no per-kind sound
    /// override [M 0x18e650 case 4 — `lVar20 = Txt_BindLabel(+0x34)` only].
    Label,
}

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
        // (drops carry no switch channel: `Pickup_CreateSimple` passes 0 [M 0x18f348])
    }

    /// `Pickup_Create` (0x18dda0) for a level-placed pickup (type 0xF0, `parsemap_create_dynamic_objects`):
    /// the placement's extra words give the type, id, switch channel, sound, respawn and lifetime. The pickup
    /// starts at rest (state 1, on the floor). The model comes from the placement's celglist (the placement's
    /// object), NOT from `weapon_data[id]+0x80` — the glist is passed as `param_4` to `Pickup_Create` [M 0x1d04f0].
    pub fn create_placed(translation: V3, rotation: [f32; 3], type_: u16, id: u16, rounds: i16, switch_channel: u16, sound: u16, _respawn: u32, lifetime: u8, model: u32, bounds: Model, w: &WeaponData) -> Option<Pickup> {
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
            switch_channel,
            sound: if sound == 0xFFFF { None } else { Some(sound as u32) },
            hidden: false, gone: false, from: None, obj: None,
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
        Pickup { kind, weapon, rounds, model, bounds, m, start: t, vel: [0.0; 3], state: 0, lifetime: DROP_LIFETIME, switch_channel: 0, sound: Some(SOUND), hidden: false, gone: false, from: None, obj: None }
    }

    /// The world sphere centre (`obj+0x80`, `Control_BuildWorldSph`): the glist centre through the matrix.
    pub fn centre(&self) -> V3 {
        let (c, m) = (self.bounds.centre, &self.m);
        [0, 1, 2].map(|k| c[0] * m[0][k] + c[1] * m[1][k] + c[2] * m[2][k] + m[3][k])
    }

    /// `Pickup_Update`'s fall (state 0): velocity += gravity · (1/60)², translation += velocity; the ray runs
    /// from the PREVIOUS tick's translation (obj+0x40: the movement pass copies last tick's `obj+0x30` there,
    /// which for pickups — flags2 0x44 — is the previous `obj+0xC0` translation [M 0x18e148:110-114 +
    /// 0x1336f8:212-231, fix-player2]) to the translation lowered by |centre y| finds the floor; once the
    /// translation is below the hit, the pickup rests at hit + `HIT+0x30` · (−|centre y|) + info `+0x10`
    /// (translation − sphere centre, fixed at creation), so its sphere centre sits |centre y| above the hit.
    /// *Suggested*: `HIT+0x30` taken as the unit ray (U-036).
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
        // The next tick's sweep starts here (the game's `obj+0x40` follows the translation each tick).
        self.start = trans(&self.m);
    }

    /// `Pickup_Update`'s touch test for Bond: alive, the sphere centre within radius + 1.1 of his position (squared
    /// distances), and a clear line from his world-sphere centre (`obj+0x80`) to the pickup's [M 0x18e148:203-214
    /// + L: the distance test uses the player's position (+0x30), the LOS test the player's world-sphere centre
    /// (+0x80) — two different points]. `pos` = the player position, `body` = the player's sphere centre
    /// (`Bond::sphere_centre`).
    pub fn touches(&self, world: &CollisionWorld, pos: V3, body: V3) -> bool {
        let c = self.centre();
        let r = self.bounds.radius + 1.1;
        let d2 = (c[0] - pos[0]).powi(2) + (c[1] - pos[1]).powi(2) + (c[2] - pos[2]).powi(2);
        // The LOS ray from the player's world-sphere centre to the pickup's centre. A resting pickup whose
        // model centre y ≈ 0 (live: the 07000001 guard drop of weapon 0xe rests with its centre ON the
        // floor [L, fix_player2_live take-test log]) ends the ray ON the floor triangle; the port's
        // `segment` counts t = 1.0 as a hit, which refused every such drop forever (the "some drops never
        // pick up" playtest symptom). The game takes floor-resting drops, so its `Collide_LineOfSight`
        // (0x1edf50 → `Collide_RayIntersect` 0x1ec690) does not fail on the endpoint contact — the port
        // treats a hit at the ray's end (t ≈ 1) as clear. *Unverified*: RayIntersect's exact t range (the
        // wrapper decompile is one call; WORKLOG fix-player2 records it).
        let los = world.segment(body, c).map_or(true, |t| t >= 1.0 - 1e-4);
        // `NF_PICKUPLOG=1`: every take test — Bond's position, the pickup centre, the distance against
        // radius + 1.1, the LOS ray, and the state (fix-player2's drop diagnostics).
        if std::env::var_os("NF_PICKUPLOG").is_some() {
            eprintln!("[take-test] weapon {:#x} kind {} state {}: bond ({:.3}, {:.3}, {:.3}) centre ({:.3}, {:.3}, {:.3}) dist {:.3} vs r {:.3} los {}",
                self.weapon, self.kind, self.state, pos[0], pos[1], pos[2], c[0], c[1], c[2], d2.sqrt(), r, los);
        }
        d2 < r * r && los
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
    /// `+0x70C + id`: key flags (`Player_EquipKey` 0x1a97a0 writes 1; never read back in the code
    /// studied — the switch channels carry the effect) [M]. Indexed by the pickup's key id.
    pub keys: Vec<bool>,
}

impl Inventory {
    pub fn empty(weapons: usize, ammo_types: usize) -> Inventory {
        Inventory { carried: vec![0; ammo_types], clip: vec![0; weapons], owned: vec![false; weapons], keys: vec![false; 0x100] }
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

    /// `Upgrade_Weapon` (0x1a8f30): the pickup id remap every human-path pickup goes through
    /// (`Pickup_Handler` 0x18e650:142-144). 6 → `UpgradedHandguns[Upgrades]`, 0x1E/0x24/0x43/0x4A/0x4E/0x56
    /// via their tables, **10 → 0xC when `GameState[0x52]` is non-zero** (the game clock — always, in play)
    /// [M]. 0x54 mutates the Laser Watch records' zoom (8.0 / 16.0) instead of remapping [M 0x1a8f30:43-51].
    /// The tables' slot 0 holds the base id and `Upgrades` = 0 without the upgrade shop [L ee_start_05.bin:
    /// UpgradedHandguns 0x2bee08 = [2,4,6,8], Snipers 0x2bee28 = [0x1e,0x20,0x22,0x22], SilencedSnipers
    /// 0x2bee38 = [0x24,0x26,0x28,0x28], DartGuns 0x2bee68 = [0x43,0x44,0x44,0x44], Tasers 0x2bee48 =
    /// [0x4a,0x4c,0x4c,0x4c], Lasers 0x2bee18 = [0x4e,0x4f,0x4f,0x4f], PDAs 0x2bee58 = [0x56,0x57,0x57,0x57],
    /// Upgrades 0x2beea8 = 0].
    pub fn upgrade_weapon(id: u16, upgrades: usize, state_clock: u32) -> u16 {
        // Table slot `upgrades`; only slot 0 is live without the upgrade shop.
        const HANDGUNS: [u16; 4] = [2, 4, 6, 8];
        const SNIPERS: [u16; 4] = [0x1e, 0x20, 0x22, 0x22];
        const SILENCED_SNIPERS: [u16; 4] = [0x24, 0x26, 0x28, 0x28];
        const DART_GUNS: [u16; 4] = [0x43, 0x44, 0x44, 0x44];
        const TASERS: [u16; 4] = [0x4a, 0x4c, 0x4c, 0x4c];
        const LASERS: [u16; 4] = [0x4e, 0x4f, 0x4f, 0x4f];
        const PDAS: [u16; 4] = [0x56, 0x57, 0x57, 0x57];
        let t = |tab: &[u16; 4]| tab[upgrades.min(3)];
        match id {
            6 => t(&HANDGUNS),
            10 if state_clock != 0 => 0xC,
            0x1e => t(&SNIPERS),
            0x24 => t(&SILENCED_SNIPERS),
            0x43 => t(&DART_GUNS),
            0x4a => t(&TASERS),
            0x4e => t(&LASERS),
            0x56 => t(&PDAS),
            // 0x54 (Laser Watch): writes weapon_data[0x54/0x55]+0x88 (zoom) = 8.0 / 16.0 [M 0x1a8f30:43-51];
            // the id itself is unchanged, and the port has no shop flag (`DAT_002beeaa`) — 8.0 is the
            // un-upgraded value [L ee_start_05.bin record 0x54 +0x88]. Not modelled: the port has no
            // mutable weapon_data at take time.
            _ => id,
        }
    }

    /// `Pickup_Handler` (0x18e650) for Bond. `armour` = `BLData+0x8B0` (the taker's armour, written on the
    /// armour case); `clock` = `GameState+0x52` (the game clock, for the 10 → 0xC remap). Types 0/1: a weapon
    /// he lacks is equipped, otherwise its rounds are taken as ammo. Type 2 (key): `Player_EquipKey`, never
    /// refuses [M :217-219]. Type 3 (armour): refused while the armour is ≥ 50.0, else hard-set 50.0 [M
    /// :220-225]. Type 6 (Bond bonus): the game just logs a stat and shows a message — no weapon or ammo
    /// [M :234-236]. D-004: the gold-medal gate is skipped. The remapped id drives the owned-check and the
    /// equip [M 0x18e650:142-144]; the message builds the weapon name from the RAW `+0x24` [M :246], so the
    /// returned `Taken` carries `p.weapon` for display.
    pub fn take(&mut self, w: &WeaponData, p: &Pickup, loaded: bool, armour: &mut f32, clock: u32) -> Option<Taken> {
        // The remapped id decides the owned-check and the equip; the display id stays raw [M 0x18e650:142-144, :246].
        let id = Self::upgrade_weapon(p.weapon, 0, clock) as usize;
        match p.kind {
            kind::WEAPON if !self.owned.get(id).copied().unwrap_or(true) => {
                self.equip_weapon(w, id, p.rounds, loaded).then_some(Taken::Weapon { weapon: p.weapon })
            }
            kind::WEAPON | kind::AMMO => {
                let n = self.equip_ammo(w, id, p.rounds);
                (n != 0).then_some(Taken::Ammo { weapon: p.weapon, rounds: n })
            }
            kind::KEY => {
                // `Player_EquipKey` (0x1a97a0): `BLData+0x70C+id = 1`; the key never refuses, so the pickup
                // is always consumed [M 0x18e650:217-219]. `id` is the key index (`+0x24`).
                if let Some(k) = self.keys.get_mut(p.weapon as usize) { *k = true; }
                Some(Taken::Key { id: p.weapon })
            }
            kind::LABEL => {
                // Case 4 [M 0x18e650]: `Txt_BindLabel(+0x34)` when the label word is non-zero (the
                // port has no HUD text path; the briefcase's label is 0), then always consumed.
                Some(Taken::Label)
            }
            kind::ARMOUR => {
                // Type 3: refused while `BLData+0x8B0 >= 50.0` (the pickup stays); else `+0x8B0 = 50.0`,
                // `+0x8BC = 1.0` — hardcoded 50, `+0x26` is not read [M 0x18e650:220-225]. (+0x8BC, the
                // HUD flash fraction, is not modelled.)
                if *armour >= 50.0 { return None; }
                *armour = 50.0;
                Some(Taken::Armour)
            }
            kind::BOND_BONUS => {
                // Type 6: the game's `Pickup_Handler` case 6 calls `PlrStat_LogBondBonus` and shows a
                // message — no weapon or ammo. D-004: the gold-medal gate (`Pickup_Create` line 93) is
                // skipped. The pickup is consumed [M 0x18e650:234-236]. fix-objects: reported as
                // `Bonus` so the caller plays the take-time sound 0x14C instead of staying silent.
                Some(Taken::Bonus)
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
        WeaponData { records, ammo_cap, ammo_raw: Vec::new() }
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
        let mut armour = 0.0;
        assert_eq!(inv.take(&w, &p, true, &mut armour, 1), Some(Taken::Ammo { weapon: 2, rounds: 5 }));
        assert_eq!(inv.carried[1], 46);
        assert_eq!(armour, 0.0, "a weapon take does not touch the armour");
    }

    /// `Upgrade_Weapon` (0x1a8f30) with the live tables [L ee_start_05.bin 0x2bee08..]: id 6 → 2 with no
    /// upgrades; 10 → 0xC once the game clock runs (GameState+0x52 ≠ 0); 0x24 → 0x24 (slot 0); the clock
    /// gate: 10 stays 10 only while the clock is 0 [M 0x1a8f30:18-22].
    #[test]
    fn pickup_ids_go_through_upgrade_weapon() {
        assert_eq!(Inventory::upgrade_weapon(6, 0, 100), 2, "UpgradedHandguns[0] = 2 [L]");
        assert_eq!(Inventory::upgrade_weapon(0x24, 0, 100), 0x24, "UpgradedSilencedSnipers[0] = 0x24 [L]");
        assert_eq!(Inventory::upgrade_weapon(0x1e, 0, 100), 0x1e, "UpgradedSnipers[0] = 0x1e [L]");
        assert_eq!(Inventory::upgrade_weapon(10, 0, 0), 10, "clock 0: 10 is not remapped [M]");
        assert_eq!(Inventory::upgrade_weapon(10, 0, 1), 0xC, "clock running: 10 → 0xC [M]");
        assert_eq!(Inventory::upgrade_weapon(0x35, 0, 100), 0x35, "other ids unchanged");
        // Sister: an upgrade level shifts the table (the shop path).
        assert_eq!(Inventory::upgrade_weapon(6, 2, 100), 6, "UpgradedHandguns[2] = 6 [L]");
    }

    /// A kind-2 pickup is always taken: `Player_EquipKey` (0x1a97a0) sets `BLData+0x70C+id = 1` and
    /// `Pickup_Handler` case 2 never refuses [M 0x18e650:217-219].
    #[test]
    fn a_key_is_always_taken_and_sets_its_flag() {
        let w = table();
        let mut inv = Inventory::level_start(0x73, 34);
        let p = Pickup::create(&[[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0; 4]], 7, 0, kind::KEY, 0, Model { centre: [0.0; 3], radius: 0.5 });
        let mut armour = 0.0;
        assert_eq!(inv.take(&w, &p, false, &mut armour, 1), Some(Taken::Key { id: 7 }), "never refused");
        assert!(inv.keys[7], "BLData+0x70C+7 = 1");
        assert!(!inv.keys[6], "sister: other keys untouched");
        // A second take of the same key still succeeds (no owned-check on keys).
        assert_eq!(inv.take(&w, &p, false, &mut armour, 1), Some(Taken::Key { id: 7 }));
    }

    /// Kind 3: refused while the armour is ≥ 50.0 (the pickup stays), else `BLData+0x8B0 = 50.0` hardcoded
    /// — the pickup's rounds are not read [M 0x18e650:220-225].
    #[test]
    fn armour_fills_to_50_and_refuses_when_full() {
        let w = table();
        let mut inv = Inventory::level_start(0x73, 34);
        let p = Pickup::create(&[[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0; 4]], 6, 40, kind::ARMOUR, 0, Model { centre: [0.0; 3], radius: 0.5 });
        let mut armour = 30.0;
        assert_eq!(inv.take(&w, &p, true, &mut armour, 1), Some(Taken::Armour));
        assert_eq!(armour, 50.0, "hard-set 50.0, not +rounds");
        // Full: refused → the pickup stays (None).
        assert_eq!(inv.take(&w, &p, true, &mut armour, 1), None, "≥ 50 refuses");
        assert_eq!(armour, 50.0);
    }

    /// The take ray's ORIGIN is the caller's third argument — `Pickup_Update` passes the player's
    /// world-sphere centre (`obj+0x80`), not his feet position [M 0x18e148:212-214]. A low shelf between
    /// the two: a ray from the sphere centre clears it, a ray from the feet does not — the origin decides
    /// the take, which is the port's second-Raptor divergence (port-check §1).
    #[test]
    fn the_take_ray_origin_decides_the_take() {
        let q = |a: V3, b: V3, c: V3, d: V3| [[a, b, c], [a, c, d]];
        let mut t = Vec::new();
        t.extend(q([-9.0, 0.0, -9.0], [9.0, 0.0, -9.0], [9.0, 0.0, 9.0], [-9.0, 0.0, 9.0]));
        // A 0.5-high wall at z = 0 between the player (z = −0.6) and the pickup (z = +0.7).
        t.extend(q([-9.0, 0.0, 0.0], [-9.0, 0.5, 0.0], [9.0, 0.5, 0.0], [9.0, 0.0, 0.0]));
        let w = CollisionWorld::from_triangles(t);
        // centre y 0.05: the pickup rests with its sphere centre 0.05 above the floor (as the live pickups
        // do) — a centre exactly ON the floor plane would end the take ray on the floor triangle.
        let mut p = Pickup::create(&[[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 1.0, 0.7, 1.0]], 2, 5, kind::WEAPON, 0, Model { centre: [0.0, 0.05, 0.0], radius: 0.5 });
        let mut ticks = 0;
        while p.state == 0 && ticks < 600 { p.fall(&w); ticks += 1; }
        assert_eq!(p.state, 1);
        // The player's position is within radius + 1.1; only the LOS origin differs. The ray to the resting
        // centre (0, 0.05, 0.7) crosses the wall plane at 46% of its length: an origin at y 0.6 passes under
        // the wall top (y 0.35 there), one at y 1.2 clears it (y 0.67).
        let pos = [0.0, 0.0, -0.6];
        let c = p.centre();
        assert!((c[2] - 0.7).abs() < 1e-4 && (c[1] - 0.05).abs() < 1e-4, "resting |centre y| above the floor: {:?}", c);
        assert!(p.touches(&w, pos, [0.0, 1.2, -0.6]), "ray from above the wall top: taken");
        assert!(!p.touches(&w, pos, [0.0, 0.6, -0.6]), "sister: ray from below the wall top is blocked");
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
