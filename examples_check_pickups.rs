//! Check 0xF0 placements on a level.
use nf_formats::iso9660::IsoImage;
use nf_formats::{disc, level::Level};
use nf_formats::weapon::WeaponData;
use std::env;

fn load_level(iso: &IsoImage, name: &str) -> Option<Level> {
    let (entries, bin) = disc::files_bin_entries(iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(name))?;
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    Some(Level::parse(&data).unwrap())
}

fn weapon_data(iso: &IsoImage) -> Option<WeaponData> {
    let elf = iso.read_file(iso.find("/ACTION.ELF").unwrap()).unwrap();
    let mut wd = WeaponData::from_action_elf(&elf).unwrap();
    // Apply the per-level Player_Init override (mirror main.rs lines around 127).
    wd.apply_player_init_override(0x0700_0001);
    Some(wd)
}

fn main() {
    let path = env::var_os("NF_ISO").expect("NF_ISO");
    let iso = IsoImage::open(&path).unwrap();
    let wd = weapon_data(&iso).unwrap();

    for name in &["07000001.bin", "07000005.bin"] {
        let level = load_level(&iso, name).unwrap();
        let m = level.main_map().unwrap();
        let ps = &level.maps[m].placements;
        let f0: Vec<&nf_formats::placement::Placement> = ps.iter().filter(|p| p.flags & 0xff == 0xf0).collect();
        println!("=== {} ===", name);
        println!("main map: {}, total placements: {}", m, ps.len());
        println!("0xF0 placements: {}", f0.len());

        let pickup_models = nf_game::pickup::models(&level);
        for (i, p) in f0.iter().enumerate() {
            let type_ = p.extra[2] as u16;
            let id = p.extra[3] as u16;
            let switch_sound = p.extra[4];
            let switch_channel = (switch_sound & 0xFFFF) as u16;
            let sound = (switch_sound >> 16) as u16;
            let respawn_lifetime = p.extra[5];
            let respawn = respawn_lifetime & 0xFFFF;
            let lifetime = ((respawn_lifetime >> 16) & 0xFF) as u8;
            let (model, bounds) = if p.by_hash() {
                let h = p.hashcode;
                level.by_hash.get(&h).and_then(|&(om, oi)| {
                    let obj = &level.maps[om].objects[oi];
                    pickup_models.get(&obj.hashcode).map(|&m| (obj.hashcode, m))
                })
            } else if (p.index as usize) < level.maps[m].objects.len() {
                let obj = &level.maps[m].objects[p.index as usize];
                pickup_models.get(&obj.hashcode).map(|&bm| (obj.hashcode, bm))
            } else { None };
            let model_str = model.map(|mh| format!("{:08x}", mh)).unwrap_or_else(|| "none".to_string());
            // What weapon does this id correspond to? Use weapon_data records.
            let weapon_name = if (id as usize) < wd.records.len() {
                let r = &wd.records[id as usize];
                let fp = u32::from_le_bytes(r[0xdc..0xe0].try_into().unwrap());
                let selectable = r[4] == 1;
                let ammo_type = r[0x90];
                let clip = i16::from_le_bytes([r[0x92], r[0x93]]);
                format!("fp={:08x} selectable={} ammo_type={} clip={}", fp, selectable, ammo_type, clip)
            } else {
                "unknown".to_string()
            };
            let can_create = nf_game::pickup::Pickup::create_placed(
                p.translation, p.rotation, type_, id, switch_channel, sound, respawn, lifetime,
                model.unwrap_or(0), bounds.unwrap_or(nf_game::pickup::Model { centre: [0.0; 3], radius: 0.5 }),
                &wd,
            ).is_some();
            println!("  [{}] type={} id={:#x} ({}) at {:?} model={} switch_ch={} sound={} respawn={} lifetime={} => {}",
                i, type_, id, weapon_name, p.translation, model_str, switch_channel, sound, respawn, lifetime,
                if can_create { "OK" } else { "REFUSED" });
        }
    }
}
