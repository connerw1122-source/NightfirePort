//! mp-pickups-respawn: the weapon-set tables, the pickup registry/respawn bookkeeping, the spawn
//! policy and the enviro toggles — numbers only (the ELF dumps and the decompiled arithmetic), no
//! ISO needed. Evidence tags in the module docs of `nf_game::mp::pickups` and `nf_game::pickup`.

use nf_game::drone::Rng;
use nf_game::mp::{
    car_kind, equip_player, gun_imp_class, make_random_weapon_set, upgrade_mp_weapon,
    weapon_for_kind, MpMatch, MpPickups, MpSettings, PICKUP_MATRIX, UPGRADED_GRAPPLE_MP,
    UPGRADED_P99_MP, UPGRADED_PPK_MP, UPGRADED_SILENCED_SNIPERS, UPGRADED_SNIPERS, USEABLE_GUNS,
};

fn match_with(level: u32) -> MpMatch {
    let mut s = MpSettings::deathmatch(level);
    s.scenario = nf_game::mp::scenario::DEATHMATCH;
    MpMatch::init(s)
}

/// `PickupMatrix` @0x2b9100, `UseableGuns` @0x2b9170 and the five `Upgrade_MPWeapon` tables are
/// the on-disc bytes [D ELF 0x2b9100/0x2b9170/0x2bee28/0x2bee38/0x2bee78/0x2bee88/0x2bee98].
#[test]
fn weapon_set_tables_pin_the_elf_values() {
    let expected_matrix: [[u16; 5]; 11] = [
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
    assert_eq!(PICKUP_MATRIX, expected_matrix);
    assert_eq!(
        USEABLE_GUNS,
        [
            0x2, 0x6, 0xa, 0xc, 0xe, 0x10, 0x12, 0x14, 0x16, 0x19, 0x1a, 0x1c, 0x1e, 0x24, 0x2a,
            0x2c, 0x2e, 0x33, 0x37, 0x35, 0x36, 0x34, 0x3a, 0x3b, 0x42, 0x11, 0x52
        ]
    );
    assert_eq!(UPGRADED_PPK_MP, [0x2, 0x4, 0x4, 0x4]);
    assert_eq!(UPGRADED_P99_MP, [0x6, 0x6, 0x6, 0x8]);
    assert_eq!(UPGRADED_GRAPPLE_MP, [0x50, 0x51, 0x51, 0x51]);
    assert_eq!(UPGRADED_SNIPERS, [0x1e, 0x20, 0x22, 0x22]);
    assert_eq!(UPGRADED_SILENCED_SNIPERS, [0x24, 0x26, 0x28, 0x28]);
    // The row 0 alias in pickup.rs (the SP set — MPSettings+0x1B4 = 0 there).
    assert_eq!(nf_game::pickup::PICKUP_MATRIX_ROW0, expected_matrix[0]);
}

/// `Pickup_CreateFromSet` 0x18f1b0: the weapon is `PickupMatrix[set][kind-7]`, set = the u16 at
/// MPSettings+0x1B4 [M :56]. Kinds outside 7..=11 never reach it (`Pickup_Create` 0x18dda0:104's
/// `uVar7 - 7 < 5` intercept) and the Random set is the live matrix's row 10.
#[test]
fn create_from_set_maps_kinds_per_set() {
    let m = MpPickups::new();
    // Set 0 (Normal): kind 7 → 0x6 (P99), 8 → 0x1c, 9 → 0x1e, 10 → 0x18, 11 → 0x2c.
    for (kind, want) in [(7u32, 0x6u16), (8, 0x1c), (9, 0x1e), (10, 0x18), (11, 0x2c)] {
        assert_eq!(weapon_for_kind(0, kind, &m.matrix), Some(want), "set 0 kind {kind}");
    }
    // Set 4 (Explosives 1) row: 0x2a, 0x2c, 0x3a, 0x2e, 0x1a; set 8 ends 0x2c.
    assert_eq!(weapon_for_kind(4, 7, &m.matrix), Some(0x2a));
    assert_eq!(weapon_for_kind(4, 11, &m.matrix), Some(0x1a));
    assert_eq!(weapon_for_kind(8, 11, &m.matrix), Some(0x2c));
    // Out-of-range kinds/sets.
    assert_eq!(weapon_for_kind(0, 6, &m.matrix), None);
    assert_eq!(weapon_for_kind(0, 12, &m.matrix), None);
    assert_eq!(weapon_for_kind(11, 7, &m.matrix), None);
}

/// `Pickup_MakeRandomWeaponSet` 0x18f098: same seed → same row; every entry is a `UseableGuns`
/// id; no duplicates within the row; slot 0 samples guns[0..26] (never guns[26] = 0x52), slots
/// 1-4 sample guns[0..27].
#[test]
fn random_set_is_deterministic_and_deduped() {
    let mut a = Rng::new(0x5eed);
    let mut b = Rng::new(0x5eed);
    let ra = make_random_weapon_set(&mut a);
    let rb = make_random_weapon_set(&mut b);
    assert_eq!(ra, rb, "same seed, same draws");
    for (slot, gun) in ra.iter().enumerate() {
        let idx = USEABLE_GUNS.iter().position(|g| g == gun).unwrap_or(99);
        assert!(idx < 27, "gun {gun:#x} not in UseableGuns");
        if slot == 0 {
            assert!(idx < 26, "slot 0 must sample guns[0..26], got guns[{idx}]");
        }
    }
    for i in 0..5 {
        for j in (i + 1)..5 {
            assert_ne!(ra[i], ra[j], "duplicate at slots {i}/{j}");
        }
    }
    // Determinism holds across the whole re-roll too.
    let mut m1 = match_with(0x0700_0023);
    let mut m2 = match_with(0x0700_0023);
    let mut r1 = Rng::new(7);
    let mut r2 = Rng::new(7);
    m1.reroll_random_weapon_set(&mut r1);
    m2.reroll_random_weapon_set(&mut r2);
    assert_eq!(m1.pickups.matrix[10], m2.pickups.matrix[10]);
    assert!(m1.pickups.matrix[10] != [0u16; 5], "the shipped zeros are re-rolled");
    // And the front-end level (0x07000048) keeps them [M 0x183438:10].
    let mut fe = match_with(0x0700_0048);
    fe.reroll_random_weapon_set(&mut Rng::new(7));
    assert_eq!(fe.pickups.matrix[10], [0u16; 5]);
}

/// `MP_RegisterPickup` 0x189d80: sequential slots, `MPSettings+0x1D8` counts, the full ring
/// evicts the smallest non-zero `PICKUPINFO+0x2E` lifetime [M :52-72], and
/// `MP_UnregisterPickup` 0x18a058 hands the slot back through `PickupLastDeletedIndex` [M :13-17
/// + 0x189d80:29].
#[test]
fn register_pickup_ring_eviction_and_count() {
    let mut m = match_with(0x0700_0023);
    for i in 0..64 {
        let slot = m.register_pickup(i, [i as f32, 1.0, 2.0], 0x100 + i as u32, 100 + i as u16);
        assert_eq!(slot, i as usize);
    }
    assert_eq!(m.settings.registered_pickups, 64);
    // Full: the smallest lifetime (100, slot 0) is evicted.
    let slot = m.register_pickup(0x1234, [9.0; 3], 0x999, 55);
    assert_eq!(slot, 0);
    assert_eq!(m.pickups.records[0].obj, Some(0x1234));
    assert_eq!(m.settings.registered_pickups, 65);
    // Unregister frees the slot and the next add reuses it first.
    m.unregister_pickup(3);
    assert_eq!(m.settings.registered_pickups, 64);
    let slot = m.register_pickup(0x77, [1.0; 3], 0x1, 10);
    assert_eq!(slot, 3);
}

/// `MP_Pickup_Process` 0x18a168: a set stamp survives while the clock is behind it and is zeroed
/// once the clock passes it [M :20-26]; `BOTSTATE_setPickupVisitTime` 0x129958 writes clock+45
/// (+0.1 when that would be 0.0) [M :9-14], and only the first `count` records are walked [M
/// 0x18a168:12-14].
#[test]
fn pickup_process_and_visit_stamps() {
    let mut m = match_with(0x0700_0023);
    let s0 = m.register_pickup(1, [0.0; 3], 0, 0);
    let s1 = m.register_pickup(2, [1.0; 3], 0, 0);
    m.pickups.set_visit_time(s0, 0, 100.0);
    m.pickups.set_visit_time(s0, 3, 50.0);
    m.pickups.set_visit_time(s1, 1, 200.0);
    assert_eq!(m.pickups.records[s0].visit[0], 145.0);
    assert_eq!(m.pickups.records[s1].visit[1], 245.0);
    // Clock 144.9: the 145.0 stamp stands, the 95.0 one is behind the clock and zeroes.
    m.pickup_process(144.9);
    assert_eq!(m.pickups.records[s0].visit[0], 145.0);
    assert_eq!(m.pickups.records[s0].visit[3], 0.0);
    // Clock 145.5: the 145.0 stamp zeroes, the 200/245 ones stand.
    m.pickup_process(145.5);
    assert_eq!(m.pickups.records[s0].visit[0], 0.0);
    assert_eq!(m.pickups.records[s0].visit[3], 0.0);
    assert_eq!(m.pickups.records[s1].visit[1], 245.0);
    // The +0.1 guard: clock −45 → stamp 0.0 → +0.1.
    m.pickups.set_visit_time(s1, 2, -45.0);
    assert_eq!(m.pickups.records[s1].visit[2], 0.1);
}

/// `MP_GetSpawnPoint` 0x185098's policy word `MPSettings+0x1C0` (the menu's Near/Far/Random
/// values 0/1/2): 0 = the nearest candidate (> 2.0 squared units from every other player), 1 =
/// the farthest, 2 = `Rand_Rand(candidates)`; the team ranges split at 0x20 for team games [M
/// :85-98, 100-183]. Numbers only.
#[test]
fn spawn_policy_picks_the_games_point() {
    // FFA deathmatch, four spawn points at known distances from one other player.
    let mut m = match_with(0x0700_0023);
    // Register 4 points (team 0 → slots 0..4 in an FFA range [0,0x40)).
    let spots: [[f32; 3]; 4] = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [30.0, 0.0, 0.0], [100.0, 0.0, 0.0]];
    for (i, p) in spots.iter().enumerate() {
        m.register_spawn_point(*p, [0.0, 0.0, 1.0], 0, &no_world());
    }
    // One player at (5,0,0): squared distances 25, 25, 625, 9025 → candidates: all but spot 0
    // (25 > 2.0 — spot 0 is a candidate too, sq-dist 25; all 4 are candidates).
    m.set_slot(1, [5.0, 0.0, 0.0]);
    let mut rng = Rng::new(3);
    // Far (policy 1): spot 3 (9025).
    m.settings.spawn_policy = 1;
    assert_eq!(m.get_spawn_point(0, 0, &mut rng), 3);
    // Near (policy 0): the smallest sq-dist — spots 0/1 tie at 25; the tracker keeps the FIRST
    // strictly-smaller one, so the first 25 (spot 0) wins and spot 1 never beats it.
    m.settings.spawn_policy = 0;
    assert_eq!(m.get_spawn_point(0, 0, &mut rng), 0);
    // Random (policy 2): a fixed seed picks deterministically among the 4 candidates; the draw
    // is Rand_Rand(4) — replay the same seed against a bare Rng to pin the index.
    m.settings.spawn_policy = 2;
    let mut r2 = Rng::new(11);
    let want = r2.rand(4) as u32;
    assert_eq!(m.get_spawn_point(0, 0, &mut Rng::new(11)), want);
    // The nearest candidate excludes points within 2.0 squared units: park a player ON spot 2
    // (the spawn eye height is floor+1.6, the game's obj+0x30 [M 0x184f20:83]) and the near pick
    // moves off it.
    m.set_slot(2, [spots[2][0], 1.6, spots[2][2]]);
    m.settings.spawn_policy = 0;
    let picked = m.get_spawn_point(0, 0, &mut Rng::new(3));
    assert_ne!(picked, 2, "a spawn point occupied by another player is excluded");
    assert_eq!(picked, 0, "spot 0 (sq-dist 25) is the nearest remaining candidate");
    // Team games split the table at 0x20: with only team-0 points registered, a team-1 spawn in
    // a team game falls back to the range start (0x20) [M 0x185098:165-183].
    let mut t = match_with(0x0700_0023);
    t.settings.scenario = nf_game::mp::scenario::TEAM_DEATHMATCH;
    t.settings.team_game = 1;
    t.register_spawn_point([0.0; 3], [0.0, 0.0, 1.0], 0, &no_world());
    t.settings.spawn_policy = 1;
    assert_eq!(t.get_spawn_point(1, 0, &mut Rng::new(1)), 0x20);
    // FFA uses the whole range [0,0x40) — the same table is visible to team 0.
    assert_eq!(m.get_spawn_point(0, 0, &mut Rng::new(1)) < 0x40, true);
}

