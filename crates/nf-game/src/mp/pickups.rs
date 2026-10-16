//! The MP weapon sets and pickups (`mp-pickups-respawn`): the `PickupMatrix` weapon-set table and
//! `Pickup_CreateFromSet` 0x18f1b0, the random-set re-roll `Pickup_MakeRandomWeaponSet` 0x18f098
//! (called by `MP_Init` 0x183438:11 on every non-Ravine match init), the spawn loadout
//! `MP_EquipPlayer` 0x1855f8 with `Upgrade_MPWeapon` 0x1a9040, the registered-pickup layer
//! (`MP_RegisterPickup` 0x189d80, `MP_Pickup_Process` 0x18a168, the per-slot visit stamps
//! `BOTSTATE_setPickupVisitTime` 0x129958) and the enviro toggles (the gun emplacements
//! `GunImp_Create` 0x131f20 and the vehicles `Car_Create` 0x1305b0).
//!
//! **Table layout** (this corrects `pickup::PICKUP_MATRIX_ROW0`'s reading, see the report):
//! `PickupMatrix` @0x2b9100 is 110 bytes in the symtab and `UseableGuns` @0x2b9170 sits at +0x70,
//! so the 11 weapon sets are 11 rows × 10 bytes (5 u16 each); `Pickup_CreateFromSet` reads
//! `*(u16*)(PickupMatrix + set*10 + (kind-7)*2)` (byte offsets — the decompiler's prints are
//! byte-based, proven by `Pickup_MakeRandomWeaponSet`, which writes row 10 at byte offsets
//! 100..108) and the menu offers exactly 11 sets. A 20-byte-row reading would need 220 bytes and
//! would overlap `UseableGuns`.
//!
//! All numbers here are [M] = read in this task from `work/decomp/ACTION.ELF/<addr>_<name>.c`, or
//! [D ELF] = byte dumps from `work/elf/ACTION.ELF` in this task (file offset = vaddr − 0x106100).

use crate::drone::Rng;

/// `PickupMatrix` @0x2b9100 — 11 weapon sets × 5 weapon ids, the weapons the level's kind 7..11
/// weapon placements become (`Pickup_Create` 0x18dda0:104-105 intercepts kinds 7-11 into
/// `Pickup_CreateFromSet`) [D ELF 0x2b9100, 110 bytes]. Row 10 (the Random set) ships zeroed and
/// is re-rolled by `Pickup_MakeRandomWeaponSet` at every `MP_Init` [M 0x183438:11].
///
/// Menu order (the 11 sets of `P_MPPLAYERMODS` 0x2104d0:26-49, labels in `mp-menus.md` §2):
/// 0 Normal, 1 Pistols, 2 Automatic, 3 Sniping, 4 Explosives 1, 5 Explosives 2, 6 MI6 Operative,
/// 7 Phoenix Weapons, 8 State of the Art, 9 Cloak and Dagger, 10 Random.
pub const PICKUP_MATRIX: [[u16; 5]; 11] = [
    [0x6, 0x1c, 0x1e, 0x18, 0x2c],
    [0x2, 0x6, 0xa, 0x10, 0x42],
    [0x12, 0x15, 0x18, 0x1c, 0x52],
    [0xe, 0x1e, 0x11, 0x24, 0x32],
    [0x2a, 0x2c, 0x3a, 0x2e, 0x1a],
    [0x35, 0x34, 0x3a, 0x37, 0x3b],
    [0x2, 0x11, 0x37, 0x24, 0x42],
    [0xa, 0x1e, 0x52, 0x1a, 0x32],
    [0x1a, 0x3b, 0x32, 0x3a, 0x2c],
    [0x15, 0x16, 0x37, 0x11, 0x24],
    [0x0, 0x0, 0x0, 0x0, 0x0],
];

