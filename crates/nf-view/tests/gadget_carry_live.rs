//! gadget-carry-map: which on-foot levels carry the thrown gadgets — Laser Tripbomb 0x3a, Satchel
//! 0x3b-0x40, Remote Mine 0x37-0x39, the grenade family 0x34 frag / 0x35 stun-flash / 0x36 smoke,
//! the Micro-Camera 0x54, the Q-Decryptor 0x56/0x57, the Q-Worm 0x58, the AT-600 Scorpion 0x2e —
//! for every story level, from the same sources the game uses:
//!
//! - the start loadout: `Player_InitWeapon` 0x1a7ef8's per-level jump table (ported as
//!   `nf_game::player::init_level_loadout`, fix-start-loadouts) — a cold load into the level;
//! - the pickups placed in the level archive [D]: placement kind 0xF0 params (kind 0 weapon /
//!   1 ammo, param 1 = the weapon id), read through the same records the sim builds
//!   (`Pickup_Create` 0x18dda0; the port's scene pickup build);
//! - the pickups a level SCRIPT spawns [D archive]: the NIS streams' `Cmd::Entity` obj_type 0xF0
//!   (the game's `Script_CreateEntity` path; the port's `script_pickup_reservations`);
//! - the captain grenade drop [M `DroneFunc_OnInitDeath` 0x145bd0, read at 0x145de8]:
//!   07000001-08 drop weapon 0x35 ×2, 07000009-0B / 11-16 / 4A drop 0x34 ×1 (docs/systems/pickups.md);
//! - the weapons a dead drone drops [M `DroneWeap_DropWeapon` 0x17a2a0]: the held weapon's base id
//!   (placement param 9) — a tracked base here means the level carries the gadget as a drop;
//! - the gadgets drones THROW [M `NDrone2_DefaultInit` 0x14b300 decode, placement param 11]:
//!   nibble 5 → 0x3a + low nibble (tripbomb 0x3a, satchel 0x3b, ...) — informational: the
//!   projectile/pickup meshes still load with the level.
//!
//! Per carried id the table also answers whether the view model (`weapon_data+0xDC`,
//! `Player_EquipWeapon` 0x1a8a58's hashtable gate) and the projectile mesh (`+0x5C`,
//! `Bullet_update` 0x12e7b0 disasm 0x12e8f0) resolve in the level's own archive (`level.by_hash`)
//! or through the session table (the game's hashtable accumulates every earlier story archive —
//! docs/models/weapons-items.md §2; the port's `load_impl` merges the same weapon-model hashes).
//!
//! Opt-in: NF_ISO (the slim ISO is enough).

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

/// The 20 story levels, ascending (the mission order — docs/systems/levels.md §1; the session
/// accumulates in this order, models_weapons_live's rule).
pub const STORY: [&str; 20] = ["07000001", "07000002", "07000003", "07000004", "07000005", "07000006", "07000007",
    "07000008", "07000009", "0700000a", "0700000b", "0700000c", "0700000d", "07000011", "07000012", "07000013",
    "07000014", "07000015", "07000016", "0700001b"];

/// The tracked gadget families, by base id (the disc name labels [L USATxt.dat via grenade.rs]:
/// 0x34 frag, 0x35 stun/flash, 0x36 smoke, 0x37-0x39 remote mine, 0x3a laser tripbomb, 0x3b-0x40
/// satchel; the AT-600 Scorpion 0x2E [L fix_weapons_all_live], Micro-Camera 0x54, Q-Decryptor
/// 0x56/0x57, Q-Worm 0x58 [M player.rs loadout cites]).
pub const TRACKED: [u16; 10] = [0x2e, 0x34, 0x35, 0x36, 0x37, 0x3a, 0x3b, 0x54, 0x56, 0x58];

/// The weapon id's tracked family base (twin 0x2f → 0x2e, mines 0x38/0x39 → 0x37, satchel
/// 0x3c-0x40 → 0x3b, decryptor twin 0x57 → 0x56). None = not tracked.
fn tracked_base(id: u16) -> Option<u16> {
    Some(match id {
        0x2e | 0x2f => 0x2e,
        0x34 => 0x34,
        0x35 => 0x35,
        0x36 => 0x36,
        0x37..=0x39 => 0x37,
        0x3a => 0x3a,
        0x3b..=0x40 => 0x3b,
        0x54 => 0x54,
        0x56 | 0x57 => 0x56,
        0x58 => 0x58,
        _ => return None,
    })
}

