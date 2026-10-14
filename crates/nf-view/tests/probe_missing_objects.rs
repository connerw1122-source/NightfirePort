//! fix-missing-objects probe: the 07000006 fountain / guard-room furniture / lockers + fusebox
//! and the 0700000c pylon. Dumps every main-map placement near the user's spots with the port's
//! draw-path classification (the same predicates scene.rs `build` applies), plus a per-level census
//! of placements whose mesh nothing draws. Opt-in via NF_ISO.

use nf_formats::level::Level;
use nf_formats::levelbin::LevelBin;
use nf_formats::iso9660::IsoImage;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

/// scene.rs `build`'s MISC_KINDS sweep (cels_only path).
const MISC_KINDS: [u8; 15] = [0x2e, 0x30, 0x34, 0x35, 0x3b, 0x71, 0xd5, 0xd8, 0xe1, 0xe3, 0xda, 0xfa, 0xfd, 0xfe, 0x4a];

fn has_mesh(level: &Level, map: usize, object: usize) -> bool {
    level.maps[map].objects[object].gfx.leaves.iter().any(|l| !l.batches.is_empty())
}

/// The port's draw-path classification for one main-map placement (scene.rs build, cels_only=true).
/// Returns (path, note).
fn classify(level: &Level, mm: usize, p: &nf_formats::placement::Placement) -> (&'static str, String) {
    let kind = p.flags & 0xff;
    let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
        else if (p.index as usize) < level.maps[mm].objects.len() { Some((mm, p.index as usize)) } else { None };
    let Some((map, object)) = target else {
        if p.by_hash() { return ("UNRESOLVED", format!("hash {:08x} not in by_hash", p.hashcode)); }
        return ("UNRESOLVED", format!("index {} out of range", p.index));
    };
    let obj = &level.maps[map].objects[object];
    let name = obj.name.trim().to_string();
    let mesh = has_mesh(level, map, object);
    let flags = obj.flags;
    let cel = p.creates_cel();
    // The static loop's retained-out kinds (drawn by their own passes):
    if matches!(kind, 0x2a | 0xd4) {
        let param = |k: u32| p.params.iter().find(|(i, _)| *i == k).map(|&(_, v)| v).unwrap_or(0);
        if kind == 0x2a && param(1) != 0 { return ("SKY-SKIP", format!("lightning variant \"{name}\"")); }
        return ("SKY", format!("\"{name}\""));
    }
    if kind == 0x22 { return ("LADDER", format!("\"{name}\" mesh {mesh}")); }
    if kind == 0x3e { return ("WIRE", format!("\"{name}\" mesh {mesh}")); }
    if kind == 0xfc { return ("CORONA", format!("\"{name}\" mesh {mesh}")); }
    if kind == 0xdb { return ("DOOR", format!("\"{name}\" mesh {mesh}")); }
    if kind == 0xde { return ("SEARCHLIGHT", format!("\"{name}\" mesh {mesh}")); }
    if cel && flags & 0x10 != 0 {
        return ("CEL-SKIP-0x10", format!("\"{name}\" glist {flags:#x} (the game's own View_AddCels skip)"));
    }
    if matches!(kind, 0x20 | 0xe2) && !cel {
        if kind == 0xe2 && p.extra[7] & 0xffff != 0 { return ("SS-ANIM", format!("\"{name}\" animated SS (key count {})", p.extra[7] & 0xffff)); }
        return ("BREAK/SS", format!("\"{name}\" mesh {mesh}"));
    }
    if kind == 0x3a {
        if !mesh { return ("GRAPPLE-NOMESH", format!("\"{name}\"")); }
        if flags & 0x400 != 0 { return ("MIRROR-ONLY", format!("\"{name}\"")); }
        return ("GRAPPLE", format!("\"{name}\""));
    }
    if MISC_KINDS.contains(&(kind as u8)) {
        if kind == 0x3b && p.params.iter().find(|(i, _)| *i == 0).is_some_and(|(_, v)| *v != 0) {
            return ("GATED-3B", format!("\"{name}\" hidden until its channel"));
        }
        if !mesh { return ("MISC-NOMESH", format!("\"{name}\" (obj flags {flags:#x})")); }
        if flags & 0x400 != 0 { return ("MIRROR-ONLY", format!("\"{name}\"")); }
        return ("MISC", format!("\"{name}\""));
    }
    if kind == 0xf0 { return ("PICKUP", format!("\"{name}\" mesh {mesh}")); }
    if kind == 0x0f {
        let skin = p.params.iter().find(|(i, _)| *i == 0).map(|&(_, v)| v).unwrap_or(0);
        if skin >> 24 == 5 && level.skins.contains_key(&skin) { return ("CHARACTER", format!("\"{name}\" skin {skin:08x}")); }
        return ("CHAR-NOSKIN", format!("\"{name}\" skin {skin:08x}"));
    }
    if cel {
        if !mesh { return ("STATIC-NOMESH", format!("\"{name}\" obj flags {flags:#x}")); }
        if flags & 0x400 != 0 { return ("MIRROR-ONLY", format!("\"{name}\"")); }
        return ("STATIC", format!("\"{name}\""));
    }
    // Non-cel, kind with no draw path:
    ("NO-DRAW-PATH", format!("kind {kind:#04x} \"{name}\" mesh {mesh} obj flags {flags:#x}"))
}

#[test]
fn probe_spots_and_census() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let spots: &[(&str, &str, [f32; 3], &str)] = &[
        ("07000006.bin", "fountain", [21.12, 39.04, -0.48], ""),
        ("07000006.bin", "guard room", [33.50, 41.13, -17.90], ""),
        ("07000006.bin", "lockers/fusebox", [31.94, 41.13, -17.36], ""),
        ("0700000c.bin", "pylon", [14.22, 16.28, -139.54], ""),
    ];
    let mut census: std::collections::BTreeMap<String, (String, usize)> = Default::default();
    for (archive, label, spot, _) in spots {
        let data = read_archive(&iso, archive);
        let level = Level::parse(&data).expect("parse");
        let mm = level.main_map().unwrap();
        eprintln!("\n=== {archive} {label} (spot {spot:?}) — placements within 25 units ===");
        let mut near: Vec<(f32, usize, String, String)> = Vec::new();
        for (pi, p) in level.maps[mm].placements.iter().enumerate() {
            let q = p.translation;
            let d = ((q[0] - spot[0]).powi(2) + (q[1] - spot[1]).powi(2) + (q[2] - spot[2]).powi(2)).sqrt();
            let (path, note) = classify(&level, mm, p);
            let e = census.entry(path.to_string()).or_insert_with(|| (String::new(), 0));
            if e.0.is_empty() { e.0 = archive[..8].to_string(); }
            e.1 += 1;
            if d <= 25.0 {
                near.push((d, pi, path.to_string(), note));
            }
        }
        near.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for (d, pi, path, note) in near {
            eprintln!("  #{pi:<4} d={d:6.2} {path:<14} {note}");
        }
    }
    eprintln!("\n=== census (both levels, per path) ===");
    for (path, (levels, n)) in &census {
        eprintln!("  {path:<14} {n:4}  ({levels})");
    }
}