fn no_world() -> nf_game::world::CollisionWorld {
    nf_game::world::CollisionWorld::from_triangles(vec![])
}

/// `Upgrade_MPWeapon` 0x1a9040: classes 2/4 → the PPK pick, 6/8 → the P99 pick, the sniper
/// triples, 0x50 → the grapple pick; other weapons return unchanged [M 0x1a9040]. Indices 0-3
/// read the tables; the port returns the weapon unchanged for a pick ≥ 4 (the game reads past
/// the 4-entry table — *Deviation*, report).
#[test]
fn upgrade_mp_weapon_tables() {
    // Every clean index row is 0 for slots/chars 0-4 (Upgrades[0..45] all zero [D ELF]).
    assert_eq!(upgrade_mp_weapon(0, 2), 0x2);
    assert_eq!(upgrade_mp_weapon(0, 4), 0x2);
    assert_eq!(upgrade_mp_weapon(0, 6), 0x6);
    assert_eq!(upgrade_mp_weapon(0, 8), 0x6);
    assert_eq!(upgrade_mp_weapon(0, 0x50), 0x50);
    assert_eq!(upgrade_mp_weapon(0, 0x1e), 0x1e);
    assert_eq!(upgrade_mp_weapon(0, 0x24), 0x24);
    assert_eq!(upgrade_mp_weapon(3, 0x1a), 0x1a, "unrelated classes unchanged");
    // A non-zero flag picks the upgraded entry (the port's flag byte comes from the game's
    // on-disc Upgrades row — index 0 byte 0; force the table pick through a synthetic index is
    // not possible, so pin the pick tables directly).
    assert_eq!(UPGRADED_PPK_MP[1], 0x4);
    assert_eq!(UPGRADED_P99_MP[3], 0x8);
    assert_eq!(UPGRADED_GRAPPLE_MP[1], 0x51);
    assert_eq!(UPGRADED_SNIPERS[2], 0x22);
    assert_eq!(UPGRADED_SILENCED_SNIPERS[2], 0x28);
}

