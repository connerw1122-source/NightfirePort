//! fix-final-blockers probe 3c: world-space seal scan — every resolved instance's collision
//! triangles transformed by the placement matrix, tested against the doorway seal box
//! (x 2..6, y −0.1..2.7, z −74.13±0.15). Diagnostic only — run with --nocapture.

fn iso() -> String { std::env::var("NF_ISO").expect("NF_ISO") }

#[test]
fn scan_world_space_seal() {
    use nf_formats::{disc, iso9660::IsoImage, level::Level};
    let iso_path = iso();
    let iso = IsoImage::open(&iso_path).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("07000016.bin")).unwrap();
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = Level::parse(&data).unwrap();
    let m = level.main_map().unwrap();
    eprintln!("=== instances (cels_only) whose WORLD triangles enter the seal box ===");
    for (i, p) in level.maps[m].placements.iter().enumerate() {
        if !p.creates_cel() { continue; }
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else if (p.index as usize) < level.maps[m].objects.len() { Some((m, p.index as usize)) } else { None };
        let Some((mm, oo)) = target else { continue };
        let Some(c) = level.maps[mm].objects[oo].collision.as_ref() else { continue };
        let mat = nf_formats::placement::object_matrix(p.rotation, p.translation);
        let tf = |p: [f32; 3]| {
            [
                mat[0][0] * p[0] + mat[1][0] * p[1] + mat[2][0] * p[2] + mat[3][0],
                mat[0][1] * p[0] + mat[1][1] * p[1] + mat[2][1] * p[2] + mat[3][1],
                mat[0][2] * p[0] + mat[1][2] * p[1] + mat[2][2] * p[2] + mat[3][2],
            ]
        };
        let mut hits = 0;
        let mut xlo = f32::MAX; let mut xhi = f32::MIN; let mut ylo = f32::MAX; let mut yhi = f32::MIN;
        for t in c.triangles() {
            for v in t.iter() {
                let w = tf(*v);
                if (w[2] + 74.13).abs() < 0.2 && w[1] > -0.15 && w[1] < 2.75 && w[0] > 1.5 && w[0] < 6.5 {
                    hits += 1;
                    xlo = xlo.min(w[0]); xhi = xhi.max(w[0]); ylo = ylo.min(w[1]); yhi = yhi.max(w[1]);
                }
            }
        }
        if hits > 0 {
            eprintln!(
                "plc {i} pos ({:.2},{:.2},{:.2}) object ({mm},{oo}) {:?} hits {hits} world x {xlo:.2}..{xhi:.2} y {ylo:.2}..{yhi:.2}",
                p.translation[0], p.translation[1], p.translation[2], level.maps[mm].objects[oo].name
            );
        }
    }
    eprintln!("=== the door object (63,160)'s collision, CLOSED pose world bbox ===");
    // The closed matrix = placement 183's matrix (the door's create pose).
    for (i, p) in level.maps[m].placements.iter().enumerate() {
        if i != 183 { continue; }
        let target = if p.by_hash() { level.by_hash.get(&p.hashcode).copied() } else { Some((m, p.index as usize)) };
        let Some((mm, oo)) = target else { continue };
        let Some(c) = level.maps[mm].objects[oo].collision.as_ref() else { continue; };
        let mat = nf_formats::placement::object_matrix(p.rotation, p.translation);
        let tf = |p: [f32; 3]| {
            [
                mat[0][0] * p[0] + mat[1][0] * p[1] + mat[2][0] * p[2] + mat[3][0],
                mat[0][1] * p[0] + mat[1][1] * p[1] + mat[2][1] * p[2] + mat[3][1],
                mat[0][2] * p[0] + mat[1][2] * p[1] + mat[2][2] * p[2] + mat[3][2],
            ]
        };
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for t in c.triangles() { for v in t.iter() {
            let w = tf(*v);
            for k in 0..3 { lo[k] = lo[k].min(w[k]); hi[k] = hi[k].max(w[k]); }
        } }
        eprintln!("plc 183 object ({mm},{oo}) {:?} closed-leaf world bbox lo {lo:?} hi {hi:?} ({} tris)", level.maps[mm].objects[oo].name, c.triangles().len());
    }
}
