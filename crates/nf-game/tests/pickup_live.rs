//! Level-placed pickups (type 0xF0, `parsemap_create_dynamic_objects` → `Pickup_Create`): the 07000005
//! placements and the pickups that spawn from them. Opt-in: set `NF_ISO` to the disc image.

use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, level::Level};
use nf_formats::weapon::WeaponData;

fn level_05() -> Option<Level> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000005.bin")).unwrap();
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Some(Level::parse(&data).unwrap())
}

fn weapon_data() -> Option<WeaponData> {
    let path = std::env::var_os("NF_ISO")?;
    let iso = IsoImage::open(&path).unwrap();
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    Some(WeaponData::from_action_elf(&elf).unwrap())
}

#[test]
fn level_placed_pickups_spawn_from_0xf0_placements() {
    let (Some(level), Some(wd)) = (level_05(), weapon_data()) else { eprintln!("skip: NF_ISO not set"); return };
    let m = level.main_map().unwrap();
    let ps = &level.maps[m].placements;
    // The 0xF0 placements on 07000005.
    let f0: Vec<&nf_formats::placement::Placement> = ps.iter().filter(|p| p.flags & 0xff == 0xf0).collect();
    assert!(!f0.is_empty(), "07000005 has type-0xF0 pickup placements");
    // Build the pickups the same way Scene::build does.
    let pickup_models = nf_game::pickup::models(&level);
    let pickups: Vec<&nf_formats::placement::Placement> = f0.iter()
        .filter_map(|p| {
            let type_ = p.extra[2] as u16;
            let id = p.extra[3] as u16;
            // The model: the placement's object hashcode.
            let (model, bounds) = if p.by_hash() {
                let h = p.hashcode;
                level.by_hash.get(&h).and_then(|&(om, oi)| {
                    let obj = &level.maps[om].objects[oi];
                    pickup_models.get(&obj.hashcode).map(|&m| (obj.hashcode, m))
                })
            } else if (p.index as usize) < level.maps[m].objects.len() {
                let obj = &level.maps[m].objects[p.index as usize];
                pickup_models.get(&obj.hashcode).map(|&bm| (obj.hashcode, bm))
            } else { None }?;
            nf_game::pickup::Pickup::create_placed(p.translation, p.rotation, type_, id, 0, 0xFFFF, 0, 0, model, bounds, &wd).is_some().then_some(*p)
        })
        .collect();
    assert!(!pickups.is_empty(), "at least some 0xF0 placements spawn pickups");
    // Sister: a placement with an invalid id (0xFFFF) does NOT spawn.
    assert!(nf_game::pickup::Pickup::create_placed([0.0; 3], [0.0; 3], 0, 0xFFFF, 0, 0xFFFF, 0, 0, 0x12345678, nf_game::pickup::Model { centre: [0.0; 3], radius: 0.5 }, &wd).is_none(),
        "a pickup with id 0xFFFF is refused");
}