struct Disc {
    img: nf_formats::iso9660::IsoImage<std::fs::File>,
}

impl Disc {
    fn open(iso: &str) -> Disc {
        Disc { img: nf_formats::iso9660::IsoImage::open(iso).expect("open ISO") }
    }
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let (entries, bin) = nf_formats::disc::files_bin_entries(&self.img).ok()?;
        let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
        self.img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).ok()
    }
    fn level(&self, name: &str) -> nf_formats::level::Level {
        let data = self.read(name).unwrap_or_else(|| panic!("no {name} in FILES.BIN"));
        nf_formats::level::Level::parse(&data).expect("parse level")
    }
    fn weapons(&self) -> nf_formats::weapon::WeaponData {
        let e = self.img.find("/ACTION.ELF").expect("ACTION.ELF on the disc");
        let elf = self.img.read_file(e).expect("read ACTION.ELF");
        nf_formats::weapon::WeaponData::from_action_elf(&elf).expect("weapon_data")
    }
}

/// One source row: a weapon id (family base) with where it comes from.
#[derive(Default, Debug)]
struct Carry {
    /// Cold-load owned (init_level_loadout ∩ tracked).
    loadout: Vec<u16>,
    /// Archive-placed pickups: (id, translation, rounds).
    placed: Vec<(u16, [f32; 3], i16)>,
    /// Script-spawned pickups: (id, stream key position).
    script: Vec<(u16, [f32; 3])>,
    /// The captain grenade drop (DroneFunc_OnInitDeath): (id, count).
    captain: Option<(u16, i16)>,
    /// Held weapons the drones drop on death (DroneWeap_DropWeapon), tracked bases only.
    drone_drop: Vec<u16>,
    /// Gadgets drones throw (param 11 decode), tracked bases only.
    thrown: Vec<u16>,
}

impl Carry {
    fn carries(&self, base: u16) -> bool {
        self.loadout.contains(&base) || self.placed.iter().any(|(id, _, _)| tracked_base(*id) == Some(base))
            || self.script.iter().any(|(id, _)| tracked_base(*id) == Some(base))
            || self.captain.map_or(false, |(id, _)| id == base)
            || self.drone_drop.contains(&base) || self.thrown.contains(&base)
    }
    fn carried_bases(&self) -> Vec<u16> {
        TRACKED.into_iter().filter(|&b| self.carries(b)).collect()
    }
}

/// The captain grenade drop [M DroneFunc_OnInitDeath 0x145bd0, the `a2` read at 0x145de8]:
/// 07000001-08 → 0x35 with 2; 07000009-0B / 11-16 / 4A → 0x34 with 1 (docs/systems/pickups.md).
fn captain_grenade(level_id: u32) -> Option<(u16, i16)> {
    match level_id & 0xff_ffff {
        0x01..=0x08 => Some((0x35, 2)),
        0x09..=0x0b | 0x11..=0x16 | 0x4a => Some((0x34, 1)),
        _ => None,
    }
}

/// The drone grenade decode [M NDrone2_DefaultInit 0x14b300:348-376, placement param 11's
/// selection nibble; the 0x07000014 clamp at :374-376]: 1 → 0x36, 2 → 0x35, 4 → 0x34,
/// 5 → 0x3a + low nibble, 6 → 0x6c.
fn drone_grenade(p11: u32, level_id: u32) -> u16 {
    let gw = match ((p11 >> 4) & 0xffff) as u16 {
        1 => 0x36, 2 => 0x35, 4 => 0x34, 5 => ((p11 & 0xf).wrapping_add(0x3a)) as u16, 6 => 0x6c, _ => 0,
    };
    if level_id == 0x0700_0014 && (0x34..0x37).contains(&gw) { 0x34 } else { gw }
}

