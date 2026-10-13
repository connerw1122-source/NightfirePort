//! walkreal-a probe: 07000002's dock geometry — what blocks (−23.4,34.2)→(−17,34.5)? Diagnostic.

use nf_formats::iso9660::IsoImage;
use nf_formats::placement::transform;

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn probe_02_dock() {
    let iso = iso();
    let img = IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000002.bin")).expect("archive");
    let data = img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read");
    let level = nf_formats::level::Level::parse(&data).expect("parse");
    let (lo, hi) = ([-28.0f32, -0.6, 27.5], [-13.5, 3.6, 37.0]);
    let mut seen = std::collections::HashSet::new();
    for inst in level.instances(true).0 {
        let Some(c) = &level.maps[inst.map].objects[inst.object].collision else { continue };
        for t in c.triangles() {
            let tw = t.map(|p| transform(&inst.matrix, p));
            let tlo = [0, 1, 2].map(|k| tw.iter().map(|v| v[k]).fold(f32::INFINITY, f32::min));
            let thi = [0, 1, 2].map(|k| tw.iter().map(|v| v[k]).fold(f32::NEG_INFINITY, f32::max));
            if (0..3).any(|k| thi[k] < lo[k] || tlo[k] > hi[k]) { continue; }
            let key = (inst.map, inst.object);
            if seen.insert(key) {
                eprintln!("obj map{} obj{} name={:?} flags={:#x}", inst.map, inst.object,
                    level.maps[inst.map].objects[inst.object].name.trim(), level.maps[inst.map].objects[inst.object].flags);
            }
        }
    }
    // And the dock's walkable probes at y 1.1: which cells are free?
    let img2 = IsoImage::open(&iso).expect("open iso");
    let (entries2, bin2) = nf_formats::disc::files_bin_entries(&img2).expect("FILES.BIN");
    let fe2 = entries2.iter().find(|e| e.name.eq_ignore_ascii_case("07000002.bin")).expect("archive");
    let data2 = img2.read_bytes(bin2.lba as u64 * 2048 + fe2.offset as u64, fe2.size as u64).expect("read");
    let level2 = nf_formats::level::Level::parse(&data2).expect("parse");
    let world = nf_game::world::CollisionWorld::build(&level2, &level2.instances(true).0);
    let mut z = 25.5f32;
    while z <= 38.0 {
        let mut row = String::new();
        let mut x = -28.0f32;
        while x <= -13.0 {
            let (a, b) = ([x, 1.7, z], [x, 0.6, z]);
            let (push, _) = world.capsule_push(a, b, 0.55);
            let blocked = push.iter().any(|v| v.abs() > 0.05);
            row.push(if blocked { '#' } else { '.' });
            x += 1.0;
        }
        eprintln!("z={z:5.1}: {row}   (x -28..-13)");
        z += 1.0;
    }
}
