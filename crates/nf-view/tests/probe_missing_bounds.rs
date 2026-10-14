//! fix-missing-objects probe 3: the named objects' geometry — min/max/centre, gfx leaf/batch
//! counts, object entity params — plus whether the placement is by-hash or by-index. Opt-in NF_ISO.

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

const NAMES: &[&str] = &["OBJECT_Fountain", "OBJECT_GuardroomTable", "OBJECT_Locker", "OBJECT_WallClock",
    "Object_bed", "Object_chair", "Object_Mapboard", "OBJECT_FuseBox", "pylon", "pylon_back", "pylon_left",
    "pylon_right", "pylonshadow", "turbine-crates2", "all-crates-lit", "Object_Crates_Seat", "OBJECT_LampPost1",
    "OBJECT_WoodenBarrel2", "Object_Pondwater", "Object_box", "blocker_sk"];

#[test]
fn probe_object_bounds() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    for archive in ["07000006.bin", "0700000c.bin"] {
        let data = read_archive(&iso, archive);
        let level = Level::parse(&data).expect("parse");
        let mm = level.main_map().unwrap();
        eprintln!("\n=== {archive}: named objects (map {mm}, {} objects) ===", level.maps[mm].objects.len());
        for (oi, o) in level.maps[mm].objects.iter().enumerate() {
            let name = o.name.trim();
            if !NAMES.iter().any(|k| name.eq_ignore_ascii_case(k)) { continue; }
            let n_leaves = o.gfx.leaves.len();
            let n_batches: usize = o.gfx.leaves.iter().map(|l| l.batches.len()).sum();
            let mut verts = 0usize; let mut tris = 0usize;
            for l in &o.gfx.leaves { for b in &l.batches { verts += b.verts.len(); tris += b.verts.len().saturating_sub(2); } }
            eprintln!("  [{oi}] \"{name}\" min {:?} max {:?} centre {:?} radius {:.2}",
                o.min, o.max, o.centre, o.radius);
            eprintln!("        leaves {n_leaves} batches {n_batches} verts {verts} tris {tris} objflags {:#x} collision {}",
                o.flags, o.collision.is_some());
        }
        // The placements of the named objects: by-hash or by-index?
        eprintln!("--- placements of named objects ---");
        for (pi, p) in level.maps[mm].placements.iter().enumerate() {
            let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() }
                else if (p.index as usize) < level.maps[mm].objects.len() { Some((mm, p.index as usize)) } else { None };
            let Some((map, object)) = target else { continue };
            let name = level.maps[map].objects[object].name.trim();
            if !NAMES.iter().any(|k| name.eq_ignore_ascii_case(k)) { continue; }
            eprintln!("  #{pi:<4} \"{}\" by_hash {} hash {:08x} index {} kind {:#04x} rot {:?} trans {:?} params {:?} extra[7] {:#x}",
                name, p.by_hash(), p.hashcode, p.index, p.flags & 0xff, p.rotation, p.translation,
                p.params.iter().take(4).collect::<Vec<_>>(), p.extra[7]);
        }
    }
}