/// The level's carry row, from the real data.
fn carry_row(d: &Disc, name: &str, weapons: &nf_formats::weapon::WeaponData) -> (u32, Carry) {
    let level = d.level(&format!("{name}.bin"));
    let mm = level.main_map().expect("main map");
    let level_id = u32::from_str_radix(name, 16).unwrap();
    let mut c = Carry::default();

    // The start loadout [M 0x1a7ef8]: the port's transcript, a cold load (no RamSave).
    let mut inv = nf_game::pickup::Inventory::empty(0x73, 40);
    let _ = nf_game::player::init_level_loadout(level_id, None, &mut nf_game::bullet::PlayerHealth::default(),
        &mut inv, &mut nf_game::triggers::Channels::default(), weapons, |w| weapons.first_person(w as usize) != 0);
    for (w, &owned) in inv.owned.iter().enumerate() {
        if owned { if let Some(b) = tracked_base(w as u16) { if !c.loadout.contains(&b) { c.loadout.push(b); } } }
    }

    // The archive's placed pickups [D]: kind 0xF0, pickup kinds 0 (weapon) / 1 (ammo).
    for p in &level.maps[mm].placements {
        if p.flags & 0xff != 0xf0 { continue; }
        let param = |i: u32| p.params.iter().find(|(k, _)| *k == i).map(|(_, v)| *v).unwrap_or(0);
        let (kind, id) = (param(0), param(1) as u16);
        if matches!(kind, 0 | 1) && tracked_base(id).is_some() {
            c.placed.push((id, p.translation, param(2) as i16));
        }
    }
    for p in &level.maps[mm].placements {
        if p.flags & 0xff != 0x0f { continue; }
        let param = |i: u32| p.params.iter().find(|(k, _)| *k == i).map(|(_, v)| *v).unwrap_or(0);
        let base = weapons.base(param(9) as usize) as u16;
        if let Some(b) = tracked_base(base) { if !c.drone_drop.contains(&b) { c.drone_drop.push(b); } }
        let gw = drone_grenade(param(11), level_id);
        if let Some(b) = tracked_base(gw) { if !c.thrown.contains(&b) { c.thrown.push(b); } }
    }

    // The scripts' pickups [D]: the NIS streams' obj_type-0xF0 entity spawns.
    let mut hashes: Vec<u32> = level.nis.keys().copied().collect();
    hashes.sort_unstable();
    for hash in hashes {
        let nis = &level.nis[&hash];
        for st in &nis.streams {
            let cmds = st.decode();
            let Some(nf_formats::nis::Cmd::Entity { obj_type: 0xf0, params, .. }) =
                cmds.iter().find(|c| matches!(c, nf_formats::nis::Cmd::Entity { obj_type: 0xf0, .. })) else { continue };
            let p = |k: usize| params.get(k).copied().unwrap_or(0);
            let (kind, id) = (p(0), p(1) as u16);
            if matches!(kind, 0 | 1) && tracked_base(id).is_some() {
                let key = nis.keys.get(st.first_key as usize).map(|k| k.pos).unwrap_or([0.0; 4]);
                c.script.push((id, [key[0], key[1], key[2]]));
            }
        }
    }

    c.captain = captain_grenade(level_id);
    (level_id, c)
}

/// Whether the view model (+0xDC) and the projectile mesh (+0x5C) of `base` resolve in the level's
/// own archive or through the session (earlier story archives' union — the game's hashtable rule,
/// docs/models/weapons-items.md §2). The view model is a character skin (`level.skins`, the
/// models_weapons_live fp rule — `Player_EquipWeapon` 0x1a8a58's hashtable gate); the projectile
/// mesh is a map object (`level.by_hash`). Returns ("local"|"session"|"MISSING") per mesh.
fn mesh_resolves(level: &nf_formats::level::Level, session: &std::collections::HashSet<u32>, weapons: &nf_formats::weapon::WeaponData, base: u16) -> (&'static str, &'static str) {
    let fp = weapons.first_person(base as usize);
    let proj = weapons.projectile_mesh(base as usize);
    let hit = |h: u32| if h == 0 { "none" }
        else if level.by_hash.contains_key(&h) || level.skins.contains_key(&h) { "local" }
        else if session.contains(&h) { "session" }
        else { "MISSING" };
    (hit(fp), hit(proj))
}

