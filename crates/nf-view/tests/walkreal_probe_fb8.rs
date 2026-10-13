//! fix-final-blockers2 probe 3: the natural positions of the cut-scene's glist objects in
//! 0700004a — decides whether the NIS entity keys are base-relative (the object's home ≈
//! key + base) or world-space (home ≈ key). Diagnostic only.

use nf_formats::{disc, iso9660::IsoImage, level::Level};

#[test]
fn dump_cutscene_object_homes() {
    let iso = IsoImage::open(std::env::var("NF_ISO").unwrap().as_str()).unwrap();
    let (entries, bin) = disc::files_bin_entries(&iso).unwrap();
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case("0700004a.bin")).unwrap();
    let data = iso.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).unwrap();
    let level = Level::parse(&data).unwrap();
    for hash in [0x02000758u32, 0x020006e1, 0x020006b1, 0x020006ee, 0x020005b4] {
        let mut hits = String::new();
        for (mi, m) in level.maps.iter().enumerate() {
            for (oi, o) in m.objects.iter().enumerate() {
                if o.hashcode == hash || (hash == 0x02000758 && (o.hashcode & 0xffff) == 0x0758) {
                    hits.push_str(&format!(" map{mi}#{oi} centre ({:.2},{:.2},{:.2}) r {:.2}", o.centre[0], o.centre[1], o.centre[2], o.radius));
                }
            }
        }
        eprintln!("{hash:08x}:{hits}");
    }
    for (pi, p) in level.maps[level.main_map().unwrap()].placements.iter().enumerate() {
        if p.flags & 0xff == 0xd9 {
            eprintln!("SP placement {pi}: flags {:08x} trans {:?} rot {:?} params {:?}", p.flags, p.translation, p.rotation, p.params);
        }
    }
    eprintln!("key-homes (base-composed): 758 (3.43,{:.2},-4.21) 6e1 (3.05,{:.2},-6.46) 6b1 (2.03,{:.2},-6.41) ee (0.00,{:.2},-3.38) | raw: 758 y+1.49 6e1 y-7.99 6b1 y-6.75 ee y+0.99",
        1.488 - 53.53, -7.99 - 53.53, -6.75 - 53.53, 0.99 - 53.53);
}