/// `UseableGuns` @0x2b9170 — the 27 guns the random set samples, u16 ids [D ELF 0x2b9170, 54
/// bytes]. `Pickup_MakeRandomWeaponSet` samples `Rand(index)` per kind: kind 7 (slot 0) draws
/// `Rand(0x1a)` = guns[0..26], kinds 8-11 draw `Rand(0x1b)` = guns[0..27] [M 0x18f098:33-37], and
/// a draw that duplicates an earlier slot of the row is re-drawn [M :40-50].
pub const USEABLE_GUNS: [u16; 27] = [
    0x2, 0x6, 0xa, 0xc, 0xe, 0x10, 0x12, 0x14, 0x16, 0x19, 0x1a, 0x1c, 0x1e, 0x24, 0x2a, 0x2c,
    0x2e, 0x33, 0x37, 0x35, 0x36, 0x34, 0x3a, 0x3b, 0x42, 0x11, 0x52,
];

/// `Pickup_MakeRandomWeaponSet` 0x18f098: zero the row, then for each of the 5 kind slots draw a
/// useable gun (`Rand(0x1a)` for slot 0, `Rand(0x1b)` for the rest) re-drawing duplicates [M
/// 0x18f098:16-50]. Returns the new row 10 (the caller stores it into the live matrix).
pub fn make_random_weapon_set(rng: &mut Rng) -> [u16; 5] {
    let mut row = [0u16; 5];
    for slot in 0..5 {
        let range = if slot == 0 { 0x1a } else { 0x1b };
        loop {
            let gun = USEABLE_GUNS[rng.rand(range) as usize];
            if !row[..slot].contains(&gun) {
                row[slot] = gun;
                break;
            }
        }
    }
    row
}

/// The weapon id a kind 7..11 placement becomes for `set` (`Pickup_CreateFromSet` 0x18f1b0:56:
/// `PickupMatrix[set][(kind-7)]`; `set` is the u16 at `MPSettings+0x1B4` [M]). `None` when the
/// kind is out of 7..=11 or the set is out of 0..=10.
pub fn weapon_for_kind(set: u32, kind: u32, matrix: &[[u16; 5]; 11]) -> Option<u16> {
    if !(7..=11).contains(&kind) {
        return None;
    }
    matrix.get(set as usize).map(|row| row[(kind - 7) as usize])
}

/// The per-character upgrade tables of `Upgrade_MPWeapon` 0x1a9040 [D ELF]:
/// - `Upgrades` @0x2beea8 — the per-index 9-byte-stride flags: +0 the PPK/P99 upgrade pick, +1 the
///   grapple pick, +3 the sniper/silenced-sniper pick. The symtab declares 36 bytes (indices 0-3
///   clean); the game reads `index*9` whatever the index is, so the port carries the game's
///   on-disc bytes for indices 0..28 and treats table picks ≥ 4 (which read past the 4-entry
///   tables) as "no upgrade" — *Deviation*, the game reads adjacent .data there (see the report's
///   live checks).
/// - `UpgradedPPK_MP` @0x2bee78 `[0x2, 0x4, 0x4, 0x4]`, `UpgradedP99_MP` @0x2bee88
///   `[0x6, 0x6, 0x6, 0x8]`, `UpgradedGrapple_MP` @0x2bee98 `[0x50, 0x51, 0x51, 0x51]`,
///   `UpgradedSnipers` @0x2bee28 `[0x1e, 0x20, 0x22, 0x22]`, `UpgradedSilencedSnipers` @0x2bee38
///   `[0x24, 0x26, 0x28, 0x28]`.
pub const UPGRADES: [[u8; 9]; 29] = build_upgrades();
pub const UPGRADED_PPK_MP: [u16; 4] = [0x2, 0x4, 0x4, 0x4];
pub const UPGRADED_P99_MP: [u16; 4] = [0x6, 0x6, 0x6, 0x8];
pub const UPGRADED_GRAPPLE_MP: [u16; 4] = [0x50, 0x51, 0x51, 0x51];
pub const UPGRADED_SNIPERS: [u16; 4] = [0x1e, 0x20, 0x22, 0x22];
pub const UPGRADED_SILENCED_SNIPERS: [u16; 4] = [0x24, 0x26, 0x28, 0x28];