/// The whole map, printed as the doc table's source. Returns the rows.
fn build_map(d: &Disc, weapons: &nf_formats::weapon::WeaponData) -> Vec<(u32, Carry, String)> {
    // (the session union below mixes skins and map objects — both accumulate in the game's table)
    // The session union: every story archive before this one, in story order.
    let mut session: std::collections::HashSet<u32> = Default::default();
    let mut out = Vec::new();
    for name in STORY {
        let level = d.level(&format!("{name}.bin"));
        let (level_id, c) = carry_row(d, name, weapons);
        let mut line = format!("{name}:");
        for b in c.carried_bases() {
            let (fp, proj) = mesh_resolves(&level, &session, weapons, b);
            line.push_str(&format!(" {b:#04x}(fp {fp}, proj {proj})"));
        }
        out.push((level_id, c, line.clone()));
        for h in level.by_hash.keys() { session.insert(*h); }
        for h in level.skins.keys() { session.insert(*h); }
    }
    out
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

/// The map itself, printed (the doc table's source — docs/playtest/gadget-carry.md).
#[test]
fn print_the_carry_map() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let d = Disc::open(&iso);
    let weapons = d.weapons();
    let rows = build_map(&d, &weapons);
    for (level_id, c, line) in &rows {
        eprintln!("MAP {line}");
        if !c.placed.is_empty() { eprintln!("     placed: {:?}", c.placed.iter().map(|(id, p, r)| format!("{id:#04x} rounds {r} @ ({:.2},{:.2},{:.2})", p[0], p[1], p[2])).collect::<Vec<_>>()); }
        if !c.script.is_empty() { eprintln!("     script: {:?}", c.script.iter().map(|(id, p)| format!("{id:#04x} @ ({:.2},{:.2},{:.2})", p[0], p[1], p[2])).collect::<Vec<_>>()); }
        if !c.drone_drop.is_empty() { eprintln!("     drone drops held: {:?}", c.drone_drop.iter().map(|b| format!("{b:#04x}")).collect::<Vec<_>>()); }
        if !c.thrown.is_empty() { eprintln!("     drones throw: {:?}", c.thrown.iter().map(|b| format!("{b:#04x}")).collect::<Vec<_>>()); }
        if let Some((id, n)) = c.captain { eprintln!("     captain drop: {id:#04x} x{n}"); }
        let _ = level_id;
    }
}

/// The loadout rows: what the port's `Player_InitWeapon` grants, intersected with the tracked
/// gadgets, per level. Pins the decompile's per-level kits (fix-start-loadouts' transcript).
#[test]
fn loadout_gadget_rows_match_the_initweapon_table() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let d = Disc::open(&iso);
    let weapons = d.weapons();
    let rows: Vec<(u32, Carry)> = STORY.iter().map(|n| carry_row(&d, n, &weapons)).collect();
    let get = |id: u32| rows.iter().find(|(l, _)| *l == id).unwrap();
    // 07000014 kit: frag 0x34 ×5, mine 0x37 ×5, camera 0x54, decryptor 0x56 — and NOTHING else
    // tracked (no tripbomb/satchel/scorpion) [M 0x1a7ef8 case 0x7000014].
    let (_, c14) = get(0x0700_0014);
    assert_eq!(c14.loadout, vec![0x34, 0x37, 0x54, 0x56], "14's tracked loadout");
    // 07000009 kit: decryptor 0x56 + Q-Worm 0x58, no grenades [M 0x1a7ef8 case 0x7000009].
    let (_, c09) = get(0x0700_0009);
    assert_eq!(c09.loadout, vec![0x56, 0x58], "09's tracked loadout");
    // 0700000c kit: camera 0x54 + Q-Worm 0x58, no grenades [M 0x1a7ef8 case 0x700000c].
    let (_, c0c) = get(0x0700_000c);
    assert_eq!(c0c.loadout, vec![0x54, 0x58], "0c's tracked loadout");
    // 07000005 kit: camera 0x54 only [M 0x1a7ef8 case 0x7000005].
    let (_, c05) = get(0x0700_0005);
    assert_eq!(c05.loadout, vec![0x54], "05's tracked loadout");
    // 07000001 kit: nothing tracked [M 0x1a7ef8 case 0x7000001].
    let (_, c01) = get(0x0700_0001);
    assert!(c01.loadout.is_empty(), "01's tracked loadout must be empty, got {:?}", c01.loadout);
    // Sister: 07000015's cold load re-runs the 14 kit [M 0x1a7ef8 case 0x7000015 → case 0x7000014].
    let (_, c15) = get(0x0700_0015);
    assert_eq!(c15.loadout, c14.loadout, "15's cold load IS the 14 kit");
}