/// `MP_EquipPlayer` 0x1855f8 as data: the set's kind-7 weapon (upgraded, `+0x92 · 2` rounds), the
/// 0x1a → 0x1b companion ammo [M :24-26], the grapple gate at MPSettings+0x1D0 [M :27-34], the
/// watch gadget 0x45 ×1 [M :35-38] and the Protection/Demolition team kit 0x3b ×1 [M :42-47].
#[test]
fn equip_loadout_numbers() {
    let m = MpPickups::new();
    // Set 0 (Normal): kind-7 weapon 0x6 → P99 upgrade pick 0 (slot 0 flag 0) → 0x6, 30·2 rounds.
    let lo = equip_player(0, 0, &m.matrix, 0, 0x1, 0, 30, 12, false);
    assert_eq!(lo, vec![Loadout(0x6, 60)]);
    // Set 8 (State of the Art): kind-7 weapon 0x1a → the 0x1b companion joins.
    let lo = equip_player(0, 8, &m.matrix, 0, 0x1, 0, 5, 7, false);
    assert_eq!(lo, vec![Loadout(0x1a, 10), Loadout(0x1b, 7)]);
    // Grapple on (+0x1D0 != 0): Upgrade_MPWeapon(slot, 0x50) with 0 rounds.
    let lo = equip_player(0, 0, &m.matrix, 1, 0x1, 0, 30, 12, false);
    assert_eq!(lo, vec![Loadout(0x6, 60), Loadout(0x50, 0)]);
    // The watch gadget and the Demolition attacker kit (scenario 0x20000040, team 1).
    let lo = equip_player(0, 0, &m.matrix, 0, 0x2000_0040, 1, 30, 12, true);
    assert_eq!(lo, vec![Loadout(0x6, 60), Loadout(0x45, 1), Loadout(0x3b, 1)]);
    // Protection defender (team 0) gets the kit; attacker team 1 doesn't.
    let lo = equip_player(0, 0, &m.matrix, 0, 0x2000_0080, 0, 30, 12, false);
    assert_eq!(lo, vec![Loadout(0x6, 60), Loadout(0x3b, 1)]);
    let lo = equip_player(0, 0, &m.matrix, 0, 0x2000_0080, 1, 30, 12, false);
    assert_eq!(lo, vec![Loadout(0x6, 60)]);
    // Deathmatch: no kit either way.
    let lo = equip_player(0, 0, &m.matrix, 0, 0x1, 1, 30, 12, false);
    assert_eq!(lo, vec![Loadout(0x6, 60)]);
}
fn Loadout(w: u16, r: u16) -> nf_game::mp::LoadoutWeapon {
    nf_game::mp::LoadoutWeapon { weapon: w, rounds: r }
}