const fn build_upgrades() -> [[u8; 9]; 29] {
    // The on-disc bytes @0x2beea8, 261 bytes (29 x 9) [D ELF 0x2beea8].
    let raw: [u8; 261] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x13, 0x14, 0x15, 0x1f,
        0x20, 0x23, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x1d, 0x00, 0x00,
        0x02, 0xa1, 0x02, 0x00, 0x00, 0x30, 0x00, 0x00, 0x02, 0xa0, 0x02, 0x00, 0x00, 0x1c, 0x00,
        0x00, 0x02, 0x98, 0x02, 0x00, 0x00, 0x1b, 0x00, 0x00, 0x02, 0x97, 0x02, 0x00, 0x00, 0x1a,
        0x00, 0x00, 0x02, 0x96, 0x02, 0x00, 0x00, 0x19, 0x00, 0x00, 0x02, 0x95, 0x02, 0x00, 0x00,
        0x15, 0x00, 0x00, 0x02, 0x9b, 0x02, 0x00, 0x00, 0x1e, 0x00, 0x00, 0x02, 0xa1, 0x02, 0x00,
        0x00, 0x31, 0x00, 0x00, 0x02, 0xa0, 0x02, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x02, 0x98, 0x02,
        0x00, 0x00, 0x1b, 0x00, 0x00, 0x02, 0x97, 0x02, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x02, 0x96,
        0x02, 0x00, 0x00, 0x19, 0x00, 0x00, 0x02, 0x95, 0x02, 0x00, 0x00, 0x16, 0x00, 0x00, 0x02,
        0x9b, 0x02, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x02, 0xa4, 0x02, 0x00, 0x00, 0x32, 0x00, 0x00,
        0x02, 0xa3, 0x02, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x02, 0x98, 0x02, 0x00, 0x00, 0x1b, 0x00,
        0x00, 0x02, 0x97, 0x02, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x02, 0x96, 0x02, 0x00, 0x00, 0x19,
        0x00, 0x00, 0x02, 0x95, 0x02, 0x00, 0x00, 0x17, 0x00, 0x00, 0x02, 0xa2, 0x02, 0x00, 0x00,
        0x20, 0x00, 0x00, 0x02, 0x31, 0x04, 0x00, 0x00, 0x33, 0x00, 0x00, 0x02, 0x3d, 0x04, 0x00,
        0x00, 0x1c, 0x00, 0x00, 0x02, 0x98,
    ];
    let mut out = [[0u8; 9]; 29];
    let mut i = 0;
    while i < 29 {
        let mut j = 0;
        while j < 9 {
            out[i][j] = raw[i * 9 + j];
            j += 1;
        }
        i += 1;
    }
    out
}

/// `Upgrade_MPWeapon` 0x1a9040: `index` = the caller's `BLData+0x94e` byte (the MP slot —
/// `MP_ReSpawn` 0x185368:72 and `MP_EquipPlayer` 0x1855f8:16 both pass it, and `MP_ReSpawn` uses
/// the same byte to index the 8-slot roster). Weapon classes 2/4 → the PPK pick (`Upgrades+0`),
/// 6/8 → the P99 pick (+0), 0x1e/0x20/0x22 → the sniper pick (+3), 0x24/0x26/0x28 → the silenced
/// sniper pick (+3), 0x50 → the grapple pick (+1); anything else returns unchanged [M 0x1a9040].
/// A table pick ≥ 4 reads past the 4-entry table in the game; the port returns the weapon
/// unchanged there (*Deviation*, see the module doc).
pub fn upgrade_mp_weapon(index: u32, weapon: u16) -> u16 {
    let i = index as usize;
    if i >= 29 {
        return weapon;
    }
    let row = &UPGRADES[i];
    let pick = |table: &[u16; 4], byte: u8| -> u16 {
        if (byte as usize) < table.len() { table[byte as usize] } else { weapon }
    };
    match weapon {
        2 | 4 => pick(&UPGRADED_PPK_MP, row[0]),
        6 | 8 => pick(&UPGRADED_P99_MP, row[0]),
        0x1e | 0x20 | 0x22 => pick(&UPGRADED_SNIPERS, row[3]),
        0x24 | 0x26 | 0x28 => pick(&UPGRADED_SILENCED_SNIPERS, row[3]),
        0x50 => pick(&UPGRADED_GRAPPLE_MP, row[1]),
        _ => weapon,
    }
}

