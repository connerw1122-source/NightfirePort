//! fix-missing-objects probe 7: the census's unchained generics — for each, the object name,
//! glist flags, whether the 0x8000 forced-draw bit covers it (View_AddForcedObjects 0x1e5fb8
//! draws glist-flag-0x8000 objects from the global list — fix-portal5 §1). Opt-in NF_ISO.

use nf_formats::level::Level;
use nf_formats::levelbin::LevelBin;
use nf_formats::iso9660::IsoImage;
use nf_view::scene;

fn iso() -> Option<String> { std::env::var("NF_ISO").ok() }

fn read_archive(iso: &str, archive: &str) -> Vec<u8> {
    let img = IsoImage::open(iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let fe = entries.iter().find(|e| e.name.eq_ignore_ascii_case(archive)).expect("archive");
    img.read_bytes(bin.lba as u64 * 2048 + fe.offset as u64, fe.size as u64).expect("read")
}

#[test]
fn probe_unchained_detail() {
    let Some(iso) = iso() else { eprintln!("NF_ISO not set — skipping"); return };
    let img = IsoImage::open(&iso).expect("open iso");
    let (entries, bin) = nf_formats::disc::files_bin_entries(&img).expect("FILES.BIN");
    let mut archives: Vec<String> = entries.iter().filter_map(|e| {
        let n = e.name.to_ascii_uppercase();
        n.starts_with("0700").then(|| e.name.clone())
    }).collect();
    archives.sort();
    for a in &archives {
        let data = read_archive(&iso, a);
        let Ok(level) = Level::parse(&data) else { continue };
        if level.main_map().is_none() { continue }
        let (_rooms, mut cel_graph) = scene::parse_cel_graph(&level);
        let instances = level.instances(true).0;
        let mut world = nf_game::world::CollisionWorld::build_scoped(&level, &instances, Some(scene::scope_graph(&cel_graph)));
        scene::build_chains(&mut cel_graph, Some(&world));
        let main = level.main_map().unwrap();
        let mut chain_owner = vec![None; cel_graph.cells.len()];
        for (o, ch) in cel_graph.chains.iter().enumerate() {
            for &n in ch { if (n as usize) < chain_owner.len() { chain_owner[n as usize] = Some(o); } }
        }
        let mut ordinal = 0usize;
        for (mi, map) in level.maps.iter().enumerate() {
            for p in &map.placements {
                if !p.creates_cel() { continue; }
                if Some(mi) == Some(main) {
                    let c = ordinal;
                    let f16 = cel_graph.cells[c].flags & 0xffff;
                    if matches!(f16, 0x8018 | 0x8019) {
                        let zero = cel_graph.cells[c].min == [0.0; 3] && cel_graph.cells[c].max == [0.0; 3] && cel_graph.cells[c].sphere.1 == 0.0;
                        if !zero && chain_owner[c].is_none() {
                            let (name, objf, mesh) = if p.by_hash() {
                                level.by_hash.get(&p.hashcode).map(|&(m, o)| (level.maps[m].objects[o].name.trim().to_string(), level.maps[m].objects[o].flags, level.maps[m].objects[o].gfx.leaves.iter().any(|l| !l.batches.is_empty()))).unwrap_or_default()
                            } else { map.objects.get(p.index as usize).map(|o| (o.name.trim().to_string(), o.flags, o.gfx.leaves.iter().any(|l| !l.batches.is_empty()))).unwrap_or_default() };
                            eprintln!("[unchained] {a} cell {c} \"{name}\" objf {objf:#x} mesh {mesh} at {:?}", p.translation);
                        }
                    }
                }
                ordinal += 1;
            }
        }
    }
}