/// The captain drop row [M 0x145bd0]: 01-08 drop 0x35 ×2; 09-0B/11-16/4A drop 0x34 ×1.
#[test]
fn captain_grenade_drops_match_the_decompile() {
    for (id, want) in [(0x0700_0001u32, Some((0x35u16, 2i16))), (0x0700_0008, Some((0x35, 2))),
                       (0x0700_0009, Some((0x34, 1))), (0x0700_000b, Some((0x34, 1))),
                       (0x0700_0011, Some((0x34, 1))), (0x0700_0016, Some((0x34, 1))),
                       (0x0700_004a, Some((0x34, 1))), (0x0700_001b, None), (0x0700_0041, None)] {
        assert_eq!(captain_grenade(id), want, "{id:#x}'s captain drop");
    }
    // Sister: the captain grenade is never a tripbomb or satchel — the game's own id list.
    for id in 0x0700_0001..=0x0700_0016 {
        if let Some((w, _)) = captain_grenade(id) { assert!(w == 0x34 || w == 0x35, "{id:#x} drops {w:#04x}?"); }
    }
}

/// The tripbomb's absence rows: 07000001 and 07000009 carry no tripbomb 0x3a from ANY source
/// (loadout, placed pickup, script spawn, captain drop, drone drop, drone throw) — the game's own
/// data. The main agent must not `give 0x3a` there expecting level-native meshes.
#[test]
fn tripbomb_is_absent_where_the_map_says() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let d = Disc::open(&iso);
    let weapons = d.weapons();
    for name in ["07000001", "07000009"] {
        let (_, c) = carry_row(&d, name, &weapons);
        assert!(!c.carries(0x3a), "{name}: tripbomb must be absent, row {c:?}");
        // Sister: the raw sources agree — no placed/scripted pickup with id 0x3a, the loadout
        // does not own it.
        assert!(!c.placed.iter().any(|(id, _, _)| *id == 0x3a), "{name}: no placed 0x3a");
        assert!(!c.script.iter().any(|(id, _)| *id == 0x3a), "{name}: no script 0x3a");
        assert!(!c.loadout.contains(&0x3a), "{name}: loadout holds no 0x3a");
    }
}