/// One weapon of the spawn loadout: (weapon id, rounds).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadoutWeapon {
    pub weapon: u16,
    pub rounds: u16,
}

/// `MP_EquipPlayer` 0x1855f8 — the MP spawn loadout, as data (the port equips through its own
/// player layer):
/// 1. the weapon set's kind-7 weapon, upgraded for the slot (`Upgrade_MPWeapon(slot, w)`), with
///    `weapon_data[w]+0x92 · 2` rounds [M 0x1855f8:15-23; the ×2 is the decompile's
///    `(s << 0x11) >> 0x10`];
/// 2. when that weapon is 0x1a, also ammo weapon 0x1b with `weapon_data[0x1b]+0x92` rounds
///    [M :24-26 — `weapon_data._7382_2_` is byte 7382 = record 0x1b + 0x92];
/// 3. when `MPSettings+0x1D0` (grapple) != 0, the grapple weapon `Upgrade_MPWeapon(slot, 0x50)`
///    with 0 rounds [M :27-34];
/// 4. the character-gadget: the equipped model's first hash 0x5000085 → weapon 0x45 ×1 [M :35-38]
///    (the port takes the gadget id as a parameter; `None` = not the watch gadget);
/// 5. the scenario kit: Protection (0x20000080) team 0 or Demolition (0x20000040) team 1 →
///    weapon 0x3b ×1 [M :42-47].
///
/// `set_weapon`/`set_rounds` come from the weapon table (the +0x92 field of the set's kind-7
/// weapon and of 0x1b); the loadout's own ids are exact.
pub fn equip_player(
    slot: u32,
    set: u32,
    matrix: &[[u16; 5]; 11],
    grapple: u32,
    scenario: u32,
    team: u32,
    set_weapon_rounds: u16,
    ammo_weapon_rounds: u16,
    watch_gadget: bool,
) -> Vec<LoadoutWeapon> {
    let mut out = Vec::new();
    let set_weapon = matrix.get(set as usize).map(|r| r[0]).unwrap_or(0);
    let w = upgrade_mp_weapon(slot, set_weapon);
    out.push(LoadoutWeapon { weapon: w, rounds: (set_weapon_rounds as u32 * 2).min(0xffff) as u16 });
    if w == 0x1a {
        out.push(LoadoutWeapon {
            weapon: 0x1b,
            rounds: ammo_weapon_rounds.min(0xfffe),
        });
    }
    if grapple != 0 {
        out.push(LoadoutWeapon { weapon: upgrade_mp_weapon(slot, 0x50), rounds: 0 });
    }
    if watch_gadget {
        out.push(LoadoutWeapon { weapon: 0x45, rounds: 1 });
    }
    let kit = (scenario == 0x2000_0080 && team == 0) || (scenario == 0x2000_0040 && team == 1);
    if kit {
        out.push(LoadoutWeapon { weapon: 0x3b, rounds: 1 });
    }
    out
}