/// The enviro toggles: `GunImp_Create` 0x131f20:50-62 (SP class 0xb, the +0x1B8 gate, Random set
/// → Rand(10), else the set id) and `Car_Create` 0x1305b0:13-42 (the +0x1CC gate, Ravine never,
/// 1/2/3 = tank/heli/51%-heli, SP helicopter).
#[test]
fn gun_imp_and_car_toggles() {
    let mut rng = Rng::new(9);
    // SP: the emplacement exists with class 0xb whatever the toggles say.
    assert_eq!(gun_imp_class(0, 0, 0, &mut rng), Some(0xb));
    // MP, toggle off: nothing.
    assert_eq!(gun_imp_class(1, 0, 3, &mut rng), None);
    // MP, toggle on: the class is the set id; the Random set draws 0..9.
    assert_eq!(gun_imp_class(1, 1, 4, &mut rng), Some(4));
    for _ in 0..20 {
        let c = gun_imp_class(1, 1, 10, &mut rng).unwrap();
        assert!((0..10).contains(&c), "random class {c} out of 0..9");
    }
    // Vehicles: MP + 0 → none; Ravine (0x0700004b) → none always.
    assert_eq!(car_kind(1, 0, 0x0700_0023, &mut rng), None);
    assert_eq!(car_kind(1, 2, 0x0700_004b, &mut rng), None);
    assert_eq!(car_kind(1, 1, 0x0700_0023, &mut rng), Some(0)); // tanks
    assert_eq!(car_kind(1, 2, 0x0700_0023, &mut rng), Some(1)); // helicopters
    assert_eq!(car_kind(0, 0, 0x0700_0023, &mut rng), Some(1)); // SP: helicopter
    // Random: 51/100 helicopter. With a fixed seed the draw is deterministic — pin it.
    let mut r = Rng::new(5);
    let want = if r.rand(100) < 0x33 { 1u8 } else { 0u8 };
    assert_eq!(car_kind(1, 3, 0x0700_0023, &mut Rng::new(5)), Some(want));
}

/// The boot defaults of the three new settings fields [M 0x1c7e78:38,42,45]: +0x1B8 = 0,
/// +0x1CC = 0, +0x1D0 = 1.
#[test]
fn boot_defaults_for_the_toggles() {
    let s = MpSettings::deathmatch(0x0700_0023);
    assert_eq!(s.gun_emplacements, 0);
    assert_eq!(s.vehicles, 0);
    assert_eq!(s.grapple, 1);
}