/// The placed pickup rows, pinned from the archive scan ([D] — the same records the sim builds).
/// Pins counts + ids + rounds for the gadget-bearing levels; a change means a pickup moved onto or
/// off a level and the doc table must be updated with it.
#[test]
fn placed_gadget_pickups_match_the_archive_scan() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let d = Disc::open(&iso);
    let weapons = d.weapons();
    let ids = |name: &str| carry_row(&d, name, &weapons).1.placed;
    // 07000011: the satchel ×1 (rounds 5) + frag ×6 (rounds 1) [D 07000011.bin].
    let p11 = ids("07000011");
    assert_eq!(p11.iter().filter(|(id, _, _)| *id == 0x3b).count(), 1, "11's satchel pickups");
    assert_eq!(p11.iter().filter(|(id, _, _)| *id == 0x34).count(), 6, "11's frag pickups");
    assert_eq!(p11.iter().find(|(id, _, _)| *id == 0x3b).unwrap().2, 5, "the satchel's rounds");
    let satchel_at = p11.iter().find(|(id, _, _)| *id == 0x3b).unwrap().1;
    assert!((satchel_at[0] - 14.79).abs() < 0.1 && (satchel_at[2] - 5.33).abs() < 0.1, "the satchel placement's spot {satchel_at:?}");
    // 07000013: tripbomb ×3, rounds 1 each, on the tower (-369 y) [D 07000013.bin].
    let p13 = ids("07000013");
    assert_eq!(p13.iter().filter(|(id, _, _)| *id == 0x3a).count(), 3, "13's tripbomb pickups");
    assert!(p13.iter().all(|(id, _, r)| *id != 0x3a || *r == 1), "13's tripbombs are rounds 1");
    // 07000014: frag ×5, tripbomb ×2 (rounds 5), mine 0x37 ×3 (rounds 5) [D 07000014.bin].
    let p14 = ids("07000014");
    assert_eq!(p14.iter().filter(|(id, _, _)| *id == 0x34).count(), 5, "14's frag pickups");
    assert_eq!(p14.iter().filter(|(id, _, _)| *id == 0x3a).count(), 2, "14's tripbomb pickups");
    assert_eq!(p14.iter().filter(|(id, _, _)| *id == 0x37).count(), 3, "14's mine pickups");
    // 0700000b: the Scorpion pickup ×1 (rounds 8) [D 0700000b.bin].
    let p0b = ids("0700000b");
    assert_eq!(p0b.iter().filter(|(id, _, _)| *id == 0x2e).count(), 1, "0b's Scorpion pickups");
    // Sister: 07000005 carries NO gadget pickups at all (its 0x35 rides the captain drop, its
    // 0x54 the loadout — the placement scan must stay empty).
    assert!(ids("07000005").is_empty(), "05 must have no placed gadget pickups");
    // Sister: the tripbomb is placed on 13/14 only.
    for name in STORY {
        if name == "07000013" || name == "07000014" { continue; }
        assert!(!ids(name).iter().any(|(id, _, _)| *id == 0x3a), "{name}: unexpected placed tripbomb");
    }
}

/// The wall spot for the tripbomb and the satchel: 07000005 (the gadget playtest level, 05's spawn
/// courtyard), standing ~1.7 m from the courtyard's flat south wall (normal (−0.91, 0, −0.41),
/// measured by the direction fan — 12 of 48 rays hit this one wall, everything else is clear ≥ 3 m).
/// Headless checks: grounded after 60 ticks, a wall hit within 2 m ahead. This is the `--bond`
/// spot the main agent throws from (docs/playtest/gadget-carry.md §wall spots).
#[test]
fn the_07000005_wall_spot_holds_for_the_tripbomb_and_the_satchel() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    // 07000005 carries camera 0x54 by loadout; the tripbomb/satchel ride the --give path, so load
    // them the way the main agent's dev give does (load_weapons + panel::apply).
    let mut scene = nf_view::load_weapons(&iso, "07000005.bin", false, &[0x3b, 0x3a]).expect("load 05 with the gadgets");
    let sim = scene.sim.as_mut().unwrap();
    sim.bond = nf_game::player::Bond::new([106.29, 24.87, 110.25], 1.178);
    sim.bond.pitch = -0.166;
    sim.disabled = false;
    // 60 ticks to settle: grounded, standing on the courtyard floor.
    for _ in 0..60 { sim.step_bond(nf_game::player::Input::default(), false, false, false); }
    assert!(sim.bond.grounded, "grounded after 60 ticks (floor_ny {:.3})", sim.bond.floor_ny);
    // A wall hit within 2 m ahead: the game's look direction is (sin yaw, ·, cos yaw) — 0 faces +Z
    // [M player.rs eye dir].
    let dir = [sim.bond.yaw.sin(), 0.0, sim.bond.yaw.cos()];
    let far = [sim.bond.pos[0] + dir[0] * 2.0, sim.bond.pos[1], sim.bond.pos[2] + dir[2] * 2.0];
    let hit = sim.world.segment_hit(sim.bond.pos, far);
    let Some((t, n)) = hit else { panic!("no wall within 2 m at yaw {:.3} — the spot needs one", sim.bond.yaw) };
    assert!(n[1].abs() < 0.7, "the hit must be a WALL (vertical-ish face), normal {:?}", n);
    eprintln!("wall spot: hit at {:.2} m ahead, normal {:?}", t * 2.0, n);
}