/// `GunImp_Create` 0x131f20's weapon class: in SP (`master == 0`) class 0xb; in MP the toggle
/// `MPSettings+0x1B8` 0 creates no emplacement (`None`), the Random set (10) draws `Rand(10)`,
/// otherwise the class is the weapon set id itself [M 0x131f20:50-62].
pub fn gun_imp_class(master: u32, enabled: u32, set: u32, rng: &mut Rng) -> Option<i32> {
    if master == 0 {
        return Some(0xb);
    }
    if enabled == 0 {
        return None;
    }
    if set == 10 {
        Some(rng.rand(10) as i32)
    } else {
        Some(set as i32)
    }
}

/// `Car_Create` 0x1305b0's vehicle kind: `None` = no vehicle (MP with `MPSettings+0x1CC` == 0 [M
/// :13-15], or the Ravine arena 0x700004b [M :17-19]); 1 tank / 2 helicopter map directly, 3
/// (Random) draws `Rand(100) < 0x33` (51 %) → helicopter else tank [M :29-42]; in SP the game
/// builds a helicopter (+0x112 = 1) [M :33-39]. The returned byte is the game's `obj+0x112`
/// (0 tank, 1 helicopter); the caller owns the 8-tank cap [M :21].
pub fn car_kind(master: u32, vehicles: u32, level: u32, rng: &mut Rng) -> Option<u8> {
    if master != 0 && vehicles == 0 {
        return None;
    }
    if level == 0x7000_04b {
        return None;
    }
    if master == 0 {
        return Some(1);
    }
    Some(match vehicles {
        2 => 1,
        3 => {
            if rng.rand(100) < 0x33 {
                1
            } else {
                0
            }
        }
        v if v < 3 => {
            if v == 1 {
                0
            } else {
                1
            }
        }
        _ => 0,
    })
}

/// One `MPpickups` record (0xa0 bytes @0x2a4b50, 64 entries) — the fields the game's MP code
/// reads: the registered pickup object (the game stores the `obj_tag*` at record+0 [M
/// 0x189d80:76]), the placement position (obj+0x30 copied to record+0x10 [M :89-92]), the cel
/// (record+0x20 [M :93]) and the four per-slot visit stamps (record+0x80, `float[4]` [M
/// 0x18a168:18-26]).
#[derive(Clone, Copy, Debug, Default)]
pub struct MpPickupRecord {
    /// The registered pickup (the port's pickup index; `None` = the game's null pointer).
    pub obj: Option<usize>,
    /// record+0x10: the pickup's position.
    pub pos: [f32; 3],
    /// record+0x20: the cel record.
    pub cel: u32,
    /// record+0x80: the four per-slot "visited until" stamps (`MPGame+0x19C` clock floats).
    pub visit: [f32; 4],
    /// `PICKUPINFO+0x2E` (the eviction key): the pickup's lifetime ticks.
    pub lifetime: u16,
}

/// The MP pickup layer: the 64 `MPpickups` records, `PickupNextAddIndex`/`PickupLastDeletedIndex`
/// (@0x30d77c/@0x30d778) and the registered count (`MPSettings+0x1D8` u16, capped by the 64
/// slots) [M 0x189d80].
#[derive(Clone, Debug)]
pub struct MpPickups {
    /// The 64 records (`memset 0x2800` at `MP_Init` 0x183438:37).
    pub records: Vec<MpPickupRecord>,
    /// The live `PickupMatrix` (the shipped table with row 10 re-rolled per match).
    pub matrix: [[u16; 5]; 11],
    /// `PickupNextAddIndex` (init −1 → 0 by `MP_Init` [M 0x183438:40,43]).
    pub next_add: usize,
    /// `PickupLastDeletedIndex` (init 0xffffffff [M 0x183438:40]; `None` = −1).
    pub last_deleted: Option<usize>,
}

impl MpPickups {
    /// The fresh state: the shipped table, row 10 zeroed [D ELF], the record array cleared.
    pub fn new() -> MpPickups {
        MpPickups {
            records: vec![MpPickupRecord::default(); 64],
            matrix: PICKUP_MATRIX,
            next_add: 0,
            last_deleted: None,
        }
    }

