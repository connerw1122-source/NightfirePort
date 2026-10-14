//! Census (task 289 fix-script-objects): every script Entity command (op 7,
//! `Script_EntityStart` 0x1d8d78 → `Script_CreateEntity` 0x1d8968) on the disc — which object
//! kinds the scripts create, in which level archives, with which glist hashes and params. The
//! port priority rides on this: kinds are ported in order of how many levels use them.
//!
//! Kind map [M 0x1d8968 disasm: the byte → class dispatch]:
//!   0x20 → Break_Create, 0x22 → 0x2d Ladder_Create, 0x29 → Switch_Create,
//!   0xe1 → Destroy_Create, 0xea → Trigger_TouchOnce, 0xeb → Trigger_Touch,
//!   0xf0 → Pickup_Create, 0xf2 → Emitter_CreatePlist, 0xf5 → 0x3e Drone_AIVolume_Create,
//!   anything else → control_create_object + gfx (the generic glist object — Corona_2).
//!
//! Opt-in via NF_ISO. Output is a scratch print (not committed).

use std::collections::BTreeMap;

fn kind_name(t: u8) -> String {
    match t {
        0x20 => "break",
        0x22 => "ladder",
        0x29 => "switch",
        0xe1 => "destroy",
        0xea => "trigger_once",
        0xeb => "trigger",
        0xf0 => "pickup",
        0xf2 => "emitter",
        0xf5 => "ai_volume",
        0x0 => "generic",
        _ => "other",
    }
    .to_string()
}

fn main() {
    let Some(iso_path) = std::env::var("NF_ISO").ok() else { eprintln!("NF_ISO not set"); return };
    let iso = nf_formats::iso9660::IsoImage::open(&iso_path).expect("open iso");
    // The level archives live inside FILES.BIN [M nf_view::load_impl: disc::files_bin_entries].
    let (entries, bin) = nf_formats::disc::files_bin_entries(&iso).expect("FILES.BIN");
    // archive name → (script hash, stream index, obj_type, glist hash, params)
    let mut per_archive: BTreeMap<String, Vec<(u32, usize, u8, u32, Vec<u32>)>> = BTreeMap::new();
    for e in entries.iter() {
        let name = e.name.clone();
        let lower = name.to_ascii_lowercase();
        if !(lower.len() == 12 && lower.ends_with(".bin") && lower[..8].bytes().all(|b| b.is_ascii_hexdigit())) { continue; }
        let Ok(data) = iso.read_bytes(bin.lba as u64 * 2048 + e.offset as u64, e.size as u64) else { continue };
        let Ok(lb) = nf_formats::levelbin::LevelBin::parse(&data) else { continue };
        for se in lb.entries.iter().filter(|e| e.kind == 7) {
            let h = se.hash();
            let Ok(nis) = nf_formats::nis::Nis::parse(h, lb.data(&data, se)) else { continue };
            for (si, s) in nis.streams.iter().enumerate() {
                for cmd in s.decode() {
                    if let nf_formats::nis::Cmd::Entity { hash, obj_type, params, .. } = cmd {
                        per_archive.entry(name.clone()).or_default()
                            .push((h, si, obj_type, hash, params));
                    }
                }
            }
        }
    }
    // level = the archive's level id (pack nibble at bits 20-23 masked off [M LoadableLoad 0x1d69e8]).
    let level_of = |a: &str| format!("{:08x}", u32::from_str_radix(&a[..8], 16).unwrap_or(0) & 0xff0f_ffff);
    let mut by_kind: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut scripts_per_kind: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    let mut total = 0usize;
    for (a, cmds) in &per_archive {
        let lvl = level_of(a);
        for (sh, si, t, gh, p) in cmds {
            let k = kind_name(*t);
            total += 1;
            *by_kind.entry(k.clone()).or_default().entry(lvl.clone()).or_insert(0) += 1;
            scripts_per_kind.entry(k.clone()).or_default().push(*sh);
            println!("{a} lvl {lvl} {:>12} script {sh:08x} stream {si} obj_type {t:02x} glist {gh:08x} params {:?}", k, p);
        }
    }
    println!("\n=== per kind: levels using it, command count ===");
    for (k, levels) in &by_kind {
        let mut scripts = scripts_per_kind[k].clone();
        scripts.sort();
        scripts.dedup();
        println!("{k:>12}: {} levels, {} commands, {} distinct scripts — {:?}",
            levels.len(), scripts_per_kind[k].len(), scripts.len(), levels.keys().cloned().collect::<Vec<_>>());
    }
    println!("total entity commands: {total}");
    println!("\n=== per archive ===");
    for (a, cmds) in &per_archive {
        let mut m: BTreeMap<String, usize> = BTreeMap::new();
        for (_, _, t, _, _) in cmds { *m.entry(kind_name(*t)).or_default() += 1; }
        println!("{a}: {:?}", m);
    }
}