    /// `Pickup_MakeRandomWeaponSet` into the live matrix (row 10) — `MP_Init` 0x183438:11 calls it
    /// for every arena except Ravine (0x700004b) [M 0x183438:10-11].
    pub fn reroll_random_set(&mut self, rng: &mut Rng) {
        self.matrix[10] = make_random_weapon_set(rng);
    }

    /// `MP_RegisterPickup` 0x189d80: the free-slot search — `PickupLastDeletedIndex` first (if
    /// still free), else the first free slot from `PickupNextAddIndex`, else the lowest free slot
    /// from 1, else evict the registered pickup with the smallest non-zero lifetime among the
    /// first `count` [M :27-73]. Returns the slot; `count` is the caller's registered count (the
    /// game's `MPSettings+0x1D8`), bumped by the caller [M :107].
    pub fn register(
        &mut self,
        obj: usize,
        pos: [f32; 3],
        cel: u32,
        lifetime: u16,
        count: u16,
    ) -> usize {
        let mut slot: Option<usize> = None;
        if let Some(d) = self.last_deleted {
            if self.records[d].obj.is_none() {
                slot = Some(d);
                self.last_deleted = None;
            }
        }
        if slot.is_none() {
            let mut i = self.next_add;
            while i < 64 && self.records[i].obj.is_some() {
                i += 1;
            }
            if i < 64 {
                slot = Some(i);
            }
        }
        if slot.is_none() && self.next_add > 0 {
            let mut i = 1;
            while i < self.next_add && self.records[i].obj.is_some() {
                i += 1;
            }
            if i < self.next_add {
                slot = Some(i);
            }
        }
        let slot = match slot {
            Some(s) => s,
            None => {
                // Full: evict the smallest non-zero lifetime among the first `count` records [M
                // 0x189d80:50-72].
                let mut best: Option<usize> = None;
                let mut best_life = 0xffffu16;
                for i in 0..(count as usize).min(64) {
                    if self.records[i].obj.is_some()
                        && self.records[i].lifetime != 0
                        && self.records[i].lifetime < best_life
                    {
                        best_life = self.records[i].lifetime;
                        best = Some(i);
                    }
                }
                match best {
                    Some(b) => b,
                    None => return usize::MAX,
                }
            }
        };
        self.records[slot] = MpPickupRecord { obj: Some(obj), pos, cel, visit: [0.0; 4], lifetime };
        self.next_add = slot + 1;
        if self.next_add > 0x3e {
            self.next_add = 0;
        }
        slot
    }

    /// `MP_UnregisterPickup` 0x18a058: clear the record and remember the slot [M :13-17].
    pub fn unregister(&mut self, slot: usize) {
        self.records[slot] = MpPickupRecord::default();
        self.last_deleted = Some(slot);
    }

    /// `MP_Pickup_Process` 0x18a168: for each of the first `count` records with an object, the
    /// four visit stamps that are set and behind the clock are zeroed [M :14-27].
    pub fn process(&mut self, clock: f32, count: u16) {
        for rec in self.records.iter_mut().take((count as usize).min(64)) {
            if rec.obj.is_none() {
                continue;
            }
            for v in rec.visit.iter_mut() {
                if *v != 0.0 && *v < clock {
                    *v = 0.0;
                }
            }
        }
    }

    /// `BOTSTATE_setPickupVisitTime` 0x129958: slot `bot`'s stamp on `record` becomes clock+45
    /// (+0.1 when that is 0.0) [M :9-14].
    pub fn set_visit_time(&mut self, record: usize, bot: usize, clock: f32) {
        if record >= 64 || bot >= 4 {
            return;
        }
        let mut t = clock + 45.0;
        if t == 0.0 {
            t += 0.1;
        }
        self.records[record].visit[bot] = t;
    }
}

impl Default for MpPickups {
    fn default() -> MpPickups {
        MpPickups::new()
    }
}
